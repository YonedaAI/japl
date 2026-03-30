use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use wasmtime::*;

use crate::distribution::DistributionLayer;
use crate::engine::JaplEngine;
use crate::process::{ProcessId, ProcessMessage, ProcessState, SchedulerCommand};

pub struct Scheduler {
    engine: Option<Arc<JaplEngine>>,
    /// Map from PID -> sender for delivering messages to that process.
    processes: Arc<Mutex<HashMap<ProcessId, mpsc::Sender<ProcessMessage>>>>,
    next_pid: Arc<Mutex<ProcessId>>,
    /// Channel the scheduler listens on for commands from processes.
    cmd_tx: mpsc::Sender<SchedulerCommand>,
    cmd_rx: Option<mpsc::Receiver<SchedulerCommand>>,
    /// Optional distribution layer for cross-node communication.
    distribution: Option<Arc<DistributionLayer>>,
}

impl Scheduler {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        Self {
            engine: None,
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(0)),
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            distribution: None,
        }
    }

    /// Get a clone of the command sender (for distribution layer to inject commands).
    pub fn command_sender(&self) -> mpsc::Sender<SchedulerCommand> {
        self.cmd_tx.clone()
    }

    /// Set the distribution layer for cross-node messaging.
    pub fn set_distribution(&mut self, dist: DistributionLayer) {
        self.distribution = Some(Arc::new(dist));
    }

    pub fn load_module(&mut self, path: &str) -> anyhow::Result<()> {
        let engine = JaplEngine::new(path)?;
        self.engine = Some(Arc::new(engine));
        Ok(())
    }

    fn alloc_pid(&self) -> ProcessId {
        let mut pid = self.next_pid.lock().unwrap();
        let id = *pid;
        *pid += 1;
        id
    }

    /// Spawn a process that runs the given exported function.
    fn spawn_process(&self, entry: &str) -> anyhow::Result<ProcessId> {
        let pid = self.alloc_pid();
        let (msg_tx, msg_rx) = mpsc::channel::<ProcessMessage>();
        self.processes.lock().unwrap().insert(pid, msg_tx);

        let engine_arc = self.engine.as_ref().unwrap().clone();
        let cmd_tx = self.cmd_tx.clone();
        let entry = entry.to_string();

        std::thread::spawn(move || {
            let wasi = JaplEngine::build_wasi_ctx();
            let state = ProcessState::new(pid, msg_rx, cmd_tx.clone(), wasi);
            let mut store = Store::new(&engine_arc.engine, state);

            let linker = match engine_arc.build_linker() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[pid {}] linker error: {}", pid, e);
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            let instance = match linker.instantiate(&mut store, &engine_arc.module) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("[pid {}] instantiation error: {}", pid, e);
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            let func = match instance.get_typed_func::<(), ()>(&mut store, &entry) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[pid {}] could not find export '{}': {}", pid, entry, e);
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            if let Err(e) = func.call(&mut store, ()) {
                eprintln!("[pid {}] runtime error: {}", pid, e);
            }

            let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
        });

        Ok(pid)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        // Spawn main process (PID 0)
        let _main_pid = self.spawn_process("_start")?;

        // Process scheduler commands until all processes exit.
        let cmd_rx = self.cmd_rx.take().unwrap();
        let mut alive_count: usize = 1; // main process

        loop {
            if alive_count == 0 {
                break;
            }

            match cmd_rx.recv() {
                Ok(SchedulerCommand::Spawn { func_name, reply }) => {
                    match self.spawn_process(&func_name) {
                        Ok(new_pid) => {
                            alive_count += 1;
                            let _ = reply.send(new_pid);
                        }
                        Err(e) => {
                            eprintln!("spawn error: {}", e);
                        }
                    }
                }
                Ok(SchedulerCommand::Send {
                    target_pid,
                    message,
                }) => {
                    let procs = self.processes.lock().unwrap();
                    if let Some(tx) = procs.get(&target_pid) {
                        let _ = tx.send(ProcessMessage::Deliver(message));
                    } else {
                        eprintln!("send to unknown pid {}", target_pid);
                    }
                }
                Ok(SchedulerCommand::Exited { pid }) => {
                    self.processes.lock().unwrap().remove(&pid);
                    alive_count = alive_count.saturating_sub(1);
                }
                Err(_) => {
                    break;
                }
            }
        }

        Ok(())
    }
}

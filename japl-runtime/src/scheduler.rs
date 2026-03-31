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

        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024) // 64MB stack for deep recursion
            .spawn(move || {
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
        }).expect("failed to spawn process thread");

        Ok(pid)
    }

    /// Spawn a process that calls __process_entry(closure_ptr) with shared memory state.
    fn spawn_closure_process(&self, closure_ptr: i64, closure_bytes: Vec<u8>) -> anyhow::Result<ProcessId> {
        let pid = self.alloc_pid();
        let (msg_tx, msg_rx) = mpsc::channel::<ProcessMessage>();
        self.processes.lock().unwrap().insert(pid, msg_tx);

        let engine_arc = self.engine.as_ref().unwrap().clone();
        let cmd_tx = self.cmd_tx.clone();

        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024) // 64MB stack for deep recursion
            .spawn(move || {
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

            // Copy closure bytes into the child's memory at the same address
            // and advance the heap pointer past them so the child doesn't overwrite
            if !closure_bytes.is_empty() {
                if let Some(mem) = instance.get_memory(&mut store, "memory") {
                    let ptr = closure_ptr as usize;
                    let data = mem.data_mut(&mut store);
                    let end = (ptr + closure_bytes.len()).min(data.len());
                    let copy_len = end - ptr;
                    data[ptr..end].copy_from_slice(&closure_bytes[..copy_len]);
                }
                // Update the heap_ptr global so child allocations don't conflict
                if let Some(heap_ptr) = instance.get_global(&mut store, "heap_ptr") {
                    let new_heap = (closure_ptr as i32) + (closure_bytes.len() as i32);
                    // Align to 8
                    let aligned = (new_heap + 7) & !7;
                    let _ = heap_ptr.set(&mut store, Val::I32(aligned));
                }
            }

            // Call __process_entry(closure_ptr)
            let func = match instance.get_typed_func::<(i64,), ()>(&mut store, "__process_entry") {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[pid {}] could not find export '__process_entry': {}", pid, e);
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            if let Err(e) = func.call(&mut store, (closure_ptr,)) {
                eprintln!("[pid {}] runtime error: {}", pid, e);
            }

            let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
        }).expect("failed to spawn closure process thread");

        Ok(pid)
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        // Spawn main process (PID 0)
        let main_pid = self.spawn_process("_start")?;

        // Process scheduler commands until all processes exit.
        let cmd_rx = self.cmd_rx.take().unwrap();
        let mut alive_count: usize = 1; // main process
        let mut main_exited = false;

        loop {
            if alive_count == 0 || (main_exited && self.distribution.is_none()) {
                if self.distribution.is_some() {
                    // Keep listening for remote commands even after all local
                    // processes have exited (the node is acting as a service).
                    loop {
                        match cmd_rx.recv() {
                            Ok(cmd) => match cmd {
                                SchedulerCommand::Spawn { func_name, reply } => {
                                    match self.spawn_process(&func_name) {
                                        Ok(new_pid) => {
                                            alive_count += 1;
                                            let _ = reply.send(new_pid);
                                        }
                                        Err(e) => eprintln!("spawn error: {}", e),
                                    }
                                }
                                SchedulerCommand::SpawnClosure { closure_ptr, closure_bytes, reply } => {
                                    match self.spawn_closure_process(closure_ptr, closure_bytes) {
                                        Ok(new_pid) => {
                                            alive_count += 1;
                                            let _ = reply.send(new_pid);
                                        }
                                        Err(e) => eprintln!("spawn closure error: {}", e),
                                    }
                                }
                                SchedulerCommand::Send { target_pid, message_bytes } => {
                                    let procs = self.processes.lock().unwrap();
                                    if let Some(tx) = procs.get(&target_pid) {
                                        let _ = tx.send(ProcessMessage::Deliver(message_bytes));
                                    }
                                }
                                SchedulerCommand::Exited { pid } => {
                                    self.processes.lock().unwrap().remove(&pid);
                                    alive_count = alive_count.saturating_sub(1);
                                }
                            },
                            Err(_) => break, // channel closed
                        }
                    }
                } else {
                    // No distribution layer -- force-exit so child threads
                    // stuck in receive loops don't keep the process alive.
                    std::process::exit(0);
                }
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
                Ok(SchedulerCommand::SpawnClosure { closure_ptr, closure_bytes, reply }) => {
                    match self.spawn_closure_process(closure_ptr, closure_bytes) {
                        Ok(new_pid) => {
                            alive_count += 1;
                            let _ = reply.send(new_pid);
                        }
                        Err(e) => {
                            eprintln!("spawn closure error: {}", e);
                        }
                    }
                }
                Ok(SchedulerCommand::Send {
                    target_pid,
                    message_bytes,
                }) => {
                    let procs = self.processes.lock().unwrap();
                    if let Some(tx) = procs.get(&target_pid) {
                        let _ = tx.send(ProcessMessage::Deliver(message_bytes));
                    } else {
                        eprintln!("send to unknown pid {}", target_pid);
                    }
                }
                Ok(SchedulerCommand::Exited { pid }) => {
                    self.processes.lock().unwrap().remove(&pid);
                    alive_count = alive_count.saturating_sub(1);
                    if pid == main_pid {
                        main_exited = true;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }

        Ok(())
    }
}

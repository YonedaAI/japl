use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use wasmtime::*;

use super::engine::JaplEngine;
use super::process::{
    self, ProcessId, ProcessMessage, ProcessState, SchedulerCommand,
    PROCESS_STACK_SIZE, DEFAULT_MAX_MAILBOX_SIZE,
};

/// Graceful shutdown timeout in seconds.
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

pub struct Scheduler {
    engine: Option<Arc<JaplEngine>>,
    processes: Arc<Mutex<HashMap<ProcessId, mpsc::Sender<ProcessMessage>>>>,
    /// Track mailbox sizes at the scheduler level for the MailboxSize query.
    mailbox_sizes: Arc<Mutex<HashMap<ProcessId, Arc<AtomicUsize>>>>,
    next_pid: Arc<Mutex<ProcessId>>,
    cmd_tx: mpsc::Sender<SchedulerCommand>,
    cmd_rx: Option<mpsc::Receiver<SchedulerCommand>>,
    max_mailbox_size: usize,
    /// Shared shutdown flag: 0 = running, 1 = shutting down.
    shutdown_flag: Arc<AtomicUsize>,
}

impl Scheduler {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        Self {
            engine: None,
            processes: Arc::new(Mutex::new(HashMap::new())),
            mailbox_sizes: Arc::new(Mutex::new(HashMap::new())),
            next_pid: Arc::new(Mutex::new(0)),
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            max_mailbox_size: DEFAULT_MAX_MAILBOX_SIZE,
            shutdown_flag: Arc::new(AtomicUsize::new(0)),
        }
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

    fn spawn_process(&self, entry: &str) -> anyhow::Result<ProcessId> {
        let pid = self.alloc_pid();
        let (msg_tx, msg_rx) = mpsc::channel::<ProcessMessage>();
        let mailbox_size = Arc::new(AtomicUsize::new(0));

        {
            let mut procs = self.processes.lock().unwrap();
            procs.insert(pid, msg_tx);
        }
        {
            let mut sizes = self.mailbox_sizes.lock().unwrap();
            sizes.insert(pid, mailbox_size.clone());
        }

        process::increment_process_count();

        let engine_arc = self.engine.as_ref().unwrap().clone();
        let cmd_tx = self.cmd_tx.clone();
        let entry = entry.to_string();
        let shutdown_flag = self.shutdown_flag.clone();
        let mb_counter = mailbox_size;

        std::thread::Builder::new()
            .name(format!("japl-pid-{}", pid))
            .stack_size(PROCESS_STACK_SIZE)
            .spawn(move || {
            let wasi = JaplEngine::build_wasi_ctx();
            let state = ProcessState::new(pid, msg_rx, cmd_tx.clone(), wasi, shutdown_flag, mb_counter);
            let mut store = Store::new(&engine_arc.engine, state);

            let linker = match engine_arc.build_linker() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[pid {}] linker error: {}", pid, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            let instance = match linker.instantiate(&mut store, &engine_arc.module) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("[pid {}] instantiation error: {}", pid, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            let func = match instance.get_typed_func::<(), ()>(&mut store, &entry) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[pid {}] could not find export '{}': {}", pid, entry, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            if let Err(e) = func.call(&mut store, ()) {
                eprintln!("[pid {}] runtime error: {}", pid, e);
            }

            process::decrement_process_count();
            let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
        }).expect("failed to spawn process thread");

        Ok(pid)
    }

    fn spawn_closure_process(&self, closure_ptr: i64, closure_bytes: Vec<u8>) -> anyhow::Result<ProcessId> {
        let pid = self.alloc_pid();
        let (msg_tx, msg_rx) = mpsc::channel::<ProcessMessage>();
        let mailbox_size = Arc::new(AtomicUsize::new(0));

        {
            let mut procs = self.processes.lock().unwrap();
            procs.insert(pid, msg_tx);
        }
        {
            let mut sizes = self.mailbox_sizes.lock().unwrap();
            sizes.insert(pid, mailbox_size.clone());
        }

        process::increment_process_count();

        let engine_arc = self.engine.as_ref().unwrap().clone();
        let cmd_tx = self.cmd_tx.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let mb_counter = mailbox_size;

        std::thread::Builder::new()
            .name(format!("japl-pid-{}", pid))
            .stack_size(PROCESS_STACK_SIZE)
            .spawn(move || {
            let wasi = JaplEngine::build_wasi_ctx();
            let state = ProcessState::new(pid, msg_rx, cmd_tx.clone(), wasi, shutdown_flag, mb_counter);
            let mut store = Store::new(&engine_arc.engine, state);

            let linker = match engine_arc.build_linker() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[pid {}] linker error: {}", pid, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            let instance = match linker.instantiate(&mut store, &engine_arc.module) {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("[pid {}] instantiation error: {}", pid, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            if !closure_bytes.is_empty() {
                if let Some(mem) = instance.get_memory(&mut store, "memory") {
                    let ptr = closure_ptr as usize;
                    let data = mem.data_mut(&mut store);
                    let end = (ptr + closure_bytes.len()).min(data.len());
                    let copy_len = end - ptr;
                    data[ptr..end].copy_from_slice(&closure_bytes[..copy_len]);
                }
                if let Some(heap_ptr) = instance.get_global(&mut store, "heap_ptr") {
                    let new_heap = (closure_ptr as i32) + (closure_bytes.len() as i32);
                    let aligned = (new_heap + 7) & !7;
                    let _ = heap_ptr.set(&mut store, Val::I32(aligned));
                }
            }

            let func = match instance.get_typed_func::<(i64,), ()>(&mut store, "__process_entry") {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[pid {}] could not find '__process_entry': {}", pid, e);
                    process::decrement_process_count();
                    let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
                    return;
                }
            };

            if let Err(e) = func.call(&mut store, (closure_ptr,)) {
                eprintln!("[pid {}] runtime error: {}", pid, e);
            }

            process::decrement_process_count();
            let _ = cmd_tx.send(SchedulerCommand::Exited { pid });
        }).expect("failed to spawn closure process thread");

        Ok(pid)
    }

    fn send_to_process(&self, target_pid: ProcessId, message_bytes: Vec<u8>) -> bool {
        let procs = self.processes.lock().unwrap();
        if let Some(tx) = procs.get(&target_pid) {
            // Check mailbox size limit
            if let Some(size) = self.mailbox_sizes.lock().unwrap().get(&target_pid) {
                let current = size.load(Ordering::Relaxed);
                if current >= self.max_mailbox_size {
                    eprintln!(
                        "mailbox full for pid {} ({}/{}), dropping message",
                        target_pid, current, self.max_mailbox_size
                    );
                    return false;
                }
                size.fetch_add(1, Ordering::Relaxed);
            }
            let _ = tx.send(ProcessMessage::Deliver(message_bytes));
            true
        } else {
            eprintln!("send to unknown pid {}", target_pid);
            false
        }
    }

    fn get_mailbox_size(&self, target_pid: ProcessId) -> usize {
        if let Some(size) = self.mailbox_sizes.lock().unwrap().get(&target_pid) {
            size.load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Send shutdown signal to all remaining processes and wait up to timeout.
    fn graceful_shutdown(&self) {
        let procs = self.processes.lock().unwrap();
        let count = procs.len();
        if count == 0 {
            return;
        }
        eprintln!(
            "[scheduler] graceful shutdown: signaling {} remaining process(es)",
            count
        );

        // Set the global shutdown flag
        self.shutdown_flag.store(1, Ordering::SeqCst);

        // Send Shutdown message to all processes
        for (pid, tx) in procs.iter() {
            if let Err(_) = tx.send(ProcessMessage::Shutdown) {
                eprintln!("[scheduler] could not signal pid {}", pid);
            }
        }
        drop(procs);

        // Wait for processes to exit, with timeout.
        // Drain the command channel so Exited signals are processed.
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_secs(SHUTDOWN_TIMEOUT_SECS);

        loop {
            let remaining = self.processes.lock().unwrap().len();
            if remaining == 0 {
                eprintln!("[scheduler] all processes exited cleanly");
                break;
            }
            if std::time::Instant::now() >= deadline {
                eprintln!(
                    "[scheduler] shutdown timeout reached, {} process(es) still alive — exiting",
                    remaining
                );
                break;
            }
            // Drain pending commands (especially Exited) to update process map
            while let Ok(cmd) = self.cmd_rx.as_ref().map_or(
                Err(std::sync::mpsc::TryRecvError::Disconnected),
                |rx| rx.try_recv()
            ) {
                self.handle_cmd_during_shutdown(cmd);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    /// Handle a scheduler command during graceful shutdown.
    /// Only processes Exited commands; other commands are dropped.
    fn handle_cmd_during_shutdown(&self, cmd: SchedulerCommand) {
        if let SchedulerCommand::Exited { pid } = cmd {
            self.processes.lock().unwrap().remove(&pid);
            self.mailbox_sizes.lock().unwrap().remove(&pid);
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let main_pid = self.spawn_process("_start")?;
        let cmd_rx = self.cmd_rx.take().unwrap();
        let mut alive_count: usize = 1;
        let mut main_exited = false;

        loop {
            if alive_count == 0 || main_exited {
                // Graceful shutdown: wait for remaining processes
                if alive_count > 0 {
                    self.graceful_shutdown();
                }
                eprintln!(
                    "[scheduler] shutdown complete. Final process count: {}",
                    process::active_process_count()
                );
                return Ok(());
            }

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
                    SchedulerCommand::Send { target_pid, message_bytes, reply } => {
                        let ok = self.send_to_process(target_pid, message_bytes);
                        if let Some(reply) = reply {
                            let _ = reply.send(ok);
                        }
                    }
                    SchedulerCommand::MailboxSize { target_pid, reply } => {
                        let size = self.get_mailbox_size(target_pid);
                        let _ = reply.send(size);
                    }
                    SchedulerCommand::Exited { pid } => {
                        self.processes.lock().unwrap().remove(&pid);
                        self.mailbox_sizes.lock().unwrap().remove(&pid);
                        alive_count = alive_count.saturating_sub(1);
                        if pid == main_pid {
                            main_exited = true;
                        }
                    }
                },
                Err(_) => break,
            }
        }

        Ok(())
    }
}

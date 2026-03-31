use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub type ProcessId = u64;

/// Default maximum mailbox size per process.
pub const DEFAULT_MAX_MAILBOX_SIZE: usize = 10_000;

/// Thread stack size for process threads (2 MB).
pub const PROCESS_STACK_SIZE: usize = 2 * 1024 * 1024;

/// Global counter of active processes.
static ACTIVE_PROCESS_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn active_process_count() -> usize {
    ACTIVE_PROCESS_COUNT.load(Ordering::Relaxed)
}

pub fn increment_process_count() {
    ACTIVE_PROCESS_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn decrement_process_count() {
    ACTIVE_PROCESS_COUNT.fetch_sub(1, Ordering::Relaxed);
}

pub enum Resource {
    TcpListener(TcpListener),
    TcpStream(TcpStream),
}

pub struct ProcessState {
    pub pid: ProcessId,
    pub mailbox: VecDeque<Vec<u8>>,
    pub receiver: mpsc::Receiver<ProcessMessage>,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
    pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
    pub resources: HashMap<u64, Resource>,
    pub next_resource_id: u64,
    pub max_mailbox_size: usize,
    pub shutdown_flag: Arc<AtomicUsize>,
    pub mailbox_counter: Arc<AtomicUsize>,
}

#[allow(dead_code)]
pub enum ProcessMessage {
    Deliver(Vec<u8>),
    Shutdown,
}

pub enum SchedulerCommand {
    Spawn {
        func_name: String,
        reply: mpsc::Sender<ProcessId>,
    },
    SpawnClosure {
        closure_ptr: i64,
        closure_bytes: Vec<u8>,
        reply: mpsc::Sender<ProcessId>,
    },
    Send {
        target_pid: ProcessId,
        message_bytes: Vec<u8>,
        reply: Option<mpsc::Sender<bool>>,
    },
    MailboxSize {
        target_pid: ProcessId,
        reply: mpsc::Sender<usize>,
    },
    Exited {
        pid: ProcessId,
    },
}

impl ProcessState {
    pub fn new(
        pid: ProcessId,
        receiver: mpsc::Receiver<ProcessMessage>,
        scheduler_tx: mpsc::Sender<SchedulerCommand>,
        wasi: wasmtime_wasi::p1::WasiP1Ctx,
        shutdown_flag: Arc<AtomicUsize>,
        mailbox_counter: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            pid,
            mailbox: VecDeque::new(),
            receiver,
            scheduler_tx,
            wasi,
            resources: HashMap::new(),
            next_resource_id: 0,
            max_mailbox_size: DEFAULT_MAX_MAILBOX_SIZE,
            shutdown_flag,
            mailbox_counter,
        }
    }

    pub fn register_resource(&mut self, r: Resource) -> u64 {
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        self.resources.insert(id, r);
        id
    }

    #[allow(dead_code)]
    pub fn get_resource(&self, id: u64) -> Option<&Resource> {
        self.resources.get(&id)
    }

    pub fn get_resource_mut(&mut self, id: u64) -> Option<&mut Resource> {
        self.resources.get_mut(&id)
    }

    pub fn close_resource(&mut self, id: u64) {
        self.resources.remove(&id);
    }
}

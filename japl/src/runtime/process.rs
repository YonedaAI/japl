use std::collections::HashMap;
use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;

pub type ProcessId = u64;

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
    ) -> Self {
        Self {
            pid,
            mailbox: VecDeque::new(),
            receiver,
            scheduler_tx,
            wasi,
            resources: HashMap::new(),
            next_resource_id: 0,
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

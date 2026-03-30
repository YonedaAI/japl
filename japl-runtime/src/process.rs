use std::collections::VecDeque;
use std::sync::mpsc;

pub type ProcessId = u64;

pub struct ProcessState {
    pub pid: ProcessId,
    pub mailbox: VecDeque<i64>,
    pub receiver: mpsc::Receiver<ProcessMessage>,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
    pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
}

/// Messages delivered to a process
#[allow(dead_code)]
pub enum ProcessMessage {
    /// A message to add to the mailbox
    Deliver(i64),
    /// Kill this process
    Shutdown,
}

/// Commands a process sends to the scheduler
pub enum SchedulerCommand {
    /// Spawn a new process running the named export
    Spawn {
        func_name: String,
        reply: mpsc::Sender<ProcessId>,
    },
    /// Send a message to another process
    Send {
        target_pid: ProcessId,
        message: i64,
    },
    /// Notify the scheduler that a process has exited
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
        }
    }
}

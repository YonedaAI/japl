//! JAPL process model: lightweight, isolated, preemptively scheduled processes.
//!
//! Each process has its own heap, mailbox, and set of links/monitors. Processes
//! communicate exclusively through message passing -- there is no shared mutable
//! state. This isolation guarantees that a crash in one process cannot corrupt
//! the state of another.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::CrashReason;
use crate::gc::ProcessHeap;
use crate::mailbox::Mailbox;
use crate::value::ProcessId;

/// Global monotonic process ID counter.
static NEXT_PID: AtomicU64 = AtomicU64::new(1);

/// Allocate a fresh, globally unique process ID.
pub fn next_pid() -> ProcessId {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}

/// The lifecycle state of a process (Section 6.6 of the spec).
///
/// ```text
/// Spawned --> Running --> (Waiting <--> Running) --> Exited(reason)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    /// Process has been created but not yet scheduled.
    Spawned,
    /// Process is actively executing code.
    Running,
    /// Process is blocked on a `receive` operation.
    Waiting,
    /// Process completed normally.
    Done,
    /// Process terminated due to a failure.
    Failed(CrashReason),
}

/// Process priority levels for the scheduler.
///
/// Higher-priority processes get more frequent scheduling slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Normal priority (default).
    Normal,
    /// High priority -- scheduled more frequently.
    High,
    /// Maximum priority -- always scheduled before lower priorities.
    Max,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// The function a process executes.
///
/// Takes a mutable reference to the process context so the function body
/// can access the mailbox and other process-local state.
pub type ProcessFn = Box<dyn FnOnce(&mut ProcessContext) + Send + 'static>;

/// Context passed to a running process, providing access to process-local resources.
pub struct ProcessContext {
    /// This process's ID.
    pub pid: ProcessId,
    /// The process mailbox for receiving messages.
    pub mailbox: Mailbox,
}

/// A JAPL lightweight process.
///
/// Processes are the fundamental unit of concurrency. Each process is isolated:
/// - Own heap (GC managed independently)
/// - Own mailbox (lock-free MPSC)
/// - Set of links (bidirectional crash propagation)
/// - Set of monitors (unidirectional crash notification)
pub struct Process {
    /// Unique process identifier.
    pub id: ProcessId,
    /// Current lifecycle state.
    pub state: ProcessState,
    /// The process's mailbox for receiving messages.
    pub mailbox: Mailbox,
    /// Parent process that spawned this one.
    pub parent: Option<ProcessId>,
    /// Bidirectional links: if a linked process crashes, this one receives an exit signal.
    pub links: Vec<ProcessId>,
    /// Unidirectional monitors: this process is notified when monitored processes exit.
    pub monitors: Vec<ProcessId>,
    /// Per-process garbage-collected heap.
    pub heap: ProcessHeap,
    /// Process priority for scheduling.
    pub priority: Priority,
    /// Reduction budget remaining (for preemption).
    pub reductions: u32,
    /// The function this process will execute.
    pub entry: Option<ProcessFn>,
}

/// Default reduction budget per scheduling quantum.
pub const DEFAULT_REDUCTIONS: u32 = 4000;

impl Process {
    /// Create a new process with the given entry function.
    pub fn new(entry: ProcessFn) -> Self {
        let id = next_pid();
        Process {
            id,
            state: ProcessState::Spawned,
            mailbox: Mailbox::new(),
            parent: None,
            links: Vec::new(),
            monitors: Vec::new(),
            heap: ProcessHeap::new(),
            priority: Priority::Normal,
            reductions: DEFAULT_REDUCTIONS,
            entry: Some(entry),
        }
    }

    /// Create a new process with a specific ID (for testing).
    pub fn with_id(id: ProcessId, entry: ProcessFn) -> Self {
        Process {
            id,
            state: ProcessState::Spawned,
            mailbox: Mailbox::new(),
            parent: None,
            links: Vec::new(),
            monitors: Vec::new(),
            heap: ProcessHeap::new(),
            priority: Priority::Normal,
            reductions: DEFAULT_REDUCTIONS,
            entry: Some(entry),
        }
    }

    /// Add a bidirectional link to another process.
    pub fn link(&mut self, other: ProcessId) {
        if !self.links.contains(&other) {
            self.links.push(other);
        }
    }

    /// Remove a bidirectional link.
    pub fn unlink(&mut self, other: ProcessId) {
        self.links.retain(|&pid| pid != other);
    }

    /// Add a monitor on another process.
    pub fn monitor(&mut self, target: ProcessId) {
        if !self.monitors.contains(&target) {
            self.monitors.push(target);
        }
    }

    /// Remove a monitor.
    pub fn demonitor(&mut self, target: ProcessId) {
        self.monitors.retain(|&pid| pid != target);
    }

    /// Check if this process has finished (normally or via crash).
    pub fn is_finished(&self) -> bool {
        matches!(self.state, ProcessState::Done | ProcessState::Failed(_))
    }

    /// Get the crash reason if the process has failed.
    pub fn crash_reason(&self) -> Option<&CrashReason> {
        match &self.state {
            ProcessState::Failed(reason) => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_creation() {
        let p = Process::new(Box::new(|_ctx| {}));
        assert!(p.id > 0);
        assert_eq!(p.state, ProcessState::Spawned);
        assert!(p.parent.is_none());
        assert!(p.links.is_empty());
        assert!(p.monitors.is_empty());
    }

    #[test]
    fn test_unique_pids() {
        let p1 = Process::new(Box::new(|_ctx| {}));
        let p2 = Process::new(Box::new(|_ctx| {}));
        assert_ne!(p1.id, p2.id);
    }

    #[test]
    fn test_link_unlink() {
        let mut p = Process::new(Box::new(|_ctx| {}));
        p.link(42);
        assert!(p.links.contains(&42));
        p.link(42); // duplicate should be ignored
        assert_eq!(p.links.len(), 1);
        p.unlink(42);
        assert!(!p.links.contains(&42));
    }

    #[test]
    fn test_monitor_demonitor() {
        let mut p = Process::new(Box::new(|_ctx| {}));
        p.monitor(99);
        assert!(p.monitors.contains(&99));
        p.demonitor(99);
        assert!(!p.monitors.contains(&99));
    }

    #[test]
    fn test_is_finished() {
        let mut p = Process::new(Box::new(|_ctx| {}));
        assert!(!p.is_finished());

        p.state = ProcessState::Done;
        assert!(p.is_finished());

        p.state = ProcessState::Failed(CrashReason::Timeout);
        assert!(p.is_finished());
    }

    #[test]
    fn test_crash_reason() {
        let mut p = Process::new(Box::new(|_ctx| {}));
        assert!(p.crash_reason().is_none());

        p.state = ProcessState::Failed(CrashReason::Custom("boom".into()));
        assert_eq!(
            p.crash_reason(),
            Some(&CrashReason::Custom("boom".into()))
        );
    }
}

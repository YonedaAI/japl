//! M:N work-stealing scheduler for JAPL processes.
//!
//! The scheduler maps many lightweight JAPL processes (M) onto a smaller
//! number of OS threads (N). Each OS thread (worker) maintains a local
//! run queue implemented as a work-stealing deque. When a worker's local
//! queue is empty, it attempts to steal work from other workers.
//!
//! Preemption is achieved via reduction counting: each process gets a
//! budget of reductions (approximately one per function call). When the
//! budget is exhausted, the process yields back to the scheduler.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crossbeam_deque::{Injector, Steal, Stealer, Worker as CbWorker};
use parking_lot::Mutex;

use crate::error::{CrashReason, RuntimeError};
use crate::process::{Process, ProcessContext, ProcessState};
use crate::value::ProcessId;

/// The main scheduler managing all JAPL processes.
///
/// The scheduler owns a set of worker threads, a global injection queue
/// for newly spawned processes, and a process table tracking all processes.
pub struct Scheduler {
    /// Global injection queue: new processes are placed here.
    injector: Arc<Injector<ProcessId>>,
    /// Stealers for each worker's local queue (used by other workers to steal).
    stealers: Vec<Stealer<ProcessId>>,
    /// Worker thread handles.
    worker_handles: Vec<JoinHandle<()>>,
    /// Shared process table: maps PID to process state.
    process_table: Arc<Mutex<HashMap<ProcessId, ProcessEntry>>>,
    /// Flag to signal workers to shut down.
    shutdown: Arc<AtomicBool>,
    /// Number of worker threads.
    num_workers: usize,
    /// Counter for completed processes (for synchronization).
    completed_count: Arc<AtomicU64>,
}

/// An entry in the process table, tracking the process and its metadata.
struct ProcessEntry {
    process: Process,
}

impl Scheduler {
    /// Create a new scheduler with the specified number of worker threads.
    ///
    /// Workers are not started until `run()` is called.
    pub fn new(num_workers: usize) -> Self {
        let num_workers = num_workers.max(1);
        Scheduler {
            injector: Arc::new(Injector::new()),
            stealers: Vec::new(),
            worker_handles: Vec::new(),
            process_table: Arc::new(Mutex::new(HashMap::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
            num_workers,
            completed_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Spawn a new process and return its PID.
    ///
    /// The process is placed in the global injection queue and will be
    /// picked up by the next available worker.
    pub fn spawn(&self, process: Process) -> ProcessId {
        let pid = process.id;
        {
            let mut table = self.process_table.lock();
            table.insert(pid, ProcessEntry { process });
        }
        self.injector.push(pid);
        pid
    }

    /// Spawn a process from a closure.
    pub fn spawn_fn<F>(&self, f: F) -> ProcessId
    where
        F: FnOnce(&mut ProcessContext) + Send + 'static,
    {
        let process = Process::new(Box::new(f));
        self.spawn(process)
    }

    /// Send a message to a process by PID.
    pub fn send_message(&self, pid: ProcessId, msg: crate::value::Value) -> Result<(), RuntimeError> {
        let table = self.process_table.lock();
        if let Some(entry) = table.get(&pid) {
            entry.process.mailbox.send(msg);
            Ok(())
        } else {
            Err(RuntimeError::ProcessNotFound(pid))
        }
    }

    /// Get the state of a process.
    pub fn process_state(&self, pid: ProcessId) -> Option<ProcessState> {
        let table = self.process_table.lock();
        table.get(&pid).map(|e| e.process.state.clone())
    }

    /// Get a list of all process IDs.
    pub fn process_list(&self) -> Vec<ProcessId> {
        let table = self.process_table.lock();
        table.keys().copied().collect()
    }

    /// Get the number of completed processes.
    pub fn completed_count(&self) -> u64 {
        self.completed_count.load(Ordering::SeqCst)
    }

    /// Run the scheduler, starting worker threads that process the run queues.
    ///
    /// This method spawns worker threads and returns immediately. Use
    /// `shutdown()` to stop all workers.
    pub fn start(&mut self) {
        self.shutdown.store(false, Ordering::SeqCst);

        let mut workers = Vec::new();
        let mut stealers = Vec::new();

        // Create worker deques
        for _ in 0..self.num_workers {
            let w = CbWorker::new_fifo();
            stealers.push(w.stealer());
            workers.push(w);
        }

        self.stealers = stealers.clone();

        // Start worker threads
        for (worker_id, local_queue) in workers.into_iter().enumerate() {
            let injector = Arc::clone(&self.injector);
            let process_table = Arc::clone(&self.process_table);
            let shutdown = Arc::clone(&self.shutdown);
            let stealers = stealers.clone();
            let completed_count = Arc::clone(&self.completed_count);

            let handle = thread::Builder::new()
                .name(format!("japl-worker-{}", worker_id))
                .spawn(move || {
                    worker_loop(
                        worker_id,
                        local_queue,
                        injector,
                        stealers,
                        process_table,
                        shutdown,
                        completed_count,
                    );
                })
                .expect("failed to spawn worker thread");

            self.worker_handles.push(handle);
        }
    }

    /// Signal all workers to shut down and wait for them to finish.
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        for handle in self.worker_handles.drain(..) {
            let _ = handle.join();
        }
    }

    /// Check if the scheduler has been signaled to shut down.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Run the scheduler until all spawned processes complete, then shut down.
    ///
    /// This is a convenience method for batch execution.
    pub fn run_until_complete(&mut self, expected_completions: u64) {
        self.start();

        // Wait for all processes to complete
        while self.completed_count.load(Ordering::SeqCst) < expected_completions {
            thread::yield_now();
        }

        self.shutdown();
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        if !self.worker_handles.is_empty() {
            self.shutdown();
        }
    }
}

/// The main loop for a worker thread.
///
/// The worker:
/// 1. Tries to pop from its local queue
/// 2. If empty, tries to steal from the global injector
/// 3. If empty, tries to steal from another worker's queue
/// 4. If work is found, executes the process
fn worker_loop(
    _worker_id: usize,
    local_queue: CbWorker<ProcessId>,
    injector: Arc<Injector<ProcessId>>,
    stealers: Vec<Stealer<ProcessId>>,
    process_table: Arc<Mutex<HashMap<ProcessId, ProcessEntry>>>,
    shutdown: Arc<AtomicBool>,
    completed_count: Arc<AtomicU64>,
) {
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }

        // Try to get work: local queue -> global injector -> steal from others
        let pid = local_queue.pop().or_else(|| {
            // Try global injector
            loop {
                match injector.steal_batch_and_pop(&local_queue) {
                    Steal::Success(pid) => return Some(pid),
                    Steal::Empty => break,
                    Steal::Retry => continue,
                }
            }

            // Try stealing from other workers
            for stealer in &stealers {
                loop {
                    match stealer.steal() {
                        Steal::Success(pid) => return Some(pid),
                        Steal::Empty => break,
                        Steal::Retry => continue,
                    }
                }
            }

            None
        });

        let pid = match pid {
            Some(pid) => pid,
            None => {
                // No work available -- yield and try again
                thread::yield_now();
                continue;
            }
        };

        // Execute the process
        execute_process(pid, &process_table, &completed_count);
    }
}

/// Execute a single process: run its entry function and update its state.
fn execute_process(
    pid: ProcessId,
    process_table: &Arc<Mutex<HashMap<ProcessId, ProcessEntry>>>,
    completed_count: &Arc<AtomicU64>,
) {
    // Extract the entry function from the process
    let entry = {
        let mut table = process_table.lock();
        if let Some(entry) = table.get_mut(&pid) {
            entry.process.state = ProcessState::Running;
            entry.process.entry.take()
        } else {
            return;
        }
    };

    if let Some(entry_fn) = entry {
        // Create a context for the process
        // Note: we create a temporary mailbox for the context; the process's
        // actual mailbox stays in the table and is accessed via send_message.
        let mut ctx = ProcessContext {
            pid,
            mailbox: crate::mailbox::Mailbox::new(),
        };

        // Run the process entry function
        // In a full implementation, this would be interruptible via reduction counting.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            entry_fn(&mut ctx);
        }));

        // Update process state based on result, then release the lock
        // before incrementing the completed count.
        let (state, links) = {
            let mut table = process_table.lock();
            if let Some(entry) = table.get_mut(&pid) {
                match result {
                    Ok(()) => {
                        entry.process.state = ProcessState::Done;
                    }
                    Err(panic_info) => {
                        let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        entry.process.state =
                            ProcessState::Failed(CrashReason::Custom(msg));
                    }
                }
                let state = entry.process.state.clone();
                let links = entry.process.links.clone();
                (state, links)
            } else {
                return;
            }
        }; // lock released here

        completed_count.fetch_add(1, Ordering::SeqCst);

        // Handle links: propagate crash to linked processes
        if let ProcessState::Failed(ref _reason) = state {
            let mut table = process_table.lock();
            for linked_pid in &links {
                if let Some(linked_entry) = table.get_mut(linked_pid) {
                    if !linked_entry.process.is_finished() {
                        linked_entry.process.state =
                            ProcessState::Failed(CrashReason::LinkedCrash(pid));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;

    #[test]
    fn test_scheduler_creation() {
        let sched = Scheduler::new(2);
        assert_eq!(sched.num_workers, 2);
        assert!(!sched.is_shutdown());
    }

    #[test]
    fn test_spawn_and_run() {
        let mut sched = Scheduler::new(2);
        let counter = Arc::new(AtomicI64::new(0));
        let c = Arc::clone(&counter);

        sched.spawn_fn(move |_ctx| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        sched.run_until_complete(1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_spawn_multiple_processes() {
        let mut sched = Scheduler::new(4);
        let counter = Arc::new(AtomicI64::new(0));
        let num_processes = 10;

        for _ in 0..num_processes {
            let c = Arc::clone(&counter);
            sched.spawn_fn(move |_ctx| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        sched.run_until_complete(num_processes);
        assert_eq!(counter.load(Ordering::SeqCst), num_processes as i64);
    }

    #[test]
    fn test_process_completes_with_done_state() {
        let mut sched = Scheduler::new(1);
        let pid = sched.spawn_fn(|_ctx| {
            // Process does nothing and exits normally
        });

        sched.run_until_complete(1);
        let state = sched.process_state(pid);
        assert_eq!(state, Some(ProcessState::Done));
    }

    #[test]
    fn test_process_crash_sets_failed_state() {
        let mut sched = Scheduler::new(1);
        let pid = sched.spawn_fn(|_ctx| {
            panic!("intentional crash");
        });

        sched.run_until_complete(1);
        let state = sched.process_state(pid);
        assert!(matches!(state, Some(ProcessState::Failed(_))));
    }

    #[test]
    fn test_process_list() {
        let sched = Scheduler::new(1);
        let pid1 = sched.spawn_fn(|_ctx| {});
        let pid2 = sched.spawn_fn(|_ctx| {});

        let pids = sched.process_list();
        assert!(pids.contains(&pid1));
        assert!(pids.contains(&pid2));
    }

    #[test]
    fn test_scheduler_shutdown() {
        let mut sched = Scheduler::new(2);
        sched.start();
        assert!(!sched.is_shutdown());
        sched.shutdown();
        assert!(sched.is_shutdown());
    }
}

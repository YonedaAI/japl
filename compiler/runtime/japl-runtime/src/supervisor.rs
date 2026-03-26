//! Supervision tree implementation for JAPL.
//!
//! Supervisors are processes that monitor child processes and restart them
//! according to a declared strategy when they fail. This implements the
//! Erlang/OTP supervision model adapted for JAPL (Spec Section 7).
//!
//! Restart strategies:
//! - **OneForOne:** Only the crashed child is restarted.
//! - **AllForOne:** All children are restarted when one crashes.
//! - **RestForOne:** The crashed child and all children started after it are restarted.
//!
//! Restart intensity limiting prevents infinite restart loops: if `max_restarts`
//! are exceeded within `max_seconds`, the supervisor itself crashes.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::error::{CrashReason, RuntimeError};
use crate::process::Process;
use crate::value::ProcessId;

/// Restart strategy determining how sibling processes are affected
/// when one child crashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Only the crashed child is restarted.
    OneForOne,
    /// All children are terminated and restarted.
    AllForOne,
    /// The crashed child and all children started after it are restarted.
    RestForOne,
}

/// When a child should be restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Always restart, regardless of exit reason.
    Permanent,
    /// Restart only on abnormal exit (not Normal).
    Transient,
    /// Never restart.
    Temporary,
}

/// How a child should be terminated during shutdown or restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPolicy {
    /// Wait up to `ms` milliseconds for graceful shutdown, then force-kill.
    Timeout(u64),
    /// Immediately terminate the child.
    Brutal,
}

/// Specification for a supervisor: strategy, intensity limits, and children.
#[derive(Clone)]
pub struct SupervisorSpec {
    /// Restart strategy for child failures.
    pub strategy: Strategy,
    /// Maximum number of restarts allowed within the time window.
    pub max_restarts: u32,
    /// Time window (in seconds) for restart intensity tracking.
    pub max_seconds: u32,
    /// Specifications for child processes, in start order.
    pub children: Vec<ChildSpec>,
}

/// Specification for a single child process.
#[derive(Clone)]
pub struct ChildSpec {
    /// Unique name for this child within the supervisor.
    pub name: String,
    /// Factory function that creates and starts the child process.
    pub start_fn: Arc<dyn Fn() -> Process + Send + Sync>,
    /// When this child should be restarted.
    pub restart: RestartPolicy,
    /// How this child should be terminated.
    pub shutdown: ShutdownPolicy,
}

/// A running supervisor managing a set of child processes.
pub struct Supervisor {
    /// The supervisor's own process ID.
    pub pid: ProcessId,
    /// The supervisor specification.
    pub spec: SupervisorSpec,
    /// Active children: (spec, current PID).
    pub children: Vec<(ChildSpec, ProcessId)>,
    /// Number of restarts in the current time window.
    pub restart_count: u32,
    /// Start of the current restart intensity window.
    pub restart_window_start: Instant,
}

impl Supervisor {
    /// Create a new supervisor from a specification.
    ///
    /// Starts all children specified in the spec and returns the supervisor.
    pub fn new(pid: ProcessId, spec: SupervisorSpec) -> Self {
        let mut supervisor = Supervisor {
            pid,
            spec: spec.clone(),
            children: Vec::new(),
            restart_count: 0,
            restart_window_start: Instant::now(),
        };

        // Start all children
        for child_spec in &spec.children {
            let process = (child_spec.start_fn)();
            let child_pid = process.id;
            supervisor
                .children
                .push((child_spec.clone(), child_pid));
        }

        supervisor
    }

    /// Handle a child crash, applying the restart strategy.
    ///
    /// Returns `Ok(Vec<Process>)` with processes that need to be spawned,
    /// or `Err` if restart intensity is exceeded.
    pub fn handle_child_crash(
        &mut self,
        crashed_pid: ProcessId,
        reason: &CrashReason,
    ) -> Result<Vec<Process>, RuntimeError> {
        // Find the crashed child's index
        let child_index = self
            .children
            .iter()
            .position(|(_, pid)| *pid == crashed_pid);

        let child_index = match child_index {
            Some(idx) => idx,
            None => return Ok(Vec::new()), // Not our child
        };

        // Check if we should restart based on the restart policy
        let should_restart = match self.children[child_index].0.restart {
            RestartPolicy::Permanent => true,
            RestartPolicy::Transient => !matches!(reason, CrashReason::Normal),
            RestartPolicy::Temporary => false,
        };

        if !should_restart {
            return Ok(Vec::new());
        }

        // Check restart intensity
        if !self.check_restart_intensity()? {
            return Err(RuntimeError::RestartIntensityExceeded);
        }

        // Apply the restart strategy
        let new_processes = match self.spec.strategy {
            Strategy::OneForOne => self.restart_one(child_index),
            Strategy::AllForOne => self.restart_all(),
            Strategy::RestForOne => self.restart_rest(child_index),
        };

        Ok(new_processes)
    }

    /// Check and update restart intensity tracking.
    ///
    /// Returns `Ok(true)` if a restart is allowed, `Err` if the intensity
    /// limit has been exceeded.
    fn check_restart_intensity(&mut self) -> Result<bool, RuntimeError> {
        let now = Instant::now();
        let window_duration = Duration::from_secs(self.spec.max_seconds as u64);

        // Reset the window if it has expired
        if now.duration_since(self.restart_window_start) > window_duration {
            self.restart_count = 0;
            self.restart_window_start = now;
        }

        self.restart_count += 1;

        if self.restart_count > self.spec.max_restarts {
            Err(RuntimeError::RestartIntensityExceeded)
        } else {
            Ok(true)
        }
    }

    /// OneForOne: restart only the crashed child.
    fn restart_one(&mut self, child_index: usize) -> Vec<Process> {
        let spec = &self.children[child_index].0;
        let new_process = (spec.start_fn)();
        let new_pid = new_process.id;
        self.children[child_index].1 = new_pid;
        vec![new_process]
    }

    /// AllForOne: restart all children.
    fn restart_all(&mut self) -> Vec<Process> {
        let mut new_processes = Vec::new();
        for child in &mut self.children {
            let new_process = (child.0.start_fn)();
            child.1 = new_process.id;
            new_processes.push(new_process);
        }
        new_processes
    }

    /// RestForOne: restart the crashed child and all children after it.
    fn restart_rest(&mut self, child_index: usize) -> Vec<Process> {
        let mut new_processes = Vec::new();
        for i in child_index..self.children.len() {
            let new_process = (self.children[i].0.start_fn)();
            self.children[i].1 = new_process.id;
            new_processes.push(new_process);
        }
        new_processes
    }

    /// Get the PIDs of all active children.
    pub fn child_pids(&self) -> Vec<ProcessId> {
        self.children.iter().map(|(_, pid)| *pid).collect()
    }

    /// Get the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Find a child by name.
    pub fn find_child(&self, name: &str) -> Option<ProcessId> {
        self.children
            .iter()
            .find(|(spec, _)| spec.name == name)
            .map(|(_, pid)| *pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Process;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn make_child_spec(name: &str, restart: RestartPolicy) -> ChildSpec {
        ChildSpec {
            name: name.to_string(),
            start_fn: Arc::new(|| Process::new(Box::new(|_ctx| {}))),
            restart,
            shutdown: ShutdownPolicy::Timeout(5000),
        }
    }

    fn make_supervisor_spec(strategy: Strategy, children: Vec<ChildSpec>) -> SupervisorSpec {
        SupervisorSpec {
            strategy,
            max_restarts: 5,
            max_seconds: 60,
            children,
        }
    }

    #[test]
    fn test_supervisor_creation() {
        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![
                make_child_spec("child1", RestartPolicy::Permanent),
                make_child_spec("child2", RestartPolicy::Permanent),
            ],
        );

        let sup = Supervisor::new(1, spec);
        assert_eq!(sup.child_count(), 2);
        assert!(sup.find_child("child1").is_some());
        assert!(sup.find_child("child2").is_some());
        assert!(sup.find_child("nonexistent").is_none());
    }

    #[test]
    fn test_one_for_one_restart() {
        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![
                make_child_spec("child1", RestartPolicy::Permanent),
                make_child_spec("child2", RestartPolicy::Permanent),
                make_child_spec("child3", RestartPolicy::Permanent),
            ],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pids = sup.child_pids();

        // Crash child2
        let crashed_pid = original_pids[1];
        let new_processes = sup
            .handle_child_crash(crashed_pid, &CrashReason::Custom("boom".into()))
            .unwrap();

        // Only one process should be restarted
        assert_eq!(new_processes.len(), 1);
        // Child1 and child3 PIDs should be unchanged
        assert_eq!(sup.children[0].1, original_pids[0]);
        assert_eq!(sup.children[2].1, original_pids[2]);
        // Child2 should have a new PID
        assert_ne!(sup.children[1].1, original_pids[1]);
    }

    #[test]
    fn test_all_for_one_restart() {
        let spec = make_supervisor_spec(
            Strategy::AllForOne,
            vec![
                make_child_spec("child1", RestartPolicy::Permanent),
                make_child_spec("child2", RestartPolicy::Permanent),
                make_child_spec("child3", RestartPolicy::Permanent),
            ],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pids = sup.child_pids();

        // Crash child2 -- all should restart
        let crashed_pid = original_pids[1];
        let new_processes = sup
            .handle_child_crash(crashed_pid, &CrashReason::Custom("boom".into()))
            .unwrap();

        assert_eq!(new_processes.len(), 3);
        // All PIDs should be new
        for (i, original_pid) in original_pids.iter().enumerate() {
            assert_ne!(sup.children[i].1, *original_pid);
        }
    }

    #[test]
    fn test_rest_for_one_restart() {
        let spec = make_supervisor_spec(
            Strategy::RestForOne,
            vec![
                make_child_spec("child1", RestartPolicy::Permanent),
                make_child_spec("child2", RestartPolicy::Permanent),
                make_child_spec("child3", RestartPolicy::Permanent),
            ],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pids = sup.child_pids();

        // Crash child2 -- child2 and child3 should restart
        let crashed_pid = original_pids[1];
        let new_processes = sup
            .handle_child_crash(crashed_pid, &CrashReason::Custom("boom".into()))
            .unwrap();

        assert_eq!(new_processes.len(), 2);
        // Child1 should be unchanged
        assert_eq!(sup.children[0].1, original_pids[0]);
        // Child2 and child3 should have new PIDs
        assert_ne!(sup.children[1].1, original_pids[1]);
        assert_ne!(sup.children[2].1, original_pids[2]);
    }

    #[test]
    fn test_transient_restart_on_abnormal() {
        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![make_child_spec("child1", RestartPolicy::Transient)],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pid = sup.children[0].1;

        // Abnormal exit should trigger restart
        let new_processes = sup
            .handle_child_crash(original_pid, &CrashReason::Custom("crash".into()))
            .unwrap();
        assert_eq!(new_processes.len(), 1);
    }

    #[test]
    fn test_transient_no_restart_on_normal() {
        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![make_child_spec("child1", RestartPolicy::Transient)],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pid = sup.children[0].1;

        // Normal exit should NOT trigger restart
        let new_processes = sup
            .handle_child_crash(original_pid, &CrashReason::Normal)
            .unwrap();
        assert_eq!(new_processes.len(), 0);
    }

    #[test]
    fn test_temporary_never_restarts() {
        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![make_child_spec("child1", RestartPolicy::Temporary)],
        );

        let mut sup = Supervisor::new(1, spec);
        let original_pid = sup.children[0].1;

        let new_processes = sup
            .handle_child_crash(original_pid, &CrashReason::Custom("crash".into()))
            .unwrap();
        assert_eq!(new_processes.len(), 0);
    }

    #[test]
    fn test_restart_intensity_exceeded() {
        let spec = SupervisorSpec {
            strategy: Strategy::OneForOne,
            max_restarts: 2,
            max_seconds: 60,
            children: vec![make_child_spec("child1", RestartPolicy::Permanent)],
        };

        let mut sup = Supervisor::new(1, spec);

        // First two restarts should succeed
        let pid1 = sup.children[0].1;
        sup.handle_child_crash(pid1, &CrashReason::Custom("crash1".into()))
            .unwrap();

        let pid2 = sup.children[0].1;
        sup.handle_child_crash(pid2, &CrashReason::Custom("crash2".into()))
            .unwrap();

        // Third restart should exceed intensity
        let pid3 = sup.children[0].1;
        let result = sup.handle_child_crash(pid3, &CrashReason::Custom("crash3".into()));
        assert!(matches!(result, Err(RuntimeError::RestartIntensityExceeded)));
    }

    #[test]
    fn test_restart_counter_tracks_start_fn_calls() {
        let call_count = Arc::new(AtomicU32::new(0));
        let cc = Arc::clone(&call_count);

        let spec = make_supervisor_spec(
            Strategy::OneForOne,
            vec![ChildSpec {
                name: "counted".to_string(),
                start_fn: Arc::new(move || {
                    cc.fetch_add(1, Ordering::SeqCst);
                    Process::new(Box::new(|_ctx| {}))
                }),
                restart: RestartPolicy::Permanent,
                shutdown: ShutdownPolicy::Timeout(5000),
            }],
        );

        let mut sup = Supervisor::new(1, spec);
        // Initial start counts as 1
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        let pid = sup.children[0].1;
        sup.handle_child_crash(pid, &CrashReason::Custom("crash".into()))
            .unwrap();
        // Restart counts as another call
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }
}

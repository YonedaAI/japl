//! Runtime error types and crash reasons for the JAPL process model.
//!
//! JAPL uses a dual error model: domain errors (Result/Fail) for expected
//! failures and process crashes for unexpected failures. This module defines
//! the crash reason types used when processes terminate abnormally.

use std::fmt;

/// Typed crash reasons matching the JAPL spec (Section 7.7).
///
/// Unlike Erlang's untyped crash reasons, JAPL provides structured crash
/// reasons that supervisors can pattern-match on.
#[derive(Debug, Clone, PartialEq)]
pub enum CrashReason {
    /// Process completed normally.
    Normal,
    /// An assertion failed at a source location.
    AssertionFailed(String, Location),
    /// A resource (memory, file descriptors, etc.) was exhausted.
    ResourceExhausted(String),
    /// A program invariant was violated.
    InvariantViolation(String),
    /// A receive or operation timed out.
    Timeout,
    /// A linked process crashed, propagating the failure.
    LinkedCrash(u64),
    /// The process was killed externally.
    Killed,
    /// Custom crash reason with a descriptive message.
    Custom(String),
}

impl fmt::Display for CrashReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrashReason::Normal => write!(f, "normal"),
            CrashReason::AssertionFailed(msg, loc) => {
                write!(f, "assertion failed: {} at {}", msg, loc)
            }
            CrashReason::ResourceExhausted(msg) => write!(f, "resource exhausted: {}", msg),
            CrashReason::InvariantViolation(msg) => write!(f, "invariant violation: {}", msg),
            CrashReason::Timeout => write!(f, "timeout"),
            CrashReason::LinkedCrash(pid) => write!(f, "linked process {} crashed", pid),
            CrashReason::Killed => write!(f, "killed"),
            CrashReason::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Source location for assertion failures.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Runtime errors that can occur during scheduler or process operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RuntimeError {
    #[error("process {0} not found")]
    ProcessNotFound(u64),

    #[error("mailbox full for process {0}")]
    MailboxFull(u64),

    #[error("scheduler shutdown")]
    SchedulerShutdown,

    #[error("spawn failed: {0}")]
    SpawnFailed(String),

    #[error("process crashed: {0}")]
    ProcessCrashed(String),

    #[error("supervisor restart intensity exceeded")]
    RestartIntensityExceeded,
}

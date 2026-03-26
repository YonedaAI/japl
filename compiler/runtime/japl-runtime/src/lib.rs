//! JAPL Runtime System
//!
//! This crate implements the core runtime for the JAPL programming language:
//!
//! - **Process model:** Lightweight, isolated processes with per-process heaps
//! - **Scheduler:** M:N work-stealing green thread scheduler
//! - **Mailbox:** Lock-free MPSC mailbox with selective receive
//! - **Supervision:** OTP-style supervision trees with restart strategies
//! - **GC:** Per-process generational garbage collector
//! - **Values:** Runtime value representation for all JAPL types
//!
//! Processes are the fundamental unit of concurrency. They share no mutable
//! state and communicate exclusively through message passing. Each process
//! has its own heap, collected independently -- no global stop-the-world pauses.

pub mod error;
pub mod gc;
pub mod mailbox;
pub mod process;
pub mod scheduler;
pub mod supervisor;
pub mod value;

// Re-export key types at the crate root for convenience.
pub use error::{CrashReason, RuntimeError};
pub use gc::ProcessHeap;
pub use mailbox::Mailbox;
pub use process::{Process, ProcessState};
pub use scheduler::Scheduler;
pub use supervisor::{
    ChildSpec, RestartPolicy, ShutdownPolicy, Strategy, Supervisor, SupervisorSpec,
};
pub use value::{ProcessId, Value};

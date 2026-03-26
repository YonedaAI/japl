//! japl-codegen: Tree-walking interpreter for the JAPL IR.
//!
//! This is the stage-0 "code generator" -- rather than emitting native code,
//! it directly interprets the IR. This is simpler and sufficient for the
//! bootstrap compiler.

mod env;
mod interpreter;

pub use interpreter::{Interpreter, InterpreterError};

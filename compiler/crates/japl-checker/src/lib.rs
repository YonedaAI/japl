#![allow(unused_variables, dead_code)]

//! japl-checker: Type checking, effect checking, and linearity checking for JAPL.
//!
//! This crate houses the semantic analysis passes that run after parsing:
//! 1. **Type checking** (bidirectional, with unification and row polymorphism)
//! 2. **Effect checking** (effect row unification and subsumption)
//! 3. **Linearity checking** (linear resource use-once verification)

pub mod env;
pub mod errors;
pub mod effects;
pub mod infer;
pub mod linearity;
pub mod unify;

pub use env::TypeEnv;
pub use errors::TypeError;
pub use infer::TypeChecker;
pub use linearity::LinearityChecker;

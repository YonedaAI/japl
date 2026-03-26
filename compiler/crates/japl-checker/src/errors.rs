//! Error types for the type checker, effect checker, and linearity checker.

use japl_common::Span;
use japl_types::{TypeId, display_type, TypeInterner};
use thiserror::Error;

/// All possible type-checking errors.
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("type mismatch: expected {expected}, found {found}")]
    Mismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("occurs check failed: type variable ?{var_id} occurs in the type being unified")]
    OccursCheck {
        var_id: u32,
        span: Span,
    },

    #[error("undefined variable: {name}")]
    UndefinedVariable {
        name: String,
        span: Span,
    },

    #[error("undefined type: {name}")]
    UndefinedType {
        name: String,
        span: Span,
    },

    #[error("undefined constructor: {name}")]
    UndefinedConstructor {
        name: String,
        span: Span,
    },

    #[error("wrong number of arguments: expected {expected}, found {found}")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("cannot apply non-function type")]
    NotAFunction {
        actual_type: String,
        span: Span,
    },

    #[error("missing record field: {field}")]
    MissingField {
        field: String,
        span: Span,
    },

    #[error("unexpected record field: {field}")]
    ExtraField {
        field: String,
        span: Span,
    },

    #[error("record field access on non-record type")]
    NotARecord {
        actual_type: String,
        span: Span,
    },

    #[error("non-exhaustive match: missing patterns")]
    NonExhaustiveMatch {
        missing: Vec<String>,
        span: Span,
    },

    #[error("duplicate field in record: {field}")]
    DuplicateField {
        field: String,
        span: Span,
    },

    #[error("cannot unify record rows")]
    RowMismatch {
        span: Span,
    },

    #[error("cannot solve rigid type variable ?{var_id}")]
    RigidVariable {
        var_id: u32,
        span: Span,
    },

    #[error("cannot solve skolem variable ?{var_id}")]
    SkolemEscape {
        var_id: u32,
        span: Span,
    },
}

impl TypeError {
    /// Return the source span associated with this error.
    pub fn span(&self) -> Span {
        match self {
            TypeError::Mismatch { span, .. }
            | TypeError::OccursCheck { span, .. }
            | TypeError::UndefinedVariable { span, .. }
            | TypeError::UndefinedType { span, .. }
            | TypeError::UndefinedConstructor { span, .. }
            | TypeError::ArityMismatch { span, .. }
            | TypeError::NotAFunction { span, .. }
            | TypeError::MissingField { span, .. }
            | TypeError::ExtraField { span, .. }
            | TypeError::NotARecord { span, .. }
            | TypeError::NonExhaustiveMatch { span, .. }
            | TypeError::DuplicateField { span, .. }
            | TypeError::RowMismatch { span, .. }
            | TypeError::RigidVariable { span, .. }
            | TypeError::SkolemEscape { span, .. } => *span,
        }
    }

    /// Create a type mismatch error with pretty-printed types.
    pub fn mismatch(expected: TypeId, found: TypeId, span: Span, interner: &TypeInterner) -> Self {
        TypeError::Mismatch {
            expected: display_type(expected, interner),
            found: display_type(found, interner),
            span,
        }
    }
}

/// Effect checking errors.
#[derive(Debug, Error)]
pub enum EffectError {
    #[error("effect not allowed: {effect} is not in the allowed effect set")]
    EffectNotAllowed {
        effect: String,
        span: Span,
    },

    #[error("pure function performs effectful operation: {effect}")]
    PurityViolation {
        effect: String,
        span: Span,
    },

    #[error("effect row mismatch")]
    RowMismatch {
        expected: String,
        found: String,
        span: Span,
    },
}

impl EffectError {
    pub fn span(&self) -> Span {
        match self {
            EffectError::EffectNotAllowed { span, .. }
            | EffectError::PurityViolation { span, .. }
            | EffectError::RowMismatch { span, .. } => *span,
        }
    }
}

/// Linearity checking errors.
#[derive(Debug, Error)]
pub enum LinearityError {
    #[error("resource used after move: `{name}` was consumed at {consumed_at:?}")]
    UseAfterMove {
        name: String,
        consumed_at: Span,
        use_at: Span,
    },

    #[error("resource consumed twice: `{name}`")]
    DoubleConsume {
        name: String,
        first: Span,
        second: Span,
    },

    #[error("resource not consumed: `{name}` must be used exactly once")]
    NotConsumed {
        name: String,
        declared_at: Span,
    },

    #[error("resource `{name}` consumed while borrowed")]
    ConsumedWhileBorrowed {
        name: String,
        borrow_at: Span,
        consume_at: Span,
    },

    #[error("branches consume different resources")]
    BranchMismatch {
        span: Span,
    },

    #[error("linear resource bound with `let` instead of `use`")]
    LinearWithLet {
        name: String,
        span: Span,
    },
}

impl LinearityError {
    pub fn span(&self) -> Span {
        match self {
            LinearityError::UseAfterMove { use_at, .. } => *use_at,
            LinearityError::DoubleConsume { second, .. } => *second,
            LinearityError::NotConsumed { declared_at, .. } => *declared_at,
            LinearityError::ConsumedWhileBorrowed { consume_at, .. } => *consume_at,
            LinearityError::BranchMismatch { span } => *span,
            LinearityError::LinearWithLet { span, .. } => *span,
        }
    }
}

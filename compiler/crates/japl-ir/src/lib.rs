//! japl-ir: Intermediate representation for the JAPL interpreter.
//!
//! This crate defines a simplified, typed IR that the tree-walking interpreter
//! evaluates directly. The AST-to-IR lowering pass translates the parsed AST
//! into this representation, erasing spans and simplifying the structure.

pub mod lower;

use japl_runtime::Value;

/// A complete IR program: a set of top-level definitions and an entry point.
#[derive(Debug, Clone)]
pub struct IrProgram {
    /// Top-level function definitions (name -> params + body).
    pub functions: Vec<IrFnDef>,
    /// Top-level type definitions (for constructor dispatch).
    pub type_defs: Vec<IrTypeDef>,
}

/// A top-level function definition.
#[derive(Debug, Clone)]
pub struct IrFnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: IrExpr,
}

/// A top-level type definition (sum type with constructors).
#[derive(Debug, Clone)]
pub struct IrTypeDef {
    pub name: String,
    pub variants: Vec<IrVariant>,
}

/// A variant in a sum type.
#[derive(Debug, Clone)]
pub struct IrVariant {
    pub name: String,
    pub arity: usize,
}

/// The core IR expression type.
#[derive(Debug, Clone)]
pub enum IrExpr {
    /// A literal value.
    Lit(Value),
    /// A variable reference.
    Var(String),
    /// Let binding: `let name = value in body`.
    Let(String, Box<IrExpr>, Box<IrExpr>),
    /// Function application: `func(args...)`.
    App(Box<IrExpr>, Vec<IrExpr>),
    /// Lambda: `fn(params) -> body`.
    Lambda(Vec<String>, Box<IrExpr>),
    /// If-then-else.
    If(Box<IrExpr>, Box<IrExpr>, Box<IrExpr>),
    /// Pattern match.
    Match(Box<IrExpr>, Vec<(IrPattern, Option<IrExpr>, IrExpr)>),
    /// Binary operation.
    BinOp(IrBinOp, Box<IrExpr>, Box<IrExpr>),
    /// Unary operation.
    UnaryOp(IrUnaryOp, Box<IrExpr>),
    /// Record literal: `{ field1 = val1, field2 = val2 }`.
    Record(Vec<(String, IrExpr)>),
    /// Field access: `expr.field`.
    FieldAccess(Box<IrExpr>, String),
    /// List literal: `[a, b, c]`.
    List(Vec<IrExpr>),
    /// Tuple literal: `(a, b, c)`.
    Tuple(Vec<IrExpr>),
    /// Constructor application: `Some(42)`.
    Constructor(String, Vec<IrExpr>),
    /// A block of expressions; the last is the result.
    Block(Vec<IrExpr>),
    /// String concatenation (<> operator).
    Concat(Box<IrExpr>, Box<IrExpr>),
    /// Pipeline: `x |> f` desugars to `f(x)`.
    Pipeline(Box<IrExpr>, Box<IrExpr>),
}

/// Binary operators in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

/// Unary operators in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrUnaryOp {
    Neg,
    Not,
}

/// IR pattern for match expressions.
#[derive(Debug, Clone)]
pub enum IrPattern {
    /// Wildcard: matches anything, binds nothing.
    Wildcard,
    /// Variable binding: matches anything, binds the value to the name.
    Var(String),
    /// Literal pattern: matches a specific value.
    Literal(Value),
    /// Constructor pattern: `Ctor(p1, p2, ...)`.
    Constructor(String, Vec<IrPattern>),
    /// Tuple pattern: `(p1, p2, ...)`.
    Tuple(Vec<IrPattern>),
    /// Record pattern: `{ field1, field2, ... }`.
    Record(Vec<(String, IrPattern)>),
    /// List pattern: `[p1, p2, ..rest]`.
    List(Vec<IrPattern>, Option<Box<IrPattern>>),
    /// Or pattern: `p1 | p2`.
    Or(Vec<IrPattern>),
}

/// Errors that can occur during IR lowering.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LowerError {
    #[error("unsupported expression: {0}")]
    Unsupported(String),
    #[error("invalid literal: {0}")]
    InvalidLiteral(String),
}

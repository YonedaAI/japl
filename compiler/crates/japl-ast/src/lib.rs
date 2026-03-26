//! japl-ast: Abstract Syntax Tree types for JAPL.
//!
//! This crate defines all AST node types produced by the parser
//! and consumed by the type checker and later passes.

pub mod pretty;

use japl_common::Span;
use smol_str::SmolStr;

/// Unique identifier assigned during parsing, used for later passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// A complete source file.
#[derive(Debug)]
pub struct SourceFile {
    pub module_decl: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub items: Vec<Item>,
    pub span: Span,
}

// -- Top-level Items --

#[derive(Debug)]
pub enum Item {
    FnDef(FnDef),
    TypeDef(TypeDef),
    TypeAlias(TypeAlias),
    TraitDef(TraitDef),
    ImplBlock(ImplBlock),
    ModuleDef(ModuleDef),
    SignatureDef(SignatureDef),
    ForeignBlock(ForeignBlock),
    TestDef(TestDef),
    PropertyDef(PropertyDef),
    BenchDef(BenchDef),
    SupervisorDef(SupervisorDef),
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub name: QualifiedName,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: QualifiedName,
    pub items: Option<Vec<ImportItem>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportItem {
    Name(SmolStr),
    Type(SmolStr),
}

#[derive(Debug, Clone)]
pub struct QualifiedName {
    pub segments: Vec<SmolStr>,
    pub span: Span,
}

// -- Function Definition --

#[derive(Debug)]
pub struct FnDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub effects: Vec<EffectExpr>,
    pub where_clause: Vec<Constraint>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub id: NodeId,
    pub pattern: Pattern,
    pub ty: Option<TypeExpr>,
    pub ownership: Ownership,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Value,
    Own,
    Ref,
}

#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: SmolStr,
    pub bounds: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub trait_name: QualifiedName,
    pub type_args: Vec<TypeExpr>,
    pub span: Span,
}

// -- Type Definitions --

#[derive(Debug)]
pub struct TypeDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub deriving: Vec<SmolStr>,
    pub is_packed: bool,
    pub body: TypeBody,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeBody {
    Sum(Vec<Variant>),
    Record(Vec<FieldDef>),
    Capability(Vec<CapabilityMethod>),
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: SmolStr,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: SmolStr,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CapabilityMethod {
    pub name: SmolStr,
    pub params: Vec<TypeExpr>,
    pub return_type: TypeExpr,
    pub effects: Vec<EffectExpr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypeAlias {
    pub id: NodeId,
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub target: TypeExpr,
    pub span: Span,
}

// -- Trait and Impl --

#[derive(Debug)]
pub struct TraitDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub supertraits: Vec<Constraint>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

#[derive(Debug)]
pub struct ImplBlock {
    pub id: NodeId,
    pub trait_name: QualifiedName,
    pub type_args: Vec<TypeExpr>,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

// -- Module and Signature --

#[derive(Debug)]
pub struct ModuleDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub signature: Option<QualifiedName>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug)]
pub struct SignatureDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub items: Vec<SignatureItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum SignatureItem {
    TypeDecl { name: SmolStr, span: Span },
    FnDecl(FnSignature),
}

#[derive(Debug, Clone)]
pub struct FnSignature {
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<TypeExpr>,
    pub return_type: TypeExpr,
    pub effects: Vec<EffectExpr>,
    pub span: Span,
}

// -- Foreign --

#[derive(Debug)]
pub struct ForeignBlock {
    pub abi: SmolStr,
    pub items: Vec<ForeignItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ForeignItem {
    Fn(FnSignature),
    Module {
        name: SmolStr,
        items: Vec<FnSignature>,
        span: Span,
    },
}

// -- Test / Property / Bench --

#[derive(Debug)]
pub struct TestDef {
    pub name: SmolStr,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct PropertyDef {
    pub name: SmolStr,
    pub generators: Vec<Param>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug)]
pub struct BenchDef {
    pub name: SmolStr,
    pub body: Expr,
    pub span: Span,
}

// -- Supervisor --

#[derive(Debug)]
pub struct SupervisorDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub strategy: Expr,
    pub children: Vec<Expr>,
    pub span: Span,
}

// -- Expressions --

#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal: `42`, `0xFF`
    IntLit { value: SmolStr, span: Span },

    /// Float literal: `3.14`
    FloatLit { value: SmolStr, span: Span },

    /// String literal (possibly with interpolation segments)
    StringLit {
        segments: Vec<StringSegment>,
        span: Span,
    },

    /// Character literal: `'x'`
    CharLit { value: char, span: Span },

    /// Boolean literal
    BoolLit { value: bool, span: Span },

    /// Unit literal: `()`
    UnitLit { span: Span },

    /// Variable reference: `x`, `foo_bar`
    Var {
        name: SmolStr,
        id: NodeId,
        span: Span,
    },

    /// Constructor reference: `Some`, `Ok`
    Constructor {
        name: QualifiedName,
        id: NodeId,
        span: Span,
    },

    /// Field access: `expr.field`
    FieldAccess {
        expr: Box<Expr>,
        field: SmolStr,
        span: Span,
    },

    /// Function application: `f(x, y)`
    App {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    /// Binary operation: `a + b`
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// Unary operation: `-x`, `!b`
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },

    /// Error propagation: `expr?`
    Try { expr: Box<Expr>, span: Span },

    /// Pipeline: `x |> f` desugars to `f(x)`
    Pipeline {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// Function composition: `f >> g`
    Compose {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },

    /// Lambda: `fn x -> x + 1` or `fn(x, y) -> x + y`
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        span: Span,
    },

    /// Let binding: `let x = e1 in e2`
    Let {
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },

    /// Use binding (linear resource): `use x = e1 in e2`
    Use {
        pattern: Pattern,
        ty: Option<TypeExpr>,
        value: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },

    /// If-then-else
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },

    /// Match expression
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// Block (sequence of expressions, last is the value)
    Block { exprs: Vec<Expr>, span: Span },

    /// Record literal: `{ x = 1, y = 2 }`
    RecordLit {
        fields: Vec<(SmolStr, Expr)>,
        span: Span,
    },

    /// Record update: `{ expr | field = value }`
    RecordUpdate {
        base: Box<Expr>,
        updates: Vec<(SmolStr, Expr)>,
        span: Span,
    },

    /// List literal: `[1, 2, 3]`
    ListLit { elements: Vec<Expr>, span: Span },

    /// Tuple literal: `(1, "hello", True)`
    TupleLit { elements: Vec<Expr>, span: Span },

    /// Loop expression
    Loop {
        bindings: Vec<(SmolStr, Expr)>,
        condition: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },

    /// Continue (in loop): `continue(new_i, new_acc)`
    Continue { args: Vec<Expr>, span: Span },

    /// Receive expression (in process context)
    Receive {
        arms: Vec<MatchArm>,
        timeout: Option<Box<Expr>>,
        span: Span,
    },

    /// Type annotation: `(expr : Type)`
    Annotation {
        expr: Box<Expr>,
        ty: TypeExpr,
        span: Span,
    },
}

impl Expr {
    /// Returns the span of this expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::FloatLit { span, .. }
            | Expr::StringLit { span, .. }
            | Expr::CharLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::UnitLit { span }
            | Expr::Var { span, .. }
            | Expr::Constructor { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::App { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::Try { span, .. }
            | Expr::Pipeline { span, .. }
            | Expr::Compose { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Let { span, .. }
            | Expr::Use { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::Block { span, .. }
            | Expr::RecordLit { span, .. }
            | Expr::RecordUpdate { span, .. }
            | Expr::ListLit { span, .. }
            | Expr::TupleLit { span, .. }
            | Expr::Loop { span, .. }
            | Expr::Continue { span, .. }
            | Expr::Receive { span, .. }
            | Expr::Annotation { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum StringSegment {
    Literal(SmolStr),
    Interpolation(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
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
    Concat,
    Append,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

// -- Patterns --

#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard { span: Span },

    /// Variable binding: `x`
    Var {
        name: SmolStr,
        id: NodeId,
        span: Span,
    },

    /// Pinned variable: `^x`
    Pin {
        name: SmolStr,
        id: NodeId,
        span: Span,
    },

    /// Constructor pattern: `Some(x)`, `Err(e)`
    Constructor {
        name: QualifiedName,
        fields: Vec<Pattern>,
        span: Span,
    },

    /// Literal pattern: `42`, `"hello"`, `True`
    Literal { expr: Box<Expr>, span: Span },

    /// Record pattern: `{ name, age }`
    Record {
        fields: Vec<(SmolStr, Pattern)>,
        rest: bool,
        span: Span,
    },

    /// List pattern: `[x, y, ..rest]`
    List {
        elements: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },

    /// Tuple pattern: `(a, b, c)`
    Tuple {
        elements: Vec<Pattern>,
        span: Span,
    },

    /// Or pattern: `Some(1) | Some(2)`
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },

    /// As pattern: `pattern as name`
    As {
        pattern: Box<Pattern>,
        name: SmolStr,
        id: NodeId,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span }
            | Pattern::Var { span, .. }
            | Pattern::Pin { span, .. }
            | Pattern::Constructor { span, .. }
            | Pattern::Literal { span, .. }
            | Pattern::Record { span, .. }
            | Pattern::List { span, .. }
            | Pattern::Tuple { span, .. }
            | Pattern::Or { span, .. }
            | Pattern::As { span, .. } => *span,
        }
    }
}

// -- Type Expressions --

#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// Named type: `Int`, `List[a]`, `Result[a, e]`
    Named {
        name: QualifiedName,
        args: Vec<TypeExpr>,
        span: Span,
    },

    /// Type variable: `a`, `b`
    Var { name: SmolStr, span: Span },

    /// Function type: `fn(A, B) -> C`
    Fn {
        params: Vec<TypeExpr>,
        return_type: Box<TypeExpr>,
        effects: Vec<EffectExpr>,
        span: Span,
    },

    /// Record type: `{ name: String, age: Int }`
    Record {
        fields: Vec<FieldDef>,
        row_var: Option<SmolStr>,
        span: Span,
    },

    /// Tuple type: `(Int, String, Bool)`
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },

    /// Owned type: `own Buffer`
    Owned {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Ref type: `ref Buffer`
    Borrowed {
        inner: Box<TypeExpr>,
        span: Span,
    },

    /// Never type
    Never { span: Span },

    /// Unit type
    Unit { span: Span },

    /// Forall type: `forall a b. ...`
    Forall {
        params: Vec<TypeParam>,
        body: Box<TypeExpr>,
        span: Span,
    },
}

// -- Effect Expressions --

#[derive(Debug, Clone)]
pub enum EffectExpr {
    /// Named effect: `Io`, `Net`, `Process[Msg]`
    Named {
        name: QualifiedName,
        args: Vec<TypeExpr>,
        span: Span,
    },
    /// Effect variable (for polymorphism): `e`
    Var { name: SmolStr, span: Span },
}

// -- Match Arms --

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
    pub span: Span,
}

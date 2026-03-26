# JAPL Compiler Architecture

> Stage-0 bootstrap compiler written in Rust, compiling JAPL to native code.
>
> Version: 0.1.0-draft
> Date: 2026-03-26

---

## Table of Contents

1. [Compiler Pipeline Overview](#1-compiler-pipeline-overview)
2. [Lexer Design](#2-lexer-design)
3. [Parser Design](#3-parser-design)
4. [Type Checker](#4-type-checker)
5. [Linearity Checker](#5-linearity-checker)
6. [IR Design](#6-ir-design)
7. [Code Generation](#7-code-generation)
8. [Runtime System](#8-runtime-system)
9. [Standard Library Architecture](#9-standard-library-architecture)
10. [Rust Project Structure](#10-rust-project-structure)
11. [Build System](#11-build-system)

---

## 1. Compiler Pipeline Overview

```
Source (.japl)
    │
    ▼
┌──────────┐   Token stream    ┌──────────┐   Untyped AST    ┌─────────────────┐
│  Lexer   │ ───────────────▶  │  Parser  │ ──────────────▶  │ Name Resolution │
│(japl-lexer)│                 │(japl-parser)│                │  (japl-checker) │
└──────────┘                   └──────────┘                   └────────┬────────┘
                                                                       │ Resolved AST
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │  Type Checker   │
                                                              │  (japl-checker) │
                                                              └────────┬────────┘
                                                                       │ Typed AST
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │ Effect Checker  │
                                                              │  (japl-checker) │
                                                              └────────┬────────┘
                                                                       │ Effect-annotated AST
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │Linearity Checker│
                                                              │  (japl-checker) │
                                                              └────────┬────────┘
                                                                       │ Verified AST
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │   IR Lowering   │
                                                              │   (japl-ir)     │
                                                              └────────┬────────┘
                                                                       │ JAPL MIR
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │  Optimization   │
                                                              │   (japl-ir)     │
                                                              └────────┬────────┘
                                                                       │ Optimized MIR
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │ Code Generation │
                                                              │ (japl-codegen)  │
                                                              └────────┬────────┘
                                                                       │ Object file (.o)
                                                                       ▼
                                                              ┌─────────────────┐
                                                              │    Linking      │
                                                              │ (japl-codegen)  │
                                                              └────────┘────────┘
                                                                       │
                                                                       ▼
                                                                  Binary / Library
```

### Error Reporting Strategy (All Stages)

Every compiler stage produces structured diagnostics via a shared `Diagnostic` type. Errors carry source spans, severity levels, and optional fix suggestions. The driver collects diagnostics from all stages and renders them through a configurable reporter (terminal with colors, JSON for editor integration, or SARIF for CI).

```rust
// In japl-ast/src/diagnostic.rs

/// A byte offset into a source file.
pub type ByteOffset = u32;

/// A span in source code, identified by file and byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file_id: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub style: LabelStyle, // Primary, Secondary
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,   // e.g., E0001, W0042
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub suggestion: Option<Suggestion>,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub replacements: Vec<(Span, String)>,
}

/// Accumulator used throughout compilation.
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
    has_errors: bool,
}
```

**Crate:** `codespan-reporting` for rendering diagnostics to the terminal.

---

## 2. Lexer Design

**Crate:** `japl-lexer`
**Rust dependency:** `logos` 0.14 for DFA-based lexing (zero-allocation, extremely fast).

### Token Types

```rust
// japl-lexer/src/token.rs

use logos::Logos;

/// Every token produced by the JAPL lexer.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")]  // skip horizontal whitespace
pub enum Token {
    // ── Keywords ────────────────────────────────────────────
    #[token("fn")]       Fn,
    #[token("let")]      Let,
    #[token("match")]    Match,
    #[token("with")]     With,
    #[token("type")]     Type,
    #[token("module")]   Module,
    #[token("import")]   Import,
    #[token("if")]       If,
    #[token("then")]     Then,
    #[token("else")]     Else,
    #[token("use")]      Use,
    #[token("own")]      Own,
    #[token("ref")]      Ref,
    #[token("opaque")]   Opaque,
    #[token("trait")]    Trait,
    #[token("impl")]     Impl,
    #[token("deriving")]  Deriving,
    #[token("test")]     Test,
    #[token("property")] Property,
    #[token("forall")]   Forall,
    #[token("assert")]   Assert,
    #[token("where")]    Where,
    #[token("signature")] Signature,
    #[token("foreign")]  Foreign,
    #[token("unsafe")]   Unsafe,
    #[token("loop")]     Loop,
    #[token("while")]    While,
    #[token("do")]       Do,
    #[token("continue")] Continue,
    #[token("packed")]   Packed,
    #[token("bench")]    Bench,
    #[token("receive")]  Receive,
    #[token("supervisor")] Supervisor,
    #[token("strategy")] Strategy,
    #[token("child")]    Child,
    #[token("capability")] Capability,
    #[token("alias")]    Alias,

    // ── Literals ────────────────────────────────────────────
    #[regex(r"[0-9][0-9_]*", priority = 2)]
    IntLiteral,

    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*")]
    HexIntLiteral,

    #[regex(r"0b[01][01_]*")]
    BinIntLiteral,

    #[regex(r"0o[0-7][0-7_]*")]
    OctIntLiteral,

    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?")]
    FloatLiteral,

    // String literals are handled by a custom callback (see below)
    // to support interpolation and escape sequences.
    StringStart,    // opening "
    StringContent,  // literal text between interpolations
    StringInterpolationStart,  // ${
    StringInterpolationEnd,    // } closing interpolation
    StringEnd,      // closing "

    #[regex(r"'[^'\\]'|'\\[nrt\\0']'|'\\u\{[0-9a-fA-F]+\}'")]
    CharLiteral,

    #[token("True")]  True,
    #[token("False")] False,

    // ── Identifiers ─────────────────────────────────────────
    #[regex(r"[a-z_][a-zA-Z0-9_]*")]
    LowerIdent,     // variable or function name

    #[regex(r"[A-Z][a-zA-Z0-9_]*")]
    UpperIdent,     // type, constructor, or module name

    // ── Operators ───────────────────────────────────────────
    #[token("+")]   Plus,
    #[token("-")]   Minus,
    #[token("*")]   Star,
    #[token("/")]   Slash,
    #[token("%")]   Percent,
    #[token("==")]  EqEq,
    #[token("!=")]  BangEq,
    #[token("<")]   Lt,
    #[token(">")]   Gt,
    #[token("<=")]  LtEq,
    #[token(">=")]  GtEq,
    #[token("&&")]  AmpAmp,
    #[token("||")]  PipePipe,
    #[token("!")]   Bang,
    #[token("++")]  PlusPlus,    // string/list concatenation
    #[token("<>")]  Diamond,     // monoid append
    #[token("|>")]  PipeRight,   // pipeline
    #[token(">>")]  ComposeRight, // function composition
    #[token("->")]  Arrow,       // function type / match arm
    #[token("=>")]  FatArrow,    // type constraint / implication
    #[token("?")]   Question,    // error propagation
    #[token("=")]   Eq,
    #[token(":")]   Colon,
    #[token("|")]   Pipe,
    #[token(",")]   Comma,
    #[token(".")]   Dot,
    #[token("..")]  DotDot,      // range / spread

    // ── Delimiters ──────────────────────────────────────────
    #[token("(")]   LParen,
    #[token(")")]   RParen,
    #[token("[")]   LBracket,
    #[token("]")]   RBracket,
    #[token("{")]   LBrace,
    #[token("}")]   RBrace,

    // ── Whitespace & Structure ──────────────────────────────
    Newline,       // \n — significant for statement separation
    Indent,        // synthetic token: indentation increased
    Dedent,        // synthetic token: indentation decreased

    // ── Comments ────────────────────────────────────────────
    #[regex(r"--[^\n]*")]
    LineComment,

    // Doc comments (triple-dash)
    #[regex(r"---[^\n]*")]
    DocComment,

    // ── Special ─────────────────────────────────────────────
    Eof,
}

/// A positioned token with source span.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
    /// The raw source text for this token (needed for identifiers, literals).
    pub text: SmolStr,
}
```

### Lexer State Machine

The lexer operates in three modes:

1. **Normal mode** -- standard token recognition via `logos`.
2. **String mode** -- entered on `"`, emits `StringStart`, then scans for `${` (interpolation start), escape sequences, and the closing `"`. Characters between interpolation boundaries are emitted as `StringContent`.
3. **Indentation mode** -- after every `Newline`, the lexer measures leading whitespace and compares to an indentation stack. It emits zero or more `Indent`/`Dedent` synthetic tokens accordingly.

```rust
// japl-lexer/src/lib.rs

pub struct Lexer<'src> {
    source: &'src str,
    inner: logos::Lexer<'src, Token>,
    indent_stack: Vec<u32>,       // stack of indentation levels
    pending: VecDeque<SpannedToken>, // buffered synthetic tokens
    mode: LexerMode,
    paren_depth: u32,             // suppress indent tracking inside parens/brackets/braces
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    Normal,
    String { interpolation_depth: u32 },
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file_id: FileId) -> Self { /* ... */ }

    /// Advance to the next token. Returns None at EOF.
    pub fn next_token(&mut self) -> SpannedToken { /* ... */ }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = SpannedToken;
    fn next(&mut self) -> Option<Self::Item> { /* ... */ }
}
```

**Key design decisions:**
- Inside parentheses, brackets, and braces, newlines and indentation are insignificant. The lexer tracks nesting depth and suppresses `Indent`/`Dedent` emission when `paren_depth > 0`.
- String interpolation uses `${}` syntax. The lexer enters string mode on `"` and pushes interpolation contexts on `${`, popping on `}`.
- The indent stack starts with `[0]`. A newline followed by more spaces than the current top pushes a new level and emits `Indent`. Fewer spaces pops levels and emits one `Dedent` per popped level. Equal spaces emit nothing extra.

### Public API

```rust
// japl-lexer/src/lib.rs

/// Lex an entire source file into a Vec<SpannedToken>.
/// Used for testing and the formatter; the parser calls next_token() lazily.
pub fn lex_all(source: &str, file_id: FileId) -> (Vec<SpannedToken>, Vec<Diagnostic>) {
    /* ... */
}
```

### Crate Dependencies

| Dependency | Purpose |
|---|---|
| `logos` 0.14 | DFA-based token recognition |
| `smol_str` | Interned, small-string-optimized identifiers |
| `japl-ast` | `Span`, `FileId`, `Diagnostic` types |

---

## 3. Parser Design

**Crate:** `japl-parser`

### Parsing Strategy: Pratt + Recursive Descent Hybrid

**Recommendation: Pratt parsing for expressions, recursive descent for declarations and statements.**

Rationale:
- Pratt parsing handles operator precedence and associativity elegantly, producing correct parse trees for complex expressions without a precedence table in the grammar.
- Recursive descent is natural for JAPL's declaration-oriented top-level (modules, types, functions, traits, impls) which are not expression-shaped.
- This hybrid is the approach used by `rustc`, `clang`, and `rust-analyzer`, and is well-understood.

### Precedence Table

Operators are parsed by the Pratt parser with these binding powers (higher = tighter):

| Precedence | Operators | Associativity | Description |
|---|---|---|---|
| 1 | `\|>` | Left | Pipeline |
| 2 | `>>` | Right | Function composition |
| 3 | `\|\|` | Left | Logical OR |
| 4 | `&&` | Left | Logical AND |
| 5 | `==` `!=` | None | Equality (non-associative) |
| 6 | `<` `>` `<=` `>=` | None | Comparison (non-associative) |
| 7 | `++` `<>` | Right | Concatenation / Append |
| 8 | `+` `-` | Left | Additive |
| 9 | `*` `/` `%` | Left | Multiplicative |
| 10 | `-` `!` | — | Unary prefix |
| 11 | `?` | — | Postfix error propagation |
| 12 | `.` function application | Left | Field access, function call |

### AST Node Types

```rust
// japl-ast/src/ast.rs

use crate::{Span, SmolStr};

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

// ── Top-level Items ─────────────────────────────────────────

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

#[derive(Debug)]
pub struct ModuleDecl {
    pub name: QualifiedName,
    pub span: Span,
}

#[derive(Debug)]
pub struct ImportDecl {
    pub path: QualifiedName,
    /// Specific items to import, or None for the whole module.
    pub items: Option<Vec<ImportItem>>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ImportItem {
    Name(SmolStr),
    Type(SmolStr),
}

#[derive(Debug)]
pub struct QualifiedName {
    pub segments: Vec<SmolStr>,
    pub span: Span,
}

// ── Function Definition ─────────────────────────────────────

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

#[derive(Debug)]
pub struct Param {
    pub id: NodeId,
    pub pattern: Pattern,
    pub ty: Option<TypeExpr>,
    pub ownership: Ownership,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Value,   // default: immutable value
    Own,     // `own`: linear ownership
    Ref,     // `ref`: borrowed reference
}

#[derive(Debug)]
pub struct TypeParam {
    pub name: SmolStr,
    pub bounds: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct Constraint {
    pub trait_name: QualifiedName,
    pub type_args: Vec<TypeExpr>,
    pub span: Span,
}

// ── Type Definitions ────────────────────────────────────────

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

#[derive(Debug)]
pub enum TypeBody {
    /// Sum type: `| Variant1(T) | Variant2(T, T) | Variant3`
    Sum(Vec<Variant>),
    /// Record type: `{ field1: T, field2: T }`
    Record(Vec<FieldDef>),
    /// Capability type: `capability { ... }`
    Capability(Vec<CapabilityMethod>),
}

#[derive(Debug)]
pub struct Variant {
    pub name: SmolStr,
    pub fields: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug)]
pub struct FieldDef {
    pub name: SmolStr,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug)]
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

// ── Trait and Impl ──────────────────────────────────────────

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

// ── Module and Signature ────────────────────────────────────

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

#[derive(Debug)]
pub enum SignatureItem {
    TypeDecl { name: SmolStr, span: Span },
    FnDecl(FnSignature),
}

#[derive(Debug)]
pub struct FnSignature {
    pub name: SmolStr,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<TypeExpr>,
    pub return_type: TypeExpr,
    pub effects: Vec<EffectExpr>,
    pub span: Span,
}

// ── Foreign ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct ForeignBlock {
    pub abi: SmolStr,  // "C", "wasm"
    pub items: Vec<ForeignItem>,
    pub span: Span,
}

#[derive(Debug)]
pub enum ForeignItem {
    Fn(FnSignature),
    Module { name: SmolStr, items: Vec<FnSignature>, span: Span },
}

// ── Test / Property / Bench ─────────────────────────────────

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

// ── Supervisor ──────────────────────────────────────────────

#[derive(Debug)]
pub struct SupervisorDef {
    pub id: NodeId,
    pub name: SmolStr,
    pub strategy: Expr,
    pub children: Vec<Expr>,
    pub span: Span,
}

// ── Expressions ─────────────────────────────────────────────

#[derive(Debug)]
pub enum Expr {
    /// Integer literal: `42`, `0xFF`
    IntLit { value: SmolStr, span: Span },

    /// Float literal: `3.14`
    FloatLit { value: SmolStr, span: Span },

    /// String literal (possibly with interpolation segments)
    StringLit { segments: Vec<StringSegment>, span: Span },

    /// Character literal: `'x'`
    CharLit { value: char, span: Span },

    /// Boolean literal
    BoolLit { value: bool, span: Span },

    /// Unit literal: `()`
    UnitLit { span: Span },

    /// Variable reference: `x`, `foo_bar`
    Var { name: SmolStr, id: NodeId, span: Span },

    /// Constructor reference: `Some`, `Ok`
    Constructor { name: QualifiedName, id: NodeId, span: Span },

    /// Field access: `expr.field`
    FieldAccess { expr: Box<Expr>, field: SmolStr, span: Span },

    /// Function application: `f(x, y)`
    App { func: Box<Expr>, args: Vec<Expr>, span: Span },

    /// Binary operation: `a + b`
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },

    /// Unary operation: `-x`, `!b`
    UnaryOp { op: UnaryOp, expr: Box<Expr>, span: Span },

    /// Error propagation: `expr?`
    Try { expr: Box<Expr>, span: Span },

    /// Pipeline: `x |> f` desugars to `f(x)`
    Pipeline { lhs: Box<Expr>, rhs: Box<Expr>, span: Span },

    /// Function composition: `f >> g`
    Compose { lhs: Box<Expr>, rhs: Box<Expr>, span: Span },

    /// Lambda: `fn x -> x + 1` or `fn(x, y) -> x + y`
    Lambda { params: Vec<Param>, body: Box<Expr>, span: Span },

    /// Let binding: `let x = e1 in e2` (e2 is the rest of the block)
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
    RecordLit { fields: Vec<(SmolStr, Expr)>, span: Span },

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
    Receive { arms: Vec<MatchArm>, timeout: Option<Box<Expr>>, span: Span },

    /// Type annotation: `(expr : Type)`
    Annotation { expr: Box<Expr>, ty: TypeExpr, span: Span },
}

#[derive(Debug)]
pub enum StringSegment {
    Literal(SmolStr),
    Interpolation(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, LtEq, GtEq,
    And, Or,
    Concat, Append,   // ++ and <>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,  // -
    Not,  // !
}

// ── Patterns ────────────────────────────────────────────────

#[derive(Debug)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard { span: Span },

    /// Variable binding: `x`
    Var { name: SmolStr, id: NodeId, span: Span },

    /// Pinned variable: `^x` (match against existing binding)
    Pin { name: SmolStr, id: NodeId, span: Span },

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
        rest: bool,  // `..` at end
        span: Span,
    },

    /// List pattern: `[x, y, ..rest]`
    List {
        elements: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
        span: Span,
    },

    /// Tuple pattern: `(a, b, c)`
    Tuple { elements: Vec<Pattern>, span: Span },

    /// Or pattern: `Some(1) | Some(2)`
    Or { patterns: Vec<Pattern>, span: Span },

    /// As pattern: `pattern as name`
    As { pattern: Box<Pattern>, name: SmolStr, id: NodeId, span: Span },
}

// ── Type Expressions ────────────────────────────────────────

#[derive(Debug)]
pub enum TypeExpr {
    /// Named type: `Int`, `List[a]`, `Result[a, e]`
    Named { name: QualifiedName, args: Vec<TypeExpr>, span: Span },

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
    Record { fields: Vec<FieldDef>, row_var: Option<SmolStr>, span: Span },

    /// Tuple type: `(Int, String, Bool)`
    Tuple { elements: Vec<TypeExpr>, span: Span },

    /// Owned type: `own Buffer`
    Owned { inner: Box<TypeExpr>, span: Span },

    /// Ref type: `ref Buffer`
    Borrowed { inner: Box<TypeExpr>, span: Span },

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

// ── Effect Expressions ──────────────────────────────────────

#[derive(Debug)]
pub enum EffectExpr {
    /// Named effect: `Io`, `Net`, `Process[Msg]`
    Named { name: QualifiedName, args: Vec<TypeExpr>, span: Span },
    /// Effect variable (for polymorphism): `e`
    Var { name: SmolStr, span: Span },
}

// ── Match Arms ──────────────────────────────────────────────

#[derive(Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
    pub span: Span,
}
```

### Error Recovery Strategy

The parser implements **synchronization-point recovery**:

1. When an error is encountered, the parser records a diagnostic.
2. It then advances tokens until it finds a **synchronization point**: a token that reliably starts a new construct (`fn`, `type`, `let`, `module`, `import`, `test`, `|`, `Dedent`, `Newline` at zero indentation).
3. An `ErrorNode` placeholder is inserted into the AST so downstream passes can skip it gracefully.
4. This allows reporting multiple errors per compilation without cascading.

For expressions, the Pratt parser naturally recovers: a missing operand produces an error node and parsing continues at the next operator or delimiter.

### Public API

```rust
// japl-parser/src/lib.rs

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: SpannedToken,
    peek: SpannedToken,
    diagnostics: DiagnosticSink,
    node_id_counter: u32,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str, file_id: FileId) -> Self { /* ... */ }

    /// Parse a complete source file.
    pub fn parse_file(mut self) -> (SourceFile, Vec<Diagnostic>) { /* ... */ }
}
```

### Crate Dependencies

| Dependency | Purpose |
|---|---|
| `japl-lexer` | Token stream |
| `japl-ast` | AST nodes, Span, Diagnostic, NodeId |
| `smol_str` | Interned strings |

---

## 4. Type Checker

**Crate:** `japl-checker`

This crate houses four sequential passes: name resolution, type checking, effect checking, and linearity checking. They share a common context and type representation.

### 4.1 Name Resolution

Before type checking, all identifiers must be resolved to their definitions.

```rust
// japl-checker/src/resolve.rs

/// A unique ID for every definition in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// What kind of thing a name refers to.
#[derive(Debug, Clone)]
pub enum Resolution {
    Local(DefId),              // let/fn parameter
    TopLevel(DefId),           // module-level function or type
    Constructor(DefId),        // variant constructor
    Module(DefId),
    Trait(DefId),
    TypeParam(DefId),
    EffectParam(DefId),
    Foreign(DefId),
    Builtin(BuiltinId),
}

/// Mapping from NodeId (AST) to Resolution (semantic).
pub type ResolutionMap = HashMap<NodeId, Resolution>;

/// Scope tree built during resolution.
pub struct Resolver {
    scopes: Vec<Scope>,
    resolutions: ResolutionMap,
    diagnostics: DiagnosticSink,
}

pub struct Scope {
    parent: Option<usize>,
    bindings: HashMap<SmolStr, Resolution>,
}

impl Resolver {
    pub fn resolve(file: &SourceFile) -> (ResolutionMap, Vec<Diagnostic>) { /* ... */ }
}
```

### 4.2 Type Representation

```rust
// japl-types/src/lib.rs

/// Interned type ID for fast comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// The core type representation, interned in a TypeInterner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive types
    Int,
    Float,
    Float32,
    Bool,
    Char,
    String,
    Bytes,
    Unit,
    Never,

    /// Type variable (unification variable or rigid)
    Var(TypeVar),

    /// Named type constructor applied to arguments: `List[Int]`
    App {
        constructor: DefId,
        args: Vec<TypeId>,
    },

    /// Function type: `(A, B) -> C with E1, E2`
    Fn {
        params: Vec<TypeId>,
        return_type: TypeId,
        effects: EffectRow,
    },

    /// Record type with row polymorphism
    Record {
        fields: Vec<(SmolStr, TypeId)>,
        /// None = closed record; Some(var) = open record with row variable
        row_var: Option<TypeVar>,
    },

    /// Tuple type
    Tuple(Vec<TypeId>),

    /// Owned resource type
    Owned(TypeId),

    /// Borrowed reference type
    Ref(TypeId),

    /// Forall (polymorphic type scheme)
    Forall {
        vars: Vec<TypeVar>,
        constraints: Vec<TraitConstraint>,
        body: TypeId,
    },

    /// Type error placeholder (allows continued checking)
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar {
    pub id: u32,
    pub kind: TypeVarKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeVarKind {
    /// Unification variable: can be solved
    Unification,
    /// Rigid variable: bound by forall, cannot be solved
    Rigid,
    /// Skolem variable: created during subsumption checks
    Skolem,
}

/// A row of effects, modeled as a set with an optional row variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectRow {
    /// Known effects in the row
    pub effects: Vec<Effect>,
    /// Optional tail variable for open effect rows
    pub row_var: Option<TypeVar>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Io,
    Async,
    Net,
    State(TypeId),
    Process(TypeId),
    Fail(TypeId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitConstraint {
    pub trait_id: DefId,
    pub args: Vec<TypeId>,
}

/// Type interner: deduplicates types and provides O(1) equality.
pub struct TypeInterner {
    types: Vec<Type>,
    map: HashMap<Type, TypeId>,
}

impl TypeInterner {
    pub fn intern(&mut self, ty: Type) -> TypeId { /* ... */ }
    pub fn resolve(&self, id: TypeId) -> &Type { /* ... */ }
}
```

### 4.3 Bidirectional Type Checking Algorithm

The type checker uses **bidirectional type checking** with three judgment modes:

1. **Check** `check(expr, expected_type) -> ()` -- push an expected type down into an expression.
2. **Infer** `infer(expr) -> TypeId` -- synthesize a type for an expression.
3. **Subsumption** `subsumes(inferred, expected) -> ()` -- verify that an inferred type is at least as general as expected (handles polymorphism).

```rust
// japl-checker/src/typecheck.rs

pub struct TypeChecker {
    interner: TypeInterner,
    substitution: Substitution,   // unification variable bindings
    resolutions: ResolutionMap,
    env: TypeEnv,                 // binding types in scope
    trait_env: TraitEnv,          // trait implementations
    diagnostics: DiagnosticSink,
    next_var: u32,
}

/// Unification substitution: maps TypeVars to TypeIds.
pub struct Substitution {
    bindings: HashMap<TypeVar, TypeId>,
}

/// Type environment: maps DefIds to their type schemes.
pub struct TypeEnv {
    bindings: HashMap<DefId, TypeId>,
}

impl TypeChecker {
    /// Infer the type of an expression.
    pub fn infer(&mut self, expr: &Expr) -> TypeId { /* ... */ }

    /// Check an expression against an expected type.
    pub fn check(&mut self, expr: &Expr, expected: TypeId) { /* ... */ }

    /// Unify two types, updating the substitution.
    pub fn unify(&mut self, a: TypeId, b: TypeId, span: Span) { /* ... */ }

    /// Instantiate a forall type with fresh unification variables.
    pub fn instantiate(&mut self, ty: TypeId) -> TypeId { /* ... */ }

    /// Generalize a type by closing over free unification variables.
    pub fn generalize(&mut self, ty: TypeId) -> TypeId { /* ... */ }
}
```

### 4.4 Unification Algorithm

Standard union-find unification extended for JAPL-specific features:

```
unify(T, T)                      = ok
unify(Var(a), T)                 = bind a -> T  (occurs check)
unify(T, Var(a))                 = bind a -> T  (occurs check)
unify(App(C, As), App(C, Bs))   = unify pairwise As, Bs
unify(Fn(As, R1, E1), Fn(Bs, R2, E2)) =
    unify pairwise As, Bs; unify R1, R2; unify_effects E1, E2
unify(Record{fs1, r1}, Record{fs2, r2}) = row unification (see below)
unify(Tuple(As), Tuple(Bs))     = unify pairwise
unify(Owned(A), Owned(B))       = unify A, B
unify(Ref(A), Ref(B))           = unify A, B
unify(_, _)                      = type error
```

### 4.5 Row Polymorphism Unification

Row unification handles structural record types:

```
unify_row(Record{fs1, None}, Record{fs2, None}) =
    -- closed rows: fields must match exactly
    for each field in fs1 ∪ fs2:
        if missing in either side -> error
        else unify field types

unify_row(Record{fs1, Some(r)}, Record{fs2, tail}) =
    -- open row: match common fields, bind row variable to remainder
    let common = fs1 ∩ fs2 (by name)
    for each (name, t1, t2) in common: unify(t1, t2)
    let only_in_2 = fs2 \ common
    let new_tail = fresh_row_var()
    bind r -> Record{only_in_2, new_tail}
    unify_tail(new_tail, tail)
```

### 4.6 Trait / Typeclass Resolution

```rust
// japl-checker/src/traits.rs

pub struct TraitEnv {
    /// All known trait definitions.
    traits: HashMap<DefId, TraitInfo>,
    /// All known implementations.
    impls: Vec<ImplInfo>,
}

pub struct TraitInfo {
    pub id: DefId,
    pub params: Vec<TypeVar>,
    pub supertraits: Vec<TraitConstraint>,
    pub methods: HashMap<SmolStr, TypeId>,
}

pub struct ImplInfo {
    pub trait_id: DefId,
    pub type_args: Vec<TypeId>,
    pub methods: HashMap<SmolStr, DefId>,
}

impl TraitEnv {
    /// Resolve a trait constraint: find an impl that satisfies it.
    /// Uses backtracking search over impls with unification.
    pub fn resolve(
        &self,
        constraint: &TraitConstraint,
        interner: &TypeInterner,
        subst: &mut Substitution,
    ) -> Result<&ImplInfo, TraitError> { /* ... */ }
}
```

Resolution strategy:
1. For each `where Trait[Args]` constraint, search all `impl` blocks.
2. Attempt to unify the impl's type arguments with the constraint's type arguments.
3. If exactly one impl matches, use it. If zero match, report "no implementation found." If multiple match, report ambiguity.
4. Derived instances (`deriving`) are synthesized during an earlier pass and added to the impl list.

### 4.7 Effect Type Checking

Effects are checked as part of the function type, using effect row unification:

```rust
// japl-checker/src/effects.rs

impl TypeChecker {
    /// Check that calling a function with effect row `callee_effects`
    /// is legal in a context that allows `context_effects`.
    pub fn check_effects(
        &mut self,
        callee_effects: &EffectRow,
        context_effects: &EffectRow,
        span: Span,
    ) {
        // For each concrete effect in callee_effects:
        //   - it must appear in context_effects, OR
        //   - context_effects has an open row variable that can absorb it
        // Pure is always allowed.
    }

    /// Unify two effect rows.
    pub fn unify_effects(&mut self, a: &EffectRow, b: &EffectRow, span: Span) {
        // Similar to row unification for records:
        // match concrete effects, bind row variables to remainders.
    }
}
```

Effect checking rules:
- A `Pure` function cannot call an `Io` function.
- A function with effects `Io, Net` can call functions with `Io`, `Net`, or `Pure`.
- Effect handlers (`State.run`, `Fail.catch`) discharge an effect, removing it from the row.
- Within a process body, the `Process[Msg]` effect is available.

### 4.8 Error Message Design

Type errors are rendered with:
- The expected type and the inferred type, both pretty-printed.
- The source span where the mismatch occurred.
- If unification failed deep inside a structure, a "type diff" showing exactly which part diverged.
- Suggestions for common mistakes (e.g., "did you mean to use `?` to propagate this error?").

```
error[E0012]: type mismatch
  --> src/main.japl:23:10
   |
23 |   let x: Int = parse_config(path)
   |          ^^^   ^^^^^^^^^^^^^^^^^^
   |          |     this has type Result[Config, ParseError]
   |          expected Int
   |
   = note: Result[Config, ParseError] ≠ Int
   = help: use `?` to propagate the error: parse_config(path)?
```

### Crate Dependencies

| Dependency | Purpose |
|---|---|
| `japl-ast` | AST, NodeId, Span, Diagnostic |
| `japl-types` | Type, TypeId, TypeInterner, EffectRow |
| `la-arena` | Typed arena allocation for environments |
| `rustc-hash` | Fast hash maps for type interner |

---

## 5. Linearity Checker

**Crate:** `japl-checker` (submodule `linearity`)

The linearity checker runs after type checking. It verifies that all `Owned` and `use`-bound resources are consumed exactly once.

### 5.1 Context Splitting Algorithm

The checker maintains a **linear context** that tracks the status of each resource-typed binding:

```rust
// japl-checker/src/linearity.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceStatus {
    /// Available for use
    Available,
    /// Has been consumed (moved or closed)
    Consumed { at: Span },
    /// Has been borrowed (temporarily unavailable)
    Borrowed { at: Span, count: u32 },
}

pub struct LinearContext {
    /// Maps DefId of resource bindings to their status.
    resources: HashMap<DefId, (TypeId, ResourceStatus)>,
}

pub struct LinearityChecker {
    context: LinearContext,
    diagnostics: DiagnosticSink,
}
```

### 5.2 Checking Rules

```
check_expr(Let { pattern, value, body }) =
    if type_of(value) is Owned(T):
        ERROR: use `use` for resource bindings, not `let`
    check_expr(value)
    check_expr(body)

check_expr(Use { pattern, value, body }) =
    check_expr(value)
    add resource bindings to linear context as Available
    check_expr(body)
    for each resource binding still Available:
        ERROR: resource `x` was not consumed. Add a close/free call.

check_expr(Var { name }) =
    if name is in linear context:
        if status == Consumed:
            ERROR: use of moved resource `x` (moved at <span>)
        if ownership == Own:
            mark as Consumed at current span
        if ownership == Ref:
            mark as Borrowed (increment count)

check_expr(App { func, args }) =
    for each arg:
        if param is `own`: mark resource as Consumed
        if param is `ref`: mark resource as Borrowed
    check function body with split context

check_expr(If { cond, then, else }) =
    check cond
    // Branch: both branches must consume the same set of resources
    let ctx_then = context.clone()
    check then with ctx_then
    let ctx_else = context.clone()
    check else with ctx_else
    merge(ctx_then, ctx_else):
        for each resource: status must be the same in both branches
        if one branch consumes and the other doesn't: ERROR

check_expr(Match { scrutinee, arms }) =
    // similar to if: all arms must agree on resource consumption
```

### 5.3 Ownership Verification

The checker ensures:
1. **No double-consume:** A resource is used at most once in an ownership position.
2. **No use-after-move:** After a resource is consumed, it cannot be referenced.
3. **Must-consume:** Every `use`-bound resource must be consumed by the end of its scope.
4. **Borrow validity:** A borrowed reference cannot outlive the original resource's scope.
5. **No borrow-during-consume:** A resource cannot be borrowed if it has been or will be consumed in the same expression.

### 5.4 Region Inference for Borrows

Borrow regions are inferred rather than annotated. The checker computes the region where a `ref` parameter is live and verifies that the original resource is not consumed during that region.

```rust
/// A borrow region is a range of NodeIds in the AST.
pub struct BorrowRegion {
    pub resource: DefId,
    pub start: NodeId,
    pub end: NodeId,
}

impl LinearityChecker {
    /// Infer borrow regions and check for conflicts.
    pub fn check_borrows(&mut self, body: &Expr) -> Vec<BorrowRegion> { /* ... */ }
}
```

---

## 6. IR Design

**Crate:** `japl-ir`

### 6.1 Mid-Level IR (MIR)

The MIR is a lowered representation between the typed AST and machine code. It makes control flow explicit, eliminates pattern matching via decision trees, and makes closures explicit.

```rust
// japl-ir/src/mir.rs

/// A MIR function.
#[derive(Debug)]
pub struct MirFunction {
    pub id: DefId,
    pub name: SmolStr,
    pub params: Vec<MirLocal>,
    pub return_ty: TypeId,
    pub effects: EffectRow,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug)]
pub struct MirLocal {
    pub id: LocalId,
    pub ty: TypeId,
    pub name: Option<SmolStr>,  // for debug info
}

/// A basic block: a sequence of statements ending in a terminator.
#[derive(Debug)]
pub struct BasicBlock {
    pub id: BlockId,
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,
}

/// A MIR statement (no control flow).
#[derive(Debug)]
pub enum Statement {
    /// local = rvalue
    Assign { dst: LocalId, rvalue: RValue },

    /// Drop a resource (deterministic destructor call)
    Drop { local: LocalId },

    /// No operation (placeholder)
    Nop,
}

/// A right-hand side value.
#[derive(Debug)]
pub enum RValue {
    /// Use a local variable
    Use(Operand),

    /// Binary operation
    BinOp { op: BinOp, lhs: Operand, rhs: Operand },

    /// Unary operation
    UnaryOp { op: UnaryOp, operand: Operand },

    /// Construct an ADT variant: `Some(x)`
    Aggregate {
        kind: AggregateKind,
        fields: Vec<Operand>,
    },

    /// Read a field from a record or tuple
    FieldAccess { base: Operand, field: FieldIndex },

    /// Construct a closure: captures + function pointer
    MakeClosure {
        func: DefId,
        captures: Vec<Operand>,
    },

    /// Allocate on the immutable heap (GC-managed)
    HeapAlloc { ty: TypeId, value: Operand },

    /// Allocate on the resource arena (ownership-managed)
    ResourceAlloc { ty: TypeId },

    /// Cast between numeric types
    Cast { operand: Operand, target_ty: TypeId },

    /// Literal value
    Literal(Literal),
}

#[derive(Debug)]
pub enum AggregateKind {
    Variant { type_id: DefId, variant_index: u32 },
    Record { type_id: DefId },
    Tuple,
    List,
}

#[derive(Debug)]
pub enum FieldIndex {
    Named(SmolStr),
    Positional(u32),
}

#[derive(Debug)]
pub enum Operand {
    Local(LocalId),
    Constant(Literal),
}

#[derive(Debug)]
pub enum Literal {
    Int(i64),       // will be BigInt for arbitrary precision
    Float(f64),
    Float32(f32),
    Bool(bool),
    Char(char),
    String(SmolStr),
    Bytes(Vec<u8>),
    Unit,
}

/// A block terminator (control flow).
#[derive(Debug)]
pub enum Terminator {
    /// Return from function
    Return(Operand),

    /// Unconditional jump
    Goto(BlockId),

    /// Conditional branch
    Branch {
        condition: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Multi-way branch (compiled pattern match)
    Switch {
        discriminant: Operand,
        targets: Vec<(SwitchValue, BlockId)>,
        default: BlockId,
    },

    /// Function call
    Call {
        func: Operand,
        args: Vec<Operand>,
        destination: LocalId,
        continuation: BlockId, // block to jump to after call returns
    },

    /// Tail call (guaranteed no stack growth)
    TailCall {
        func: Operand,
        args: Vec<Operand>,
    },

    /// Process send
    Send {
        target: Operand,
        message: Operand,
        continuation: BlockId,
    },

    /// Process receive (blocks until message)
    Receive {
        targets: Vec<(PatternPredicate, BlockId)>,
        timeout: Option<(Operand, BlockId)>,
    },

    /// Process spawn
    Spawn {
        func: Operand,
        destination: LocalId,
        continuation: BlockId,
    },

    /// Unreachable (after Never-typed expressions)
    Unreachable,

    /// Crash the process with a reason
    Crash(Operand),
}

#[derive(Debug)]
pub enum SwitchValue {
    Int(i64),
    Bool(bool),
    Tag(u32),       // variant discriminant tag
    String(SmolStr),
}

#[derive(Debug)]
pub struct PatternPredicate {
    pub tag: Option<u32>,
    pub bindings: Vec<(LocalId, FieldIndex)>,
}
```

### 6.2 Optimization Passes

Passes operate on `MirFunction` and are run in this order:

| Pass | Description | Key technique |
|---|---|---|
| **Monomorphization** | Specialize generic functions for each concrete type combination. Generate a new `MirFunction` for each instantiation. Eliminates virtual dispatch and enables further optimization. | Type substitution + worklist of used instantiations |
| **Closure Conversion** | Convert closures to `{fn_ptr, env_ptr}` pairs. Captured variables are packed into a heap-allocated environment struct. | Lambda lifting for non-capturing closures; environment structs for capturing ones |
| **Pattern Match Compilation** | Convert `Switch` terminators with complex patterns into optimal decision trees (Maranget's algorithm). | Necessity heuristic for column selection, produces `Branch`/`Switch` chains |
| **Inlining** | Inline small functions (body size < threshold) and always-inline builtins. Controlled by a cost model. | Copy-and-rename with substitution |
| **Constant Folding** | Evaluate operations on known constants at compile time. | Interpret `BinOp`/`UnaryOp` on `Literal` operands |
| **Dead Code Elimination** | Remove unreachable blocks and unused assignments. Effect-free pure code with unused results is eliminated. | Liveness analysis via reverse dataflow |
| **Tail Call Optimization** | Convert self-recursive calls in tail position to `TailCall` terminators, which lower to jumps. | Detect call-then-return pattern |
| **Common Subexpression Elimination** | Deduplicate identical computations within a block. | Value numbering |
| **Effect-Guided DCE** | Pure functions whose results are unused can be eliminated entirely. Functions with effects cannot. | Check effect row of called function |

```rust
// japl-ir/src/optimize.rs

pub trait MirPass {
    fn name(&self) -> &str;
    fn run(&self, func: &mut MirFunction, interner: &TypeInterner);
}

pub struct OptimizationPipeline {
    passes: Vec<Box<dyn MirPass>>,
}

impl OptimizationPipeline {
    pub fn default_pipeline() -> Self {
        Self {
            passes: vec![
                Box::new(MonomorphizationPass),
                Box::new(ClosureConversionPass),
                Box::new(PatternMatchCompilationPass),
                Box::new(InliningPass { max_cost: 50 }),
                Box::new(ConstantFoldingPass),
                Box::new(DeadCodeEliminationPass),
                Box::new(TailCallOptimizationPass),
                Box::new(CommonSubexpressionEliminationPass),
            ],
        }
    }

    pub fn run(&self, func: &mut MirFunction, interner: &TypeInterner) {
        for pass in &self.passes {
            pass.run(func, interner);
        }
    }
}
```

### Crate Dependencies

| Dependency | Purpose |
|---|---|
| `japl-ast` | DefId, Span |
| `japl-types` | TypeId, TypeInterner, EffectRow |
| `rustc-hash` | Fast hash maps |

---

## 7. Code Generation

**Crate:** `japl-codegen`

### 7.1 Dual Backend Strategy

| Mode | Backend | Use case |
|---|---|---|
| **Dev** (`japl build`) | **Cranelift** | Fast compilation (~5x faster than LLVM), good-enough code quality for development iteration |
| **Release** (`japl build --release`) | **LLVM** (via `inkwell`) | Maximum optimization, production-quality native code |

Both backends consume the same optimized MIR.

### 7.2 Calling Convention

JAPL uses a custom calling convention that accommodates the process-based runtime:

```
JAPL Calling Convention:
  - Arguments passed in registers (following platform ABI: System V on x86_64/ARM64)
  - First argument (hidden): pointer to the current Process Context (scheduler handle,
    mailbox pointer, heap pointer, reduction counter)
  - Return value: in registers (small values) or via return pointer (large structs)
  - Stack frames include a "reduction check" point: after N function calls,
    the process yields to the scheduler
```

```rust
// japl-codegen/src/calling_convention.rs

/// Layout of the implicit process context pointer, passed as first argument.
pub struct ProcessContextLayout {
    pub scheduler_ptr: u64,     // offset 0: pointer to scheduler
    pub mailbox_ptr: u64,       // offset 8: pointer to process mailbox
    pub heap_ptr: u64,          // offset 16: pointer to process heap
    pub heap_limit: u64,        // offset 24: heap allocation limit
    pub reduction_counter: u64, // offset 32: remaining reductions before yield
    pub process_id: u64,        // offset 40: unique process identifier
}
```

### 7.3 Closure Representation

After closure conversion, a closure is a pair:

```
Closure = { fn_ptr: *const (), env_ptr: *const ClosureEnv }

ClosureEnv (heap-allocated) = {
    field_0: CapturedValue0,
    field_1: CapturedValue1,
    ...
}
```

Non-capturing lambdas are optimized to bare function pointers (no allocation).

### 7.4 Tagged Union Representation

ADT variants use a tagged-pointer scheme:

```
Small enums (≤ 256 variants):
  +-------+----------------------------+
  | tag:u8 | payload (variant fields)  |
  +-------+----------------------------+

  Tag is the first byte. Payload follows, aligned.

Packed enums (packed keyword):
  Compiler chooses optimal tag width.
  No pointers, contiguous layout for cache efficiency.

Option optimization:
  Option[Ref[T]] uses null pointer for None (no tag needed).
  Option[Bool] uses a 2-byte representation.

Nested enums:
  Each level has its own tag. The compiler may flatten
  known patterns (e.g., Result[Option[T], E]).
```

```rust
// japl-codegen/src/layout.rs

pub struct TypeLayout {
    pub size: u32,
    pub alignment: u32,
    pub kind: LayoutKind,
}

pub enum LayoutKind {
    Primitive,
    Record { field_offsets: Vec<(SmolStr, u32)> },
    Variant {
        tag_size: u32,  // 1, 2, or 4 bytes
        variants: Vec<VariantLayout>,
    },
    Tuple { element_offsets: Vec<u32> },
    Closure,
    Boxed,  // heap-allocated, represented as pointer
}

pub struct VariantLayout {
    pub tag_value: u32,
    pub fields: Vec<u32>,  // field offsets within variant payload
    pub total_size: u32,
}
```

### 7.5 Process Stack Layout

Each JAPL process gets its own stack, allocated from the process scheduler:

```
Process Stack:
  +-----------------------+  <- stack top (grows downward)
  | Frame N               |
  |   locals              |
  |   saved registers     |
  |   return address      |
  +-----------------------+
  | Frame N-1             |
  |   ...                 |
  +-----------------------+
  | ...                   |
  +-----------------------+
  | Frame 0 (entry)       |
  |   process context ptr |
  +-----------------------+  <- stack bottom
  | Guard page            |  <- unmapped, catches overflow
  +-----------------------+

Initial stack size: 4 KB (segmented / growable)
Maximum stack size: 1 MB (configurable per process)
Growth strategy: double on overflow, copy frames
```

### 7.6 Backend Interface

```rust
// japl-codegen/src/lib.rs

pub trait CodegenBackend {
    /// Compile a single MIR function to machine code.
    fn compile_function(
        &mut self,
        func: &MirFunction,
        layouts: &LayoutCache,
    ) -> Result<CompiledFunction, CodegenError>;

    /// Finalize and emit an object file.
    fn finalize(self) -> Result<Vec<u8>, CodegenError>;
}

pub struct CraneliftBackend {
    module: cranelift_module::JITModule, // or ObjectModule
    ctx: cranelift_codegen::Context,
    layouts: LayoutCache,
}

pub struct LlvmBackend {
    context: inkwell::context::Context,
    module: inkwell::module::Module<'static>,
    builder: inkwell::builder::Builder<'static>,
    layouts: LayoutCache,
}

pub struct LayoutCache {
    layouts: HashMap<TypeId, TypeLayout>,
}

/// The driver orchestrates compilation and linking.
pub fn compile_module(
    mir: &[MirFunction],
    backend: &mut dyn CodegenBackend,
    layouts: &LayoutCache,
) -> Result<Vec<u8>, CodegenError> { /* ... */ }

/// Link object files into a final binary.
pub fn link(
    objects: &[PathBuf],
    runtime_lib: &Path,
    output: &Path,
    target: &Target,
) -> Result<(), LinkError> { /* ... */ }
```

### Crate Dependencies

| Dependency | Purpose |
|---|---|
| `japl-ir` | MirFunction, Operand, etc. |
| `japl-types` | TypeId, TypeInterner |
| `cranelift-codegen` | Dev-mode native code generation |
| `cranelift-module` | Module/object emission for Cranelift |
| `cranelift-frontend` | IR builder for Cranelift |
| `inkwell` | LLVM bindings for release-mode codegen |
| `target-lexicon` | Target triple handling for cross-compilation |
| `object` | Object file reading/writing |

---

## 8. Runtime System

**Crate:** `japl-runtime`

The runtime is a Rust library linked into every JAPL binary. It provides the process scheduler, garbage collector, mailbox implementation, and supervision tree.

### 8.1 Process Scheduler

**Architecture:** M:N threading with work-stealing.

```
┌──────────────────────────────────────────────────────────┐
│                   JAPL Runtime                            │
│                                                           │
│  ┌────────────┐  ┌────────────┐       ┌────────────┐    │
│  │ OS Thread 0│  │ OS Thread 1│  ...  │ OS Thread N│    │
│  │ (Worker)   │  │ (Worker)   │       │ (Worker)   │    │
│  │            │  │            │       │            │    │
│  │ Run Queue  │  │ Run Queue  │       │ Run Queue  │    │
│  │ [P1,P5,P8] │  │ [P2,P6]   │       │ [P3,P4,P7]│    │
│  └────────────┘  └────────────┘       └────────────┘    │
│         │               │                    │            │
│         └───────────────┼────────────────────┘            │
│                         │                                 │
│              Work Stealing: idle workers                  │
│              steal from busy workers' queues              │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐ │
│  │                 Timer Wheel                          │ │
│  │     (process timeouts, receive timeouts)             │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                           │
│  ┌─────────────────────────────────────────────────────┐ │
│  │                 I/O Reactor                          │ │
│  │     (epoll/kqueue, async file I/O)                   │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

```rust
// japl-runtime/src/scheduler.rs

pub struct Scheduler {
    workers: Vec<Worker>,
    global_queue: SegQueue<ProcessHandle>,  // crossbeam lock-free queue
    io_reactor: IoReactor,
    timer_wheel: TimerWheel,
    process_table: DashMap<ProcessId, ProcessInfo>,
}

pub struct Worker {
    id: usize,
    local_queue: VecDeque<ProcessHandle>,
    rng: SmallRng,  // for work-stealing victim selection
}

pub struct ProcessHandle {
    pub id: ProcessId,
    pub state: ProcessState,
    pub stack: ProcessStack,
    pub heap: ProcessHeap,
    pub mailbox: Mailbox,
    pub links: Vec<ProcessId>,
    pub monitors: Vec<(MonitorRef, ProcessId)>,
    pub reduction_budget: u32,
    pub supervisor: Option<ProcessId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Runnable,
    WaitingForMessage,
    WaitingForIo,
    Suspended,
    Exited(ExitReason),
}

#[derive(Debug, Clone)]
pub enum ExitReason {
    Normal,
    Crash(SmolStr),
    Killed,
    LinkedCrash(ProcessId),
}
```

**Scheduling policy:**
- Each process gets a **reduction budget** (default: 4000 reductions). A reduction is approximately one function call or one MIR basic block.
- When the budget expires, the process is preempted and placed back on the run queue.
- Workers execute processes from their local queue. When the local queue is empty, they attempt to steal from other workers (random victim selection).
- I/O-bound processes are parked on the I/O reactor (epoll on Linux, kqueue on macOS) and woken when I/O is ready.

**Crate:** `crossbeam` for lock-free queues, `mio` for the I/O reactor.

### 8.2 Per-Process Garbage Collector

Each process has its own heap, collected independently. No global stop-the-world.

```rust
// japl-runtime/src/gc.rs

pub struct ProcessHeap {
    nursery: Nursery,       // young generation (bump allocator)
    old_gen: OldGeneration,  // tenured objects
    stats: GcStats,
}

pub struct Nursery {
    base: *mut u8,
    ptr: *mut u8,    // bump pointer
    limit: *mut u8,
    size: usize,     // default: 256 KB
}

pub struct OldGeneration {
    blocks: Vec<Block>,
    free_list: FreeList,
}

pub struct GcStats {
    pub nursery_collections: u64,
    pub major_collections: u64,
    pub total_allocated: u64,
    pub current_live: u64,
}
```

**GC strategy: generational, copying collector for nursery, mark-compact for old gen.**

1. **Nursery (young generation):** Bump allocation. When full, live objects are copied to old gen (Cheney's algorithm). Dead objects are reclaimed in bulk. This is very fast for JAPL because most values are short-lived.

2. **Old generation:** Mark-compact. Triggered when old gen grows past a threshold. Since all data is immutable, there are **no write barriers** needed -- an object in old gen can only point to other old-gen objects or immutable values in shared heaps.

3. **Cross-process sharing:** Immutable values can be shared across process heaps via reference counting on the shared region. Since values are immutable, no synchronization is needed beyond the reference count (atomic increment/decrement).

4. **Process death:** When a process exits, its entire heap is freed in O(1) -- no tracing needed.

### 8.3 Resource Arena

```rust
// japl-runtime/src/resource.rs

pub struct ResourceArena {
    /// Active resources owned by this process.
    resources: Vec<ResourceEntry>,
    free_slots: Vec<usize>,
}

pub struct ResourceEntry {
    pub handle: RawResourceHandle,
    pub destructor: fn(RawResourceHandle),
    pub type_id: TypeId,
    pub status: ResourceStatus,
}

pub type RawResourceHandle = *mut ();

impl ResourceArena {
    /// Allocate a new resource slot.
    pub fn alloc(&mut self, handle: RawResourceHandle, destructor: fn(RawResourceHandle), type_id: TypeId) -> ResourceId { /* ... */ }

    /// Consume (destroy) a resource.
    pub fn consume(&mut self, id: ResourceId) { /* ... */ }

    /// On process death, destroy all remaining resources.
    pub fn destroy_all(&mut self) { /* ... */ }
}
```

### 8.4 Mailbox Implementation

Each process has a single mailbox, implemented as a lock-free MPSC (multi-producer, single-consumer) queue.

```rust
// japl-runtime/src/mailbox.rs

pub struct Mailbox {
    queue: SegQueue<Message>,      // crossbeam MPSC queue
    save_queue: VecDeque<Message>, // messages skipped by selective receive
}

pub struct Message {
    pub tag: u32,
    pub payload: Box<[u8]>,  // serialized JAPL value
}

impl Mailbox {
    /// Enqueue a message (called by sender, any thread).
    pub fn send(&self, msg: Message) { /* ... */ }

    /// Dequeue the next message (called by owning process only).
    pub fn receive(&mut self) -> Option<Message> { /* ... */ }

    /// Selective receive: find the first message matching a predicate.
    pub fn receive_matching(
        &mut self,
        predicate: impl Fn(&Message) -> bool,
    ) -> Option<Message> { /* ... */ }

    /// Current queue length.
    pub fn len(&self) -> usize { /* ... */ }
}
```

**Selective receive** works by scanning the queue, moving non-matching messages to the save queue, and returning the first match. On the next non-selective receive, the save queue is drained first.

### 8.5 Supervision Tree Runtime

```rust
// japl-runtime/src/supervisor.rs

pub struct Supervisor {
    pub id: ProcessId,
    pub strategy: RestartStrategy,
    pub max_restarts: u32,
    pub max_seconds: u32,
    pub children: Vec<ChildEntry>,
    pub restart_log: VecDeque<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartStrategy {
    OneForOne,
    AllForOne,
    RestForOne,
}

pub struct ChildEntry {
    pub id: SmolStr,
    pub pid: Option<ProcessId>,
    pub start_fn: fn() -> (),  // function pointer to child entry
    pub restart: RestartPolicy,
    pub shutdown: ShutdownPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Permanent,   // always restart
    Transient,   // restart only on abnormal exit
    Temporary,   // never restart
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownPolicy {
    Timeout(u32),  // milliseconds
    Brutal,        // immediate kill
}

impl Supervisor {
    /// Handle a child crash according to the restart strategy.
    pub fn handle_child_exit(
        &mut self,
        child_pid: ProcessId,
        reason: ExitReason,
        scheduler: &Scheduler,
    ) {
        // 1. Check restart intensity (max_restarts / max_seconds)
        // 2. Apply strategy (restart one, all, or rest)
        // 3. If intensity exceeded, crash self (propagate up tree)
    }
}
```

### 8.6 Node Mesh / Distribution Layer

```rust
// japl-runtime/src/distribution.rs

pub struct NodeMesh {
    pub local_node: NodeInfo,
    pub connections: DashMap<NodeId, NodeConnection>,
    pub registry: ProcessRegistry,
}

pub struct NodeInfo {
    pub name: SmolStr,
    pub cookie: [u8; 32],  // authentication cookie
    pub listen_addr: SocketAddr,
}

pub struct NodeConnection {
    pub node_id: NodeId,
    pub stream: TcpStream,
    pub serializer: Serializer,
}

pub struct ProcessRegistry {
    pub local: DashMap<ProcessId, ProcessHandle>,
    pub remote: DashMap<ProcessId, (NodeId, ProcessId)>,
}

impl NodeMesh {
    /// Send a message to a process (local or remote).
    pub fn send(&self, pid: ProcessId, msg: Message) -> Result<(), SendError> {
        if let Some(local) = self.registry.local.get(&pid) {
            local.mailbox.send(msg);
            Ok(())
        } else if let Some(remote) = self.registry.remote.get(&pid) {
            let conn = self.connections.get(&remote.0)?;
            conn.serializer.send_remote(remote.1, msg)?;
            Ok(())
        } else {
            Err(SendError::ProcessNotFound)
        }
    }

    /// Connect to a remote node.
    pub fn connect(&self, addr: SocketAddr) -> Result<NodeId, ConnectError> { /* ... */ }

    /// Spawn a process on a remote node.
    pub fn spawn_remote(
        &self,
        node_id: NodeId,
        func: SerializedClosure,
    ) -> Result<ProcessId, SpawnError> { /* ... */ }
}
```

**Wire protocol:** Binary, length-prefixed frames. Message serialization is derived from JAPL types (any type with `deriving(Serialize)` can cross node boundaries). The protocol includes:
- Handshake with cookie authentication
- Process spawn requests/responses
- Message delivery
- Link/monitor notifications
- Node heartbeats

### Runtime Crate Dependencies

| Dependency | Purpose |
|---|---|
| `crossbeam` | Lock-free queues, scoped threads |
| `mio` | I/O reactor (epoll/kqueue abstraction) |
| `dashmap` | Concurrent hash map for process tables |
| `parking_lot` | Fast mutexes/rwlocks where needed |
| `rand` | Work-stealing victim selection |
| `socket2` | Low-level socket operations for distribution |

---

## 9. Standard Library Architecture

**Crate:** `japl-stdlib`

The standard library is initially written in Rust (calling runtime APIs) and will be progressively rewritten in JAPL as the compiler matures.

### Module Hierarchy

```
Core                    -- built-in types and operators (compiler intrinsics)
  Int                   -- arbitrary-precision integer
  Float                 -- 64-bit float
  Float32               -- 32-bit float
  Bool                  -- boolean
  Char                  -- unicode scalar
  String                -- UTF-8 string
  Bytes                 -- raw byte sequences
  Option                -- Option[a] = Some(a) | None
  Result                -- Result[a, e] = Ok(a) | Err(e)
  Ordering              -- Lt | Eq | Gt
  Tuple                 -- tuple operations

Collections
  List                  -- persistent singly-linked list (+ efficient ops)
  Map                   -- persistent hash-array mapped trie (HAMT)
  Set                   -- persistent hash set (backed by Map)
  Array                 -- immutable random-access array (RRB-tree)
  Deque                 -- double-ended queue
  MutableArray          -- resource-layer, ownership-tracked mutable array

Io
  File                  -- file operations
  Path                  -- path manipulation
  Console               -- stdin/stdout/stderr
  Env                   -- environment variables
  Clock                 -- time and timers
  Random                -- random number generation

Process
  Process               -- spawn, send, receive, link, monitor
  Reply                 -- request-reply channel
  Supervisor            -- supervision trees
  Registry              -- process name registry
  Task                  -- structured concurrency (spawn + await pattern)

Net
  Tcp                   -- TCP sockets
  Udp                   -- UDP sockets
  Http                  -- HTTP client and server
  Tls                   -- TLS wrapping
  Dns                   -- DNS resolution
  WebSocket             -- WebSocket protocol

Data
  Json                  -- JSON encoding/decoding
  Csv                   -- CSV parsing
  Base64                -- base64 encoding
  Hex                   -- hexadecimal encoding
  Regex                 -- regular expressions

State
  State                 -- effect handler for State[s]
  Fail                  -- effect handler for Fail[e]

Node
  Node                  -- cluster membership, connect, disconnect
  Rpc                   -- remote procedure call helpers

Test
  Test                  -- test runner
  Assert                -- assertion functions
  Property              -- property-based testing (QuickCheck-style)
  Bench                 -- benchmarking harness

Format
  Debug                 -- debug representation (Show trait)
  Display               -- user-facing representation
```

### Implementation Strategy

Each stdlib module is a Rust module with `#[no_mangle] extern "C"` functions matching JAPL's calling convention. The compiler knows about these as builtins and generates direct calls to them.

```rust
// japl-stdlib/src/list.rs

/// List is a persistent singly-linked list backed by an Arc-based cons cell.
/// For performance-critical paths, backed by a chunked RRB vector.

#[repr(C)]
pub enum JaplList {
    Nil,
    Cons { head: JaplValue, tail: Arc<JaplList> },
}

#[no_mangle]
pub extern "C" fn japl_list_map(
    ctx: *mut ProcessContext,
    list: *const JaplList,
    func: *const JaplClosure,
) -> *const JaplList {
    // ...
}
```

---

## 10. Rust Project Structure

```
japl/
├── Cargo.toml                      # workspace root
├── compiler/
│   ├── Cargo.toml                  # workspace definition
│   └── crates/
│       ├── japl-ast/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # re-exports
│       │       ├── ast.rs          # AST node types
│       │       ├── diagnostic.rs   # Span, Diagnostic, DiagnosticSink
│       │       ├── source.rs       # FileId, SourceMap
│       │       └── visitor.rs      # AST visitor trait
│       │
│       ├── japl-lexer/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # Lexer struct, lex_all()
│       │       └── token.rs        # Token enum
│       │
│       ├── japl-parser/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # Parser struct, parse_file()
│       │       ├── expr.rs         # Pratt expression parser
│       │       ├── decl.rs         # Declaration parser (fn, type, trait, etc.)
│       │       ├── pattern.rs      # Pattern parser
│       │       └── types.rs        # Type expression parser
│       │
│       ├── japl-types/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # Type, TypeId, TypeInterner
│       │       ├── effect.rs       # Effect, EffectRow
│       │       └── display.rs      # Pretty-printing for types
│       │
│       ├── japl-checker/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # orchestrate all checking passes
│       │       ├── resolve.rs      # name resolution
│       │       ├── typecheck.rs    # bidirectional type checker
│       │       ├── unify.rs        # unification algorithm
│       │       ├── traits.rs       # trait resolution
│       │       ├── effects.rs      # effect checking
│       │       ├── linearity.rs    # linearity / ownership checking
│       │       └── infer.rs        # type inference helpers
│       │
│       ├── japl-ir/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # re-exports
│       │       ├── mir.rs          # MIR node types
│       │       ├── lower.rs        # AST -> MIR lowering
│       │       ├── optimize.rs     # optimization pass trait + pipeline
│       │       ├── passes/
│       │       │   ├── monomorphize.rs
│       │       │   ├── closure_convert.rs
│       │       │   ├── pattern_compile.rs
│       │       │   ├── inline.rs
│       │       │   ├── const_fold.rs
│       │       │   ├── dce.rs
│       │       │   ├── tco.rs
│       │       │   └── cse.rs
│       │       └── pretty.rs       # MIR pretty-printer (for debugging)
│       │
│       ├── japl-codegen/
│       │   ├── Cargo.toml
│       │   └── src/
│       │       ├── lib.rs          # CodegenBackend trait, compile_module(), link()
│       │       ├── layout.rs       # TypeLayout, LayoutCache
│       │       ├── cranelift.rs    # CraneliftBackend
│       │       ├── llvm.rs         # LlvmBackend
│       │       └── link.rs         # linker invocation
│       │
│       └── japl-driver/
│           ├── Cargo.toml
│           └── src/
│               ├── main.rs         # CLI entry point (clap)
│               ├── build.rs        # `japl build` command
│               ├── test_cmd.rs     # `japl test` command
│               ├── fmt.rs          # `japl fmt` command
│               ├── run.rs          # `japl run` command
│               └── project.rs      # project manifest (japl.toml) parsing
│
├── runtime/
│   ├── japl-runtime/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              # runtime initialization, entry point
│   │       ├── scheduler.rs        # M:N work-stealing scheduler
│   │       ├── process.rs          # ProcessHandle, ProcessState
│   │       ├── gc.rs               # per-process generational GC
│   │       ├── resource.rs         # ResourceArena
│   │       ├── mailbox.rs          # lock-free MPSC mailbox
│   │       ├── supervisor.rs       # supervision tree
│   │       ├── distribution.rs     # node mesh, remote messaging
│   │       ├── io_reactor.rs       # async I/O via mio
│   │       └── timer.rs            # timer wheel
│   │
│   └── japl-stdlib/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs              # stdlib registration
│           ├── core/               # Int, Float, Bool, String, etc.
│           ├── collections/        # List, Map, Set, Array
│           ├── io/                 # File, Path, Console, Env
│           ├── process/            # Process, Supervisor, Reply
│           ├── net/                # Tcp, Http, Tls
│           ├── data/               # Json, Csv, Regex
│           ├── state/              # State and Fail effect handlers
│           ├── node/               # Distribution
│           └── test/               # Test runner, property testing
│
└── tools/
    └── japl-fmt/
        ├── Cargo.toml
        └── src/
            ├── lib.rs              # formatting engine
            └── rules.rs            # formatting rules
```

### Workspace Cargo.toml

```toml
# japl/Cargo.toml
[workspace]
resolver = "2"
members = [
    "compiler/crates/japl-ast",
    "compiler/crates/japl-lexer",
    "compiler/crates/japl-parser",
    "compiler/crates/japl-types",
    "compiler/crates/japl-checker",
    "compiler/crates/japl-ir",
    "compiler/crates/japl-codegen",
    "compiler/crates/japl-driver",
    "runtime/japl-runtime",
    "runtime/japl-stdlib",
    "tools/japl-fmt",
]

[workspace.dependencies]
# Shared versions across workspace
smol_str = "0.3"
rustc-hash = "2.0"
la-arena = "0.3"
codespan-reporting = "0.11"
logos = "0.14"
cranelift-codegen = "0.108"
cranelift-frontend = "0.108"
cranelift-module = "0.108"
inkwell = { version = "0.5", features = ["llvm18-0"] }
target-lexicon = "0.12"
crossbeam = "0.8"
mio = { version = "1.0", features = ["os-poll", "net"] }
dashmap = "6.0"
parking_lot = "0.12"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

### Per-Crate Dependency Graph

```
japl-driver
  ├── japl-parser
  │   ├── japl-lexer
  │   │   └── japl-ast
  │   └── japl-ast
  ├── japl-checker
  │   ├── japl-ast
  │   └── japl-types
  │       └── japl-ast
  ├── japl-ir
  │   ├── japl-ast
  │   └── japl-types
  ├── japl-codegen
  │   ├── japl-ir
  │   └── japl-types
  ├── japl-runtime (linked into output binary, not the compiler itself)
  └── japl-stdlib  (linked into output binary)
```

---

## 11. Build System

### `japl build`

```
1. Read `japl.toml` project manifest.
2. Resolve dependencies (from registry or local paths).
3. Compute module dependency graph from imports.
4. For each module (in dependency order):
   a. Check file modification time against cached artifacts.
   b. If stale or new:
      - Lex → Parse → Resolve → TypeCheck → EffectCheck → LinearityCheck
      - Lower to MIR → Optimize → CodeGen
      - Emit object file to build cache (~/.japl/cache/<hash>/)
   c. If clean: reuse cached object file.
5. Link all object files + japl-runtime + japl-stdlib into final binary.
6. Output: single static binary at ./build/<project-name>
```

**Incremental compilation:** The build cache keys on (source file hash + dependency interface hashes). If a module's interface (exported type signatures) hasn't changed, downstream modules don't need recompilation even if the implementation changed.

**Parallelism:** Modules that don't depend on each other are compiled in parallel across all CPU cores.

### `japl test`

```
1. japl build (with test harness linked in).
2. Discover all `test` and `property` definitions across all modules.
3. Each test runs in its own JAPL process (isolated, crash-safe).
4. Property tests generate random inputs (configurable seed, iteration count).
5. Report: pass/fail counts, failure details with shrunk counterexamples.
6. Exit code: 0 if all pass, 1 if any fail.
```

### `japl fmt`

```
1. Parse source files (lexer + parser only; type checking not needed).
2. Apply formatting rules:
   - 2-space indentation
   - One blank line between top-level items
   - Trailing commas in multi-line constructs
   - Alignment of match arms
   - Sort imports alphabetically
   - Wrap lines at 100 characters
3. Overwrite source files in place (or `--check` mode for CI).
```

The formatter uses a Wadler-Lindig pretty-printing algorithm for optimal line-breaking decisions.

### `japl run`

```
1. japl build (dev mode, Cranelift backend).
2. Execute the resulting binary.
3. Forward stdin/stdout/stderr.
4. Exit with the binary's exit code.
```

### Cross-Compilation

```bash
# List available targets
japl targets

# Build for a specific target
japl build --target x86_64-unknown-linux-musl
japl build --target aarch64-unknown-linux-musl
japl build --target aarch64-apple-darwin
japl build --target wasm32-wasi

# Release build with LLVM backend
japl build --release --target x86_64-unknown-linux-musl
```

Cross-compilation is handled by:
1. Cranelift/LLVM targeting the specified `target-lexicon` triple.
2. The runtime library is pre-compiled for each supported target (or compiled from source during the build).
3. Static linking with `musl` libc on Linux produces fully self-contained binaries with zero external dependencies.

### Project Manifest (`japl.toml`)

```toml
[project]
name = "my-app"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]
http = "1.2.0"
json = "0.5.1"
postgres = { git = "https://github.com/japl-lang/postgres", tag = "v0.3.0" }

[dev-dependencies]
mock = "0.1.0"

[build]
# Number of parallel compilation jobs (default: num_cpus)
jobs = 8

[release]
# Targets to build for `japl release`
targets = [
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
]
# Enable LTO for release builds
lto = true
# Strip debug symbols
strip = true
```

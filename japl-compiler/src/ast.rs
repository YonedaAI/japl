#[derive(Debug, Clone)]
pub enum Type {
    Named(String),
    FnType(Vec<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Void,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    ByteLit(u8),
    Ident(String),
    Call(Box<Expr>, Vec<Expr>),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Block(Vec<Stmt>, Option<Box<Expr>>),
    Lambda(Vec<Param>, Option<Type>, Box<Expr>),
    Match(Box<Expr>, Vec<MatchArm>),
    Record(Vec<(String, Expr)>),
    FieldAccess(Box<Expr>, String),
    RecordUpdate(Box<Expr>, Vec<(String, Expr)>),
    Pipe(Box<Expr>, Box<Expr>),
    Receive(Vec<MatchArm>),
    Tuple(Vec<Expr>),
    TupleAccess(Box<Expr>, usize),
    UseExpr(String, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Variant(String, Vec<String>),
    Wildcard,
    IntLit(i64),
    StringLit(String),
    BoolLit(bool),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, LtEq, GtEq,
    Concat,
    And, Or,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr),
    LetTyped(String, Type, Expr),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    FnDef(FnDef),
    TypeDef(TypeDef),
    ForeignFn(ForeignFnDef),
    Import(ImportDef),
    Const(ConstDef),
    TraitDef(TraitDef),
    OpaqueType(OpaqueTypeDef),
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
    pub body: Expr,
    pub is_pub: bool,
    pub doc_comment: Option<String>,
    pub effect: Option<String>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct ForeignFnDef {
    pub module: String,
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct ImportDef {
    pub module_path: Vec<String>,
    pub names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConstDef {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub type_param: String,
    pub methods: Vec<TraitMethod>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Type,
}

#[derive(Debug, Clone)]
pub struct OpaqueTypeDef {
    pub name: String,
    pub inner: Type,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum IrExpr {
    I64Const(i64),
    StringConst(u32, u32), // offset, length in data segment
    BoolConst(bool),
    LocalGet(String),
    LocalSet(String, Box<IrExpr>),
    Call(String, Vec<IrExpr>),
    CallIndirect(Box<IrExpr>, Vec<IrExpr>), // closure call
    BinOp(IrBinOp, Box<IrExpr>, Box<IrExpr>),
    If(Box<IrExpr>, Box<IrExpr>, Option<Box<IrExpr>>),
    Block(Vec<IrStmt>, Option<Box<IrExpr>>),
    Loop(String, Vec<IrStmt>), // label, body (for TCO)
    Continue(String, Vec<(String, IrExpr)>), // label, param updates
    Break(String, Box<IrExpr>),              // break out of TCO loop with value
    // Tagged union construction: tag, fields
    TaggedNew(u32, Vec<IrExpr>),
    // Tagged union field access: expr, field_index (offset from tag header)
    TaggedGetField(Box<IrExpr>, u32),
    // Tagged union get tag
    TaggedGetTag(Box<IrExpr>),
    // Record construction: sorted fields (name, value)
    RecordNew(Vec<(String, IrExpr)>),
    // Record field access: expr, field_index (computed from sorted field names)
    RecordGetField(Box<IrExpr>, u32),
    // Closure creation: function table index, captured values
    ClosureNew(u32, Vec<IrExpr>),
    // Closure get captured var
    ClosureGetCapture(Box<IrExpr>, u32),
    // String concat
    StringConcat(Box<IrExpr>, Box<IrExpr>),
    // Drop result (for void statements)
    Drop(Box<IrExpr>),
    // Show builtin: int -> string
    ShowInt(Box<IrExpr>),
    ShowBool(Box<IrExpr>),
    // Process operations
    Spawn(Box<IrExpr>),         // spawn(closure_ptr) -> pid
    Send(Box<IrExpr>, Box<IrExpr>), // send(pid, msg_ptr)
    Receive,                     // receive() -> msg_ptr
    SelfPid,                     // self_pid() -> pid
}

#[derive(Debug, Clone)]
pub enum IrStmt {
    Let(String, IrExpr),
    Expr(IrExpr),
}

#[derive(Debug, Clone, Copy)]
pub enum IrBinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, LtEq, GtEq,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>,
    pub locals: Vec<String>,
    pub body: IrExpr,
    pub has_return: bool,
    pub is_closure_body: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmType {
    I32,
    I64,
}

#[derive(Debug, Clone)]
pub struct IrForeignImport {
    pub module: String,
    pub name: String,
    pub param_count: usize,
    pub has_return: bool,
    /// Actual WASM parameter types (if known from runtime signatures)
    pub param_types: Vec<WasmType>,
    /// Actual WASM return types (if known from runtime signatures)
    pub return_types: Vec<WasmType>,
}

#[derive(Debug, Clone)]
pub struct IrTypeInfo {
    pub name: String,
    pub variants: Vec<IrVariantInfo>,
}

#[derive(Debug, Clone)]
pub struct IrVariantInfo {
    pub name: String,
    pub tag: u32,
    pub field_count: usize,
}

#[derive(Debug, Clone)]
pub struct IrStringData {
    pub offset: u32,
    pub length: u32,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub foreign_imports: Vec<IrForeignImport>,
    pub types: Vec<IrTypeInfo>,
    pub string_data: Vec<IrStringData>,
    pub heap_start: u32,
    pub uses_processes: bool,
    pub constants: Vec<(String, i64)>,
}

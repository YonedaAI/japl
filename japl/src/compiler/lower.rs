use std::collections::{HashMap, HashSet};
use super::ast;
use super::ir::*;

/// Returns the known WASM signatures for japl runtime host functions.
/// (param_types, return_types)
fn runtime_signature(module: &str, name: &str) -> Option<(Vec<WasmType>, Vec<WasmType>)> {
    if module != "japl" {
        return None;
    }
    use WasmType::*;
    match name {
        "spawn"          => Some((vec![I64], vec![I64])),
        "send"           => Some((vec![I64, I64], vec![])),
        "receive"        => Some((vec![], vec![I64])),
        "self_pid"       => Some((vec![], vec![I64])),
        "println"        => Some((vec![I32, I32], vec![])),
        "llm"            => Some((vec![I32, I32], vec![I32, I32])),
        "time_now"       => Some((vec![], vec![I64])),
        "time_sleep"     => Some((vec![I64], vec![])),
        "tcp_listen"     => Some((vec![I32], vec![I64])),
        "tcp_accept"     => Some((vec![I64], vec![I64])),
        "tcp_connect"    => Some((vec![I32, I32, I32], vec![I64])),
        "tcp_read"       => Some((vec![I64, I32, I32], vec![I32])),
        "tcp_write"      => Some((vec![I64, I32, I32], vec![I32])),
        "tcp_close"      => Some((vec![I64], vec![])),
        "env_get"        => Some((vec![I32, I32], vec![I32, I32])),
        "env_args_count" => Some((vec![], vec![I32])),
        "crypto_sha256"  => Some((vec![I32, I32, I32], vec![])),
        "crypto_random"  => Some((vec![I32, I32], vec![])),
        "file_read_str"  => Some((vec![I32], vec![I32])),
        "file_read"      => Some((vec![I32, I32], vec![I32, I32])),
        "file_write"     => Some((vec![I32, I32, I32, I32], vec![I32])),
        "file_exists"    => Some((vec![I32, I32], vec![I32])),
        "bytes_alloc"    => Some((vec![I32], vec![I32])),
        "print_bytes"    => Some((vec![I32, I32], vec![])),
        "char_at"        => Some((vec![I32, I32], vec![I32])),
        "substring"      => Some((vec![I32, I32, I32], vec![I32])),
        "string_index_of"=> Some((vec![I32, I32], vec![I32])),
        "from_char_code" => Some((vec![I32], vec![I32])),
        "str_length"     => Some((vec![I32], vec![I32])),
        "string_eq"      => Some((vec![I32, I32], vec![I32])),
        "str_contains"   => Some((vec![I32, I32], vec![I32])),
        "str_starts_with"=> Some((vec![I32, I32], vec![I32])),
        "str_ends_with"  => Some((vec![I32, I32], vec![I32])),
        "str_trim"       => Some((vec![I32], vec![I32])),
        "str_to_upper"   => Some((vec![I32], vec![I32])),
        "str_to_lower"   => Some((vec![I32], vec![I32])),
        "str_replace"    => Some((vec![I32, I32, I32], vec![I32])),
        "str_index_of"   => Some((vec![I32, I32], vec![I32])),
        "str_parse_int"  => Some((vec![I32], vec![I32])),
        "bytes_new"      => Some((vec![I32], vec![I32])),
        "bytes_from_string" => Some((vec![I32], vec![I32])),
        "bytes_to_string"   => Some((vec![I32], vec![I32])),
        "bytes_length"   => Some((vec![I32], vec![I32])),
        "bytes_slice"    => Some((vec![I32, I32, I32], vec![I32])),
        "bytes_concat"   => Some((vec![I32, I32], vec![I32])),
        "bytes_get"      => Some((vec![I32, I32], vec![I32])),
        "bytes_set"      => Some((vec![I32, I32, I32], vec![I32])),
        _ => None,
    }
}

pub struct Lowerer {
    string_data: Vec<IrStringData>,
    next_string_offset: u32,
    functions: Vec<IrFunction>,
    foreign_imports: Vec<IrForeignImport>,
    types: Vec<IrTypeInfo>,
    // variant name -> (type_name, tag, field_count)
    variant_map: HashMap<String, (String, u32, usize)>,
    // For closures: table index
    next_table_index: u32,
    // closure body functions generated
    closure_funcs: Vec<IrFunction>,
    // current function name (for TCO detection)
    current_fn_name: Option<String>,
    current_fn_param_count: usize,
    current_fn_params: Vec<String>,
    // Track record field names per variable for field access resolution
    record_fields: HashMap<String, Vec<String>>,
    // Known top-level function names -> param count
    known_functions: HashSet<String>,
    known_function_arity: HashMap<String, usize>,
    // Known foreign function names
    known_foreign_fns: HashSet<String>,
    // Variables known to hold bool values
    bool_vars: HashSet<String>,
    // Whether the program uses process operations
    uses_processes: bool,
    // Constants
    constants: HashMap<String, i64>,
}

impl Lowerer {
    pub fn new() -> Self {
        Lowerer {
            string_data: Vec::new(),
            next_string_offset: 1024, // start after scratch space
            functions: Vec::new(),
            foreign_imports: Vec::new(),
            types: Vec::new(),
            variant_map: HashMap::new(),
            next_table_index: 0,
            closure_funcs: Vec::new(),
            current_fn_name: None,
            current_fn_param_count: 0,
            current_fn_params: Vec::new(),
            record_fields: HashMap::new(),
            known_functions: HashSet::new(),
            known_function_arity: HashMap::new(),
            known_foreign_fns: HashSet::new(),
            bool_vars: HashSet::new(),
            uses_processes: false,
            constants: HashMap::new(),
        }
    }

    fn intern_string(&mut self, s: &str) -> (u32, u32) {
        // Check if already interned
        for sd in &self.string_data {
            if sd.content == s {
                return (sd.offset, sd.length);
            }
        }
        let offset = self.next_string_offset;
        let len = s.len() as u32;
        // We store [4-byte len][UTF-8 bytes]
        let total = 4 + len;
        self.string_data.push(IrStringData {
            offset,
            length: len,
            content: s.to_string(),
        });
        self.next_string_offset = offset + total;
        // Align to 8 bytes
        if self.next_string_offset % 8 != 0 {
            self.next_string_offset += 8 - (self.next_string_offset % 8);
        }
        (offset, len)
    }

    pub fn lower_program(&mut self, program: &ast::Program) -> IrModule {
        // First pass: collect type definitions and constants
        for item in &program.items {
            match item {
                ast::TopLevel::TypeDef(td) => {
                    let mut variants = Vec::new();
                    for (i, v) in td.variants.iter().enumerate() {
                        let tag = i as u32;
                        variants.push(IrVariantInfo {
                            name: v.name.clone(),
                            tag,
                            field_count: v.fields.len(),
                        });
                        self.variant_map.insert(v.name.clone(), (td.name.clone(), tag, v.fields.len()));
                    }
                    self.types.push(IrTypeInfo {
                        name: td.name.clone(),
                        variants,
                    });
                }
                ast::TopLevel::Const(cd) => {
                    // Only support integer constants for now
                    if let ast::Expr::IntLit(n) = &cd.value {
                        self.constants.insert(cd.name.clone(), *n);
                    }
                }
                _ => {}
            }
        }

        // Second pass: collect all function/foreign names for call resolution
        for item in &program.items {
            match item {
                ast::TopLevel::FnDef(fd) => {
                    self.known_functions.insert(fd.name.clone());
                    self.known_function_arity.insert(fd.name.clone(), fd.params.len());
                }
                ast::TopLevel::ForeignFn(ff) => {
                    self.known_foreign_fns.insert(ff.name.clone());
                    let (param_types, return_types) = runtime_signature(&ff.module, &ff.name)
                        .unwrap_or_else(|| {
                            let pts: Vec<WasmType> = (0..ff.params.len()).map(|_| WasmType::I64).collect();
                            let rts = if ff.ret_ty.is_some() { vec![WasmType::I64] } else { vec![] };
                            (pts, rts)
                        });
                    self.foreign_imports.push(IrForeignImport {
                        module: ff.module.clone(),
                        name: ff.name.clone(),
                        param_count: param_types.len(),
                        has_return: !return_types.is_empty(),
                        param_types,
                        return_types,
                    });
                }
                _ => {}
            }
        }

        // Third pass: lower functions
        for item in &program.items {
            if let ast::TopLevel::FnDef(fd) = item {
                let func = self.lower_fn_def(fd);
                self.functions.push(func);
            }
        }

        // If processes are used, add process-related imports
        if self.uses_processes {
            let process_fns = [
                ("spawn",    vec![WasmType::I64], vec![WasmType::I64]),
                ("send",     vec![WasmType::I64, WasmType::I64], vec![]),
                ("receive",  vec![], vec![WasmType::I64]),
                ("self_pid", vec![], vec![WasmType::I64]),
            ];
            for (name, ptypes, rtypes) in &process_fns {
                if !self.known_foreign_fns.contains(*name) {
                    self.foreign_imports.push(IrForeignImport {
                        module: "japl".to_string(),
                        name: name.to_string(),
                        param_count: ptypes.len(),
                        has_return: !rtypes.is_empty(),
                        param_types: ptypes.clone(),
                        return_types: rtypes.clone(),
                    });
                }
            }
        }

        let mut all_functions = std::mem::take(&mut self.functions);
        all_functions.append(&mut self.closure_funcs);

        let heap_start = self.next_string_offset;
        let heap_start = if heap_start % 16 != 0 {
            heap_start + (16 - heap_start % 16)
        } else {
            heap_start
        };

        let const_list: Vec<(String, i64)> = self.constants.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        IrModule {
            functions: all_functions,
            foreign_imports: std::mem::take(&mut self.foreign_imports),
            types: std::mem::take(&mut self.types),
            string_data: std::mem::take(&mut self.string_data),
            heap_start,
            uses_processes: self.uses_processes,
            constants: const_list,
        }
    }

    fn has_tail_call(&self, expr: &ast::Expr, fn_name: &str) -> bool {
        match expr {
            ast::Expr::Call(func, _) => {
                if let ast::Expr::Ident(name) = func.as_ref() {
                    name == fn_name
                } else {
                    false
                }
            }
            ast::Expr::If(_, then_branch, else_branch) => {
                self.has_tail_call(then_branch, fn_name)
                    || else_branch.as_ref().map_or(false, |e| self.has_tail_call(e, fn_name))
            }
            ast::Expr::Block(stmts, final_expr) => {
                if let Some(e) = final_expr {
                    self.has_tail_call(e, fn_name)
                } else if let Some(ast::Stmt::Expr(e)) = stmts.last() {
                    self.has_tail_call(e, fn_name)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn lower_fn_def(&mut self, fd: &ast::FnDef) -> IrFunction {
        let params: Vec<String> = fd.params.iter().map(|p| p.name.clone()).collect();
        let has_return = fd.ret_ty.is_some();
        self.current_fn_name = Some(fd.name.clone());
        self.current_fn_param_count = fd.params.len();
        self.current_fn_params = params.clone();

        let mut locals = HashSet::new();
        self.collect_locals(&fd.body, &mut locals);
        // Remove params from locals
        for p in &params {
            locals.remove(p);
        }

        // Check for tail recursion
        let is_tail_recursive = self.has_tail_call(&fd.body, &fd.name);

        let body = if is_tail_recursive {
            self.lower_expr_tco(&fd.body, &fd.name, &params)
        } else {
            self.lower_expr(&fd.body, has_return)
        };
        self.current_fn_name = None;

        let mut locals: Vec<String> = locals.into_iter().collect();
        if is_tail_recursive {
            locals.push("__tco_result".to_string());
        }

        IrFunction {
            name: fd.name.clone(),
            params,
            locals,
            body,
            has_return,
            is_closure_body: false,
        }
    }

    fn lower_expr_tco(&mut self, expr: &ast::Expr, fn_name: &str, params: &[String]) -> IrExpr {
        // Wrap the body in a loop; tail calls become continues
        let body_stmts = self.lower_body_with_tco(expr, fn_name, params);
        IrExpr::Loop("$tco_loop".to_string(), body_stmts)
    }

    fn lower_body_with_tco(&mut self, expr: &ast::Expr, fn_name: &str, params: &[String]) -> Vec<IrStmt> {
        match expr {
            ast::Expr::Block(stmts, final_expr) => {
                let mut ir_stmts = Vec::new();
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let(name, e) | ast::Stmt::LetTyped(name, _, e) => {
                            let val = self.lower_expr(e, false);
                            ir_stmts.push(IrStmt::Let(name.clone(), val));
                        }
                        ast::Stmt::Expr(e) => {
                            if self.has_tail_call(e, fn_name) {
                                // This is a tail call in statement position
                                let tco_stmt = self.lower_tail_stmt(e, fn_name, params);
                                ir_stmts.push(tco_stmt);
                            } else {
                                let val = self.lower_expr(e, false);
                                ir_stmts.push(IrStmt::Expr(val));
                            }
                        }
                    }
                }
                if let Some(e) = final_expr {
                    if self.has_tail_call(e, fn_name) {
                        let tco_stmt = self.lower_tail_stmt(e, fn_name, params);
                        ir_stmts.push(tco_stmt);
                    } else {
                        let val = self.lower_expr(e, false);
                        ir_stmts.push(IrStmt::Expr(val));
                    }
                }
                ir_stmts
            }
            ast::Expr::If(cond, then_branch, else_branch) => {
                // Lower if with TCO in branches
                let cond_ir = self.lower_expr(cond, false);
                let then_ir = self.lower_branch_tco(then_branch, fn_name, params);
                let else_ir = else_branch.as_ref().map(|e| {
                    self.lower_branch_tco(e, fn_name, params)
                });
                vec![IrStmt::Expr(IrExpr::If(
                    Box::new(cond_ir),
                    Box::new(then_ir),
                    else_ir.map(Box::new),
                ))]
            }
            ast::Expr::Call(func, args) => {
                if let ast::Expr::Ident(name) = func.as_ref() {
                    if name == fn_name {
                        // Direct tail call - convert to continue
                        let updates: Vec<(String, IrExpr)> = params.iter().zip(args.iter())
                            .map(|(p, a)| (p.clone(), self.lower_expr(a, false)))
                            .collect();
                        return vec![IrStmt::Expr(IrExpr::Continue("$tco_loop".to_string(), updates))];
                    }
                }
                let val = self.lower_expr(expr, false);
                vec![IrStmt::Expr(val)]
            }
            _ => {
                let val = self.lower_expr(expr, false);
                vec![IrStmt::Expr(val)]
            }
        }
    }

    fn lower_branch_tco(&mut self, expr: &ast::Expr, fn_name: &str, params: &[String]) -> IrExpr {
        if self.has_tail_call(expr, fn_name) {
            let stmts = self.lower_body_with_tco(expr, fn_name, params);
            IrExpr::Block(stmts, None)
        } else {
            // Non-recursive branch: break out of TCO loop with the value
            let val = self.lower_expr(expr, false);
            IrExpr::Break("$tco_loop__exit".to_string(), Box::new(val))
        }
    }

    fn lower_tail_stmt(&mut self, expr: &ast::Expr, fn_name: &str, params: &[String]) -> IrStmt {
        match expr {
            ast::Expr::Call(func, args) => {
                if let ast::Expr::Ident(name) = func.as_ref() {
                    if name == fn_name {
                        let updates: Vec<(String, IrExpr)> = params.iter().zip(args.iter())
                            .map(|(p, a)| (p.clone(), self.lower_expr(a, false)))
                            .collect();
                        return IrStmt::Expr(IrExpr::Continue("$tco_loop".to_string(), updates));
                    }
                }
                IrStmt::Expr(self.lower_expr(expr, false))
            }
            ast::Expr::If(cond, then_branch, else_branch) => {
                let cond_ir = self.lower_expr(cond, false);
                let then_ir = self.lower_branch_tco(then_branch, fn_name, params);
                let else_ir = else_branch.as_ref().map(|e| {
                    self.lower_branch_tco(e, fn_name, params)
                });
                IrStmt::Expr(IrExpr::If(
                    Box::new(cond_ir),
                    Box::new(then_ir),
                    else_ir.map(Box::new),
                ))
            }
            ast::Expr::Block(stmts, final_expr) => {
                let ir_stmts = self.lower_body_with_tco(expr, fn_name, params);
                IrStmt::Expr(IrExpr::Block(ir_stmts, None))
            }
            _ => IrStmt::Expr(self.lower_expr(expr, false)),
        }
    }

    fn collect_locals(&self, expr: &ast::Expr, locals: &mut HashSet<String>) {
        match expr {
            ast::Expr::Block(stmts, final_expr) => {
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let(name, e) | ast::Stmt::LetTyped(name, _, e) => {
                            locals.insert(name.clone());
                            self.collect_locals(e, locals);
                        }
                        ast::Stmt::Expr(e) => self.collect_locals(e, locals),
                    }
                }
                if let Some(e) = final_expr {
                    self.collect_locals(e, locals);
                }
            }
            ast::Expr::If(c, t, e) => {
                self.collect_locals(c, locals);
                self.collect_locals(t, locals);
                if let Some(e) = e {
                    self.collect_locals(e, locals);
                }
            }
            ast::Expr::Call(f, args) => {
                self.collect_locals(f, locals);
                for a in args {
                    self.collect_locals(a, locals);
                }
            }
            ast::Expr::BinOp(_, l, r) => {
                self.collect_locals(l, locals);
                self.collect_locals(r, locals);
            }
            ast::Expr::Receive(arms) => {
                locals.insert("$recv_msg".to_string());
                self.collect_match_arm_locals(arms, locals);
            }
            ast::Expr::Match(scrutinee, arms) => {
                self.collect_locals(scrutinee, locals);
                self.collect_match_arm_locals(arms, locals);
            }
            ast::Expr::Record(fields) => {
                for (_, v) in fields {
                    self.collect_locals(v, locals);
                }
            }
            ast::Expr::FieldAccess(e, _) => self.collect_locals(e, locals),
            ast::Expr::RecordUpdate(base, fields) => {
                self.collect_locals(base, locals);
                for (_, v) in fields {
                    self.collect_locals(v, locals);
                }
            }
            ast::Expr::Pipe(l, r) => {
                self.collect_locals(l, locals);
                self.collect_locals(r, locals);
            }
            ast::Expr::Lambda(_, _, body) => {
                self.collect_locals(body, locals);
            }
            ast::Expr::Tuple(exprs) => {
                for e in exprs {
                    self.collect_locals(e, locals);
                }
            }
            ast::Expr::TupleAccess(e, _) => self.collect_locals(e, locals),
            ast::Expr::UseExpr(name, resource, body) => {
                locals.insert(name.clone());
                self.collect_locals(resource, locals);
                self.collect_locals(body, locals);
            }
            _ => {}
        }
    }

    fn collect_match_arm_locals(&self, arms: &[ast::MatchArm], locals: &mut HashSet<String>) {
        for arm in arms {
            match &arm.pattern {
                ast::Pattern::Variant(name, bindings) => {
                    // If it's not a known variant, it's a catch-all binding
                    if !self.variant_map.contains_key(name) && bindings.is_empty() {
                        locals.insert(name.clone());
                    }
                    for b in bindings {
                        locals.insert(b.clone());
                    }
                }
                _ => {}
            }
            if let Some(guard) = &arm.guard {
                self.collect_locals(guard, locals);
            }
            self.collect_locals(&arm.body, locals);
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr, _in_tail: bool) -> IrExpr {
        match expr {
            ast::Expr::IntLit(n) => IrExpr::I64Const(*n),
            ast::Expr::StringLit(s) => {
                let (off, len) = self.intern_string(s);
                IrExpr::StringConst(off, len)
            }
            ast::Expr::BoolLit(b) => IrExpr::BoolConst(*b),
            ast::Expr::Ident(name) => {
                // Check if it's a constant
                if let Some(val) = self.constants.get(name).copied() {
                    IrExpr::I64Const(val)
                // Check if it's a nullary variant constructor
                } else if let Some((_, tag, 0)) = self.variant_map.get(name) {
                    IrExpr::TaggedNew(*tag, vec![])
                } else if self.known_functions.contains(name) {
                    self.wrap_function_as_closure(name)
                } else {
                    IrExpr::LocalGet(name.clone())
                }
            }
            ast::Expr::Call(func, args) => {
                self.lower_call(func, args)
            }
            ast::Expr::BinOp(op, l, r) => {
                if matches!(op, ast::BinOp::Concat) {
                    let ll = self.lower_expr(l, false);
                    let rr = self.lower_expr(r, false);
                    IrExpr::StringConcat(Box::new(ll), Box::new(rr))
                } else {
                    let irop = match op {
                        ast::BinOp::Add => IrBinOp::Add,
                        ast::BinOp::Sub => IrBinOp::Sub,
                        ast::BinOp::Mul => IrBinOp::Mul,
                        ast::BinOp::Div => IrBinOp::Div,
                        ast::BinOp::Mod => IrBinOp::Mod,
                        ast::BinOp::Eq => IrBinOp::Eq,
                        ast::BinOp::Neq => IrBinOp::Neq,
                        ast::BinOp::Lt => IrBinOp::Lt,
                        ast::BinOp::Gt => IrBinOp::Gt,
                        ast::BinOp::LtEq => IrBinOp::LtEq,
                        ast::BinOp::GtEq => IrBinOp::GtEq,
                        ast::BinOp::Concat => unreachable!(),
                        ast::BinOp::And => IrBinOp::Mul, // logical AND as multiply (both must be 1)
                        ast::BinOp::Or => IrBinOp::Add,  // logical OR: crude but works for 0/1
                    };
                    let ll = self.lower_expr(l, false);
                    let rr = self.lower_expr(r, false);
                    IrExpr::BinOp(irop, Box::new(ll), Box::new(rr))
                }
            }
            ast::Expr::If(cond, then, else_) => {
                let c = self.lower_expr(cond, false);
                let t = self.lower_expr(then, false);
                let e = else_.as_ref().map(|e| Box::new(self.lower_expr(e, false)));
                IrExpr::If(Box::new(c), Box::new(t), e)
            }
            ast::Expr::Block(stmts, final_expr) => {
                let mut ir_stmts = Vec::new();
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let(name, e) | ast::Stmt::LetTyped(name, _, e) => {
                            // Track variable types for field access and show resolution
                            match e {
                                ast::Expr::Record(fields) => {
                                    let mut sorted_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                                    sorted_names.sort();
                                    self.record_fields.insert(name.clone(), sorted_names);
                                }
                                ast::Expr::RecordUpdate(base, _) => {
                                    if let ast::Expr::Ident(base_name) = base.as_ref() {
                                        if let Some(fields) = self.record_fields.get(base_name).cloned() {
                                            self.record_fields.insert(name.clone(), fields);
                                        }
                                    }
                                }
                                ast::Expr::BoolLit(_) => {
                                    self.bool_vars.insert(name.clone());
                                }
                                _ => {}
                            }
                            let val = self.lower_expr(e, false);
                            ir_stmts.push(IrStmt::Let(name.clone(), val));
                        }
                        ast::Stmt::Expr(e) => {
                            let val = self.lower_expr(e, false);
                            ir_stmts.push(IrStmt::Expr(val));
                        }
                    }
                }
                let final_ir = final_expr.as_ref().map(|e| Box::new(self.lower_expr(e, false)));
                IrExpr::Block(ir_stmts, final_ir)
            }
            ast::Expr::Match(scrutinee, arms) => {
                self.lower_match(scrutinee, arms)
            }
            ast::Expr::Record(fields) => {
                let mut sorted: Vec<(String, IrExpr)> = fields.iter().map(|(n, e)| {
                    (n.clone(), self.lower_expr(e, false))
                }).collect();
                sorted.sort_by(|a, b| a.0.cmp(&b.0));
                IrExpr::RecordNew(sorted)
            }
            ast::Expr::FieldAccess(base, field) => {
                let base_ir = self.lower_expr(base, false);
                // Try to resolve field index from known record types
                let index = self.resolve_record_field(base, field);
                IrExpr::RecordGetField(Box::new(base_ir), index)
            }
            ast::Expr::RecordUpdate(base, updates) => {
                // Get all field names from the base record
                let all_fields = if let ast::Expr::Ident(var_name) = base.as_ref() {
                    self.record_fields.get(var_name).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                };

                let update_map: HashMap<String, &ast::Expr> = updates.iter()
                    .map(|(n, e)| (n.clone(), e))
                    .collect();

                // Build new record with all fields, using update values where present
                let mut sorted_fields: Vec<(String, IrExpr)> = all_fields.iter().map(|name| {
                    if let Some(expr) = update_map.get(name) {
                        (name.clone(), self.lower_expr(expr, false))
                    } else {
                        let index = all_fields.iter().position(|f| f == name).unwrap() as u32;
                        let base_ir = self.lower_expr(base, false);
                        (name.clone(), IrExpr::RecordGetField(Box::new(base_ir), index))
                    }
                }).collect();
                sorted_fields.sort_by(|a, b| a.0.cmp(&b.0));

                // Track the same field names for the result variable
                if let ast::Expr::Ident(var_name) = base.as_ref() {
                    if let Some(fields) = self.record_fields.get(var_name).cloned() {
                        // We'll register this when it's bound to a let
                        let _ = fields;
                    }
                }

                IrExpr::RecordNew(sorted_fields)
            }
            ast::Expr::Pipe(left, right) => {
                // Desugar: a |> f  =>  f(a)
                let l = self.lower_expr(left, false);
                let r = self.lower_expr(right, false);
                // right should be a function, call it with left as argument
                // But right is an IrExpr... we need to reconstruct this as a call
                // The simplest: if right is LocalGet(name), emit Call(name, [left])
                match r {
                    IrExpr::LocalGet(name) => IrExpr::Call(name, vec![l]),
                    _ => IrExpr::CallIndirect(Box::new(r), vec![l]),
                }
            }
            ast::Expr::Lambda(params, _ret_ty, body) => {
                self.lower_lambda(params, body)
            }
            ast::Expr::Receive(arms) => {
                self.uses_processes = true;
                // receive { Pattern => body } desugars to:
                // let $msg = japl.receive()
                // match $msg { ... }
                let recv = IrExpr::Receive;
                // Build match arms from receive arms, matching on the received message
                self.lower_receive_match(recv, arms)
            }
            ast::Expr::Tuple(exprs) => {
                // Lower tuple as a record with positional fields
                let fields: Vec<(String, IrExpr)> = exprs.iter().enumerate()
                    .map(|(i, e)| (format!("_{}", i), self.lower_expr(e, false)))
                    .collect();
                IrExpr::RecordNew(fields)
            }
            ast::Expr::TupleAccess(expr, index) => {
                let base = self.lower_expr(expr, false);
                IrExpr::RecordGetField(Box::new(base), *index as u32)
            }
            ast::Expr::FloatLit(f) => {
                // Store float as i64 bits
                IrExpr::I64Const(f.to_bits() as i64)
            }
            ast::Expr::ByteLit(b) => {
                IrExpr::I64Const(*b as i64)
            }
            ast::Expr::UseExpr(_name, _resource, _body) => {
                // TODO: implement use expressions with auto-close
                IrExpr::I64Const(0)
            }
        }
    }

    fn lower_call(&mut self, func: &ast::Expr, args: &[ast::Expr]) -> IrExpr {
        // Check for builtin calls
        if let ast::Expr::Ident(name) = func {
            match name.as_str() {
                "println" => {
                    let arg = self.lower_expr(&args[0], false);
                    return IrExpr::Call("$println".to_string(), vec![arg]);
                }
                "show" => {
                    let is_bool = match &args[0] {
                        ast::Expr::BoolLit(_) => true,
                        ast::Expr::Ident(n) => self.bool_vars.contains(n),
                        _ => false,
                    };
                    let arg = self.lower_expr(&args[0], false);
                    if is_bool {
                        return IrExpr::ShowBool(Box::new(arg));
                    } else {
                        return IrExpr::ShowInt(Box::new(arg));
                    }
                }
                "spawn" => {
                    self.uses_processes = true;
                    if args.len() == 1 {
                        let arg = self.lower_expr(&args[0], false);
                        return IrExpr::Spawn(Box::new(arg));
                    }
                }
                "send" => {
                    self.uses_processes = true;
                    if args.len() == 2 {
                        let pid = self.lower_expr(&args[0], false);
                        let msg = self.lower_expr(&args[1], false);
                        return IrExpr::Send(Box::new(pid), Box::new(msg));
                    }
                }
                "self_pid" | "self" => {
                    self.uses_processes = true;
                    return IrExpr::SelfPid;
                }
                _ => {}
            }
            // Check if it's a variant constructor with fields
            if let Some((_, tag, field_count)) = self.variant_map.get(name).cloned() {
                if field_count > 0 {
                    let ir_args: Vec<IrExpr> = args.iter().map(|a| self.lower_expr(a, false)).collect();
                    return IrExpr::TaggedNew(tag, ir_args);
                }
            }
            // Check if it's a known function or foreign function
            if self.known_functions.contains(name) || self.known_foreign_fns.contains(name) {
                let ir_args: Vec<IrExpr> = args.iter().map(|a| self.lower_expr(a, false)).collect();
                return IrExpr::Call(name.clone(), ir_args);
            }
            // Otherwise it's a local variable holding a closure - use indirect call
            let f = self.lower_expr(func, false);
            let ir_args: Vec<IrExpr> = args.iter().map(|a| self.lower_expr(a, false)).collect();
            return IrExpr::CallIndirect(Box::new(f), ir_args);
        }
        // Indirect call (calling a closure/function value)
        let f = self.lower_expr(func, false);
        let ir_args: Vec<IrExpr> = args.iter().map(|a| self.lower_expr(a, false)).collect();
        IrExpr::CallIndirect(Box::new(f), ir_args)
    }

    fn lower_match(&mut self, scrutinee: &ast::Expr, arms: &[ast::MatchArm]) -> IrExpr {
        let scrut = self.lower_expr(scrutinee, false);
        self.lower_match_ir(scrut, arms)
    }

    fn lower_match_ir(&mut self, scrut: IrExpr, arms: &[ast::MatchArm]) -> IrExpr {
        let mut result: Option<IrExpr> = None;

        for arm in arms.iter().rev() {
            let body = self.lower_expr(&arm.body, false);

            match &arm.pattern {
                ast::Pattern::Wildcard => {
                    // Wildcard always matches
                    result = Some(body);
                }
                ast::Pattern::IntLit(n) => {
                    let cond = IrExpr::BinOp(
                        IrBinOp::Eq,
                        Box::new(scrut.clone()),
                        Box::new(IrExpr::I64Const(*n)),
                    );
                    let else_body = result.unwrap_or(IrExpr::I64Const(0));
                    result = Some(IrExpr::If(Box::new(cond), Box::new(body), Some(Box::new(else_body))));
                }
                ast::Pattern::StringLit(_s) => {
                    // String comparison is complex; for now just use the body
                    result = Some(body);
                }
                ast::Pattern::BoolLit(b) => {
                    let cond = IrExpr::BinOp(
                        IrBinOp::Eq,
                        Box::new(scrut.clone()),
                        Box::new(IrExpr::I64Const(if *b { 1 } else { 0 })),
                    );
                    let else_body = result.unwrap_or(IrExpr::I64Const(0));
                    result = Some(IrExpr::If(Box::new(cond), Box::new(body), Some(Box::new(else_body))));
                }
                ast::Pattern::Variant(vname, bindings) => {
                    if let Some((_, tag, _field_count)) = self.variant_map.get(vname).cloned() {
                        // Extract fields first (before guard), then check guard, then body
                        let mut stmts = Vec::new();
                        for (i, binding) in bindings.iter().enumerate() {
                            stmts.push(IrStmt::Let(
                                binding.clone(),
                                IrExpr::TaggedGetField(Box::new(scrut.clone()), i as u32),
                            ));
                        }

                        // If guard present: extract fields -> check guard -> body
                        let arm_body = if let Some(guard) = &arm.guard {
                            let guard_ir = self.lower_expr(guard, false);
                            let else_body = result.clone().unwrap_or(IrExpr::I64Const(0));
                            let guarded_body = IrExpr::If(Box::new(guard_ir), Box::new(body), Some(Box::new(else_body)));
                            if stmts.is_empty() {
                                guarded_body
                            } else {
                                IrExpr::Block(stmts, Some(Box::new(guarded_body)))
                            }
                        } else {
                            if stmts.is_empty() {
                                body
                            } else {
                                IrExpr::Block(stmts, Some(Box::new(body)))
                            }
                        };

                        let cond = IrExpr::BinOp(
                            IrBinOp::Eq,
                            Box::new(IrExpr::TaggedGetTag(Box::new(scrut.clone()))),
                            Box::new(IrExpr::I64Const(tag as i64)),
                        );

                        let else_body = result.unwrap_or(IrExpr::I64Const(0));
                        result = Some(IrExpr::If(Box::new(cond), Box::new(arm_body), Some(Box::new(else_body))));
                    } else {
                        // Unknown variant -- might be a catch-all identifier binding
                        // Treat as wildcard that binds the value
                        let mut stmts = Vec::new();
                        stmts.push(IrStmt::Let(vname.clone(), scrut.clone()));
                        let arm_body = IrExpr::Block(stmts, Some(Box::new(body)));
                        result = Some(arm_body);
                    }
                }
            }
        }

        result.unwrap_or(IrExpr::I64Const(0))
    }

    fn lower_receive_match(&mut self, recv: IrExpr, arms: &[ast::MatchArm]) -> IrExpr {
        // receive { Pattern => body } becomes:
        // let $recv_msg = receive()
        // match $recv_msg { ... }
        let stmts = vec![IrStmt::Let("$recv_msg".to_string(), recv)];
        let match_expr = self.lower_match_ir(IrExpr::LocalGet("$recv_msg".to_string()), arms);
        IrExpr::Block(stmts, Some(Box::new(match_expr)))
    }

    fn lower_lambda(&mut self, params: &[ast::Param], body: &ast::Expr) -> IrExpr {
        // Find free variables
        let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
        let mut free_vars = Vec::new();
        self.find_free_vars(body, &param_names, &mut HashSet::new(), &mut free_vars);

        let table_idx = self.next_table_index;
        self.next_table_index += 1;

        let closure_name = format!("$closure_{}", table_idx);

        // Build params: first param is the closure pointer, then regular params
        let mut all_params = vec!["$closure_ptr".to_string()];
        all_params.extend(params.iter().map(|p| p.name.clone()));

        // Body: first extract captured vars from closure
        let mut stmts = Vec::new();
        for (i, var) in free_vars.iter().enumerate() {
            stmts.push(IrStmt::Let(
                var.clone(),
                IrExpr::ClosureGetCapture(
                    Box::new(IrExpr::LocalGet("$closure_ptr".to_string())),
                    i as u32,
                ),
            ));
        }

        let lowered_body = self.lower_expr(body, false);
        let func_body = if stmts.is_empty() {
            lowered_body
        } else {
            IrExpr::Block(stmts, Some(Box::new(lowered_body)))
        };

        let mut locals = HashSet::new();
        self.collect_ir_locals(&func_body, &mut locals);
        for p in &all_params {
            locals.remove(p);
        }

        self.closure_funcs.push(IrFunction {
            name: closure_name,
            params: all_params,
            locals: locals.into_iter().collect(),
            body: func_body,
            has_return: true,
            is_closure_body: true,
        });

        // Create closure struct with captured values
        let captured: Vec<IrExpr> = free_vars.iter().map(|v| IrExpr::LocalGet(v.clone())).collect();
        IrExpr::ClosureNew(table_idx, captured)
    }

    fn find_free_vars(
        &self, expr: &ast::Expr, bound: &HashSet<String>,
        seen: &mut HashSet<String>, result: &mut Vec<String>,
    ) {
        let builtins: HashSet<&str> = ["println", "show", "spawn", "send", "self_pid", "self"].iter().copied().collect();
        match expr {
            ast::Expr::Ident(name) => {
                if !bound.contains(name) && !seen.contains(name) {
                    if !self.variant_map.contains_key(name)
                        && !builtins.contains(name.as_str())
                        && !self.known_functions.contains(name)
                        && !self.known_foreign_fns.contains(name)
                        && !self.constants.contains_key(name)
                    {
                        seen.insert(name.clone());
                        result.push(name.clone());
                    }
                }
            }
            ast::Expr::Call(f, args) => {
                self.find_free_vars(f, bound, seen, result);
                for a in args {
                    self.find_free_vars(a, bound, seen, result);
                }
            }
            ast::Expr::BinOp(_, l, r) => {
                self.find_free_vars(l, bound, seen, result);
                self.find_free_vars(r, bound, seen, result);
            }
            ast::Expr::If(c, t, e) => {
                self.find_free_vars(c, bound, seen, result);
                self.find_free_vars(t, bound, seen, result);
                if let Some(e) = e {
                    self.find_free_vars(e, bound, seen, result);
                }
            }
            ast::Expr::Block(stmts, final_expr) => {
                let mut bound = bound.clone();
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let(name, e) | ast::Stmt::LetTyped(name, _, e) => {
                            self.find_free_vars(e, &bound, seen, result);
                            bound.insert(name.clone());
                        }
                        ast::Stmt::Expr(e) => self.find_free_vars(e, &bound, seen, result),
                    }
                }
                if let Some(e) = final_expr {
                    self.find_free_vars(e, &bound, seen, result);
                }
            }
            ast::Expr::Lambda(params, _, body) => {
                let mut inner_bound = bound.clone();
                for p in params {
                    inner_bound.insert(p.name.clone());
                }
                self.find_free_vars(body, &inner_bound, seen, result);
            }
            ast::Expr::Match(scrutinee, arms) => {
                self.find_free_vars(scrutinee, bound, seen, result);
                self.find_free_vars_arms(arms, bound, seen, result);
            }
            ast::Expr::Receive(arms) => {
                self.find_free_vars_arms(arms, bound, seen, result);
            }
            ast::Expr::Pipe(l, r) => {
                self.find_free_vars(l, bound, seen, result);
                self.find_free_vars(r, bound, seen, result);
            }
            ast::Expr::Record(fields) => {
                for (_, v) in fields {
                    self.find_free_vars(v, bound, seen, result);
                }
            }
            ast::Expr::FieldAccess(e, _) => self.find_free_vars(e, bound, seen, result),
            ast::Expr::RecordUpdate(base, fields) => {
                self.find_free_vars(base, bound, seen, result);
                for (_, v) in fields {
                    self.find_free_vars(v, bound, seen, result);
                }
            }
            ast::Expr::Tuple(exprs) => {
                for e in exprs {
                    self.find_free_vars(e, bound, seen, result);
                }
            }
            ast::Expr::TupleAccess(e, _) => self.find_free_vars(e, bound, seen, result),
            ast::Expr::UseExpr(name, resource, body) => {
                self.find_free_vars(resource, bound, seen, result);
                let mut inner_bound = bound.clone();
                inner_bound.insert(name.clone());
                self.find_free_vars(body, &inner_bound, seen, result);
            }
            _ => {}
        }
    }

    fn find_free_vars_arms(
        &self, arms: &[ast::MatchArm], bound: &HashSet<String>,
        seen: &mut HashSet<String>, result: &mut Vec<String>,
    ) {
        for arm in arms {
            let mut arm_bound = bound.clone();
            if let ast::Pattern::Variant(_, bindings) = &arm.pattern {
                for b in bindings {
                    arm_bound.insert(b.clone());
                }
            }
            if let Some(guard) = &arm.guard {
                self.find_free_vars(guard, &arm_bound, seen, result);
            }
            self.find_free_vars(&arm.body, &arm_bound, seen, result);
        }
    }

    fn collect_ir_locals(&self, expr: &IrExpr, locals: &mut HashSet<String>) {
        match expr {
            IrExpr::Block(stmts, final_expr) => {
                for stmt in stmts {
                    match stmt {
                        IrStmt::Let(name, e) => {
                            locals.insert(name.clone());
                            self.collect_ir_locals(e, locals);
                        }
                        IrStmt::Expr(e) => self.collect_ir_locals(e, locals),
                    }
                }
                if let Some(e) = final_expr {
                    self.collect_ir_locals(e, locals);
                }
            }
            IrExpr::If(c, t, e) => {
                self.collect_ir_locals(c, locals);
                self.collect_ir_locals(t, locals);
                if let Some(e) = e {
                    self.collect_ir_locals(e, locals);
                }
            }
            IrExpr::Call(_, args) => {
                for a in args { self.collect_ir_locals(a, locals); }
            }
            IrExpr::BinOp(_, l, r) => {
                self.collect_ir_locals(l, locals);
                self.collect_ir_locals(r, locals);
            }
            _ => {}
        }
    }

    fn resolve_record_field(&self, base: &ast::Expr, field: &str) -> u32 {
        if let ast::Expr::Ident(var_name) = base {
            if let Some(fields) = self.record_fields.get(var_name) {
                if let Some(pos) = fields.iter().position(|f| f == field) {
                    return pos as u32;
                }
            }
        }
        0
    }

    fn wrap_function_as_closure(&mut self, fn_name: &str) -> IrExpr {
        let arity = self.known_function_arity.get(fn_name).copied().unwrap_or(1);
        let table_idx = self.next_table_index;
        self.next_table_index += 1;

        let closure_name = format!("$closure_{}", table_idx);

        // Build closure body: forward all args to the real function
        let mut params = vec!["$closure_ptr".to_string()];
        let mut call_args = Vec::new();
        for i in 0..arity {
            let pname = format!("$arg_{}", i);
            params.push(pname.clone());
            call_args.push(IrExpr::LocalGet(pname));
        }

        let body = IrExpr::Call(fn_name.to_string(), call_args);

        self.closure_funcs.push(IrFunction {
            name: closure_name,
            params,
            locals: Vec::new(),
            body,
            has_return: true,
            is_closure_body: true,
        });

        // Create closure with no captures
        IrExpr::ClosureNew(table_idx, vec![])
    }
}

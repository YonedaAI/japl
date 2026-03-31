use std::collections::HashMap;
use crate::ast;
use crate::types::Type;

/// Effect tracking
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Pure,
    IO,
    LLM,
    Process,
    Fail,
}

struct Checker {
    errors: Vec<String>,
    env: HashMap<String, Type>,
    variant_types: HashMap<String, (String, Vec<Type>)>, // variant_name -> (type_name, field_types)
    fn_sigs: HashMap<String, (Vec<Type>, Type)>, // fn_name -> (param_types, return_type)
    fn_effects: HashMap<String, Vec<Effect>>, // fn_name -> observed effects
    strict: bool,
    current_fn: Option<String>,
}

fn ast_type_to_type(t: &ast::Type) -> Type {
    match t {
        ast::Type::Named(name) => {
            match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "String" => Type::String,
                "Bool" => Type::Bool,
                "Byte" => Type::Byte,
                "Unit" => Type::Unit,
                _ => Type::Named(name.clone(), vec![]),
            }
        }
        ast::Type::FnType(params, ret) => {
            let pts: Vec<Type> = params.iter().map(|p| ast_type_to_type(p)).collect();
            let rt = ast_type_to_type(ret);
            Type::Fn(pts, Box::new(rt))
        }
        ast::Type::Tuple(types) => {
            Type::Tuple(types.iter().map(|t| ast_type_to_type(t)).collect())
        }
        ast::Type::Void => Type::Unit,
    }
}

impl Checker {
    fn new(strict: bool) -> Self {
        let mut env = HashMap::new();
        // Builtins
        env.insert("println".to_string(), Type::Fn(vec![Type::String], Box::new(Type::Unit)));
        env.insert("show".to_string(), Type::Fn(vec![Type::Int], Box::new(Type::String)));
        env.insert("spawn".to_string(), Type::Fn(vec![Type::Fn(vec![], Box::new(Type::Unit))], Box::new(Type::Int)));
        env.insert("send".to_string(), Type::Fn(vec![Type::Int, Type::Int], Box::new(Type::Unit)));
        env.insert("self_pid".to_string(), Type::Fn(vec![], Box::new(Type::Int)));

        Checker {
            errors: Vec::new(),
            env,
            variant_types: HashMap::new(),
            fn_sigs: HashMap::new(),
            fn_effects: HashMap::new(),
            strict,
            current_fn: None,
        }
    }

    fn record_effect(&mut self, effect: Effect) {
        if let Some(ref fn_name) = self.current_fn {
            self.fn_effects.entry(fn_name.clone())
                .or_insert_with(Vec::new)
                .push(effect);
        }
    }

    fn infer_expr(&mut self, expr: &ast::Expr) -> Type {
        match expr {
            ast::Expr::IntLit(_) => Type::Int,
            ast::Expr::FloatLit(_) => Type::Float,
            ast::Expr::StringLit(_) => Type::String,
            ast::Expr::BoolLit(_) => Type::Bool,
            ast::Expr::ByteLit(_) => Type::Byte,
            ast::Expr::Ident(name) => {
                if let Some(ty) = self.env.get(name) {
                    ty.clone()
                } else if self.variant_types.contains_key(name) {
                    Type::Named(name.clone(), vec![])
                } else {
                    // Unknown variable - could be a variant or generic
                    Type::Var(0)
                }
            }
            ast::Expr::Call(func, args) => {
                if let ast::Expr::Ident(name) = func.as_ref() {
                    match name.as_str() {
                        "println" => {
                            self.record_effect(Effect::IO);
                            if let Some(arg) = args.first() {
                                let arg_ty = self.infer_expr(arg);
                                if arg_ty != Type::String && arg_ty != Type::Var(0) {
                                    self.errors.push(format!("type error: println expects String, got {}", arg_ty));
                                }
                            }
                            Type::Unit
                        }
                        "show" => {
                            if let Some(arg) = args.first() {
                                let _arg_ty = self.infer_expr(arg);
                            }
                            Type::String
                        }
                        "spawn" => {
                            self.record_effect(Effect::Process);
                            for arg in args {
                                self.infer_expr(arg);
                            }
                            Type::Int
                        }
                        "send" => {
                            self.record_effect(Effect::Process);
                            for arg in args {
                                self.infer_expr(arg);
                            }
                            Type::Unit
                        }
                        "self_pid" | "self" => {
                            self.record_effect(Effect::Process);
                            Type::Int
                        }
                        _ => {
                            for arg in args {
                                self.infer_expr(arg);
                            }
                            if let Some((_, ret)) = self.fn_sigs.get(name) {
                                ret.clone()
                            } else if self.variant_types.contains_key(name) {
                                Type::Named(name.clone(), vec![])
                            } else {
                                Type::Var(0)
                            }
                        }
                    }
                } else {
                    self.infer_expr(func);
                    for arg in args {
                        self.infer_expr(arg);
                    }
                    Type::Var(0)
                }
            }
            ast::Expr::BinOp(op, left, right) => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                match op {
                    ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div | ast::BinOp::Mod => {
                        if lt != Type::Var(0) && lt != Type::Int {
                            self.errors.push(format!("type error: arithmetic expects Int, got {}", lt));
                        }
                        if rt != Type::Var(0) && rt != Type::Int {
                            self.errors.push(format!("type error: arithmetic expects Int, got {}", rt));
                        }
                        Type::Int
                    }
                    ast::BinOp::Eq | ast::BinOp::Neq | ast::BinOp::Lt | ast::BinOp::Gt |
                    ast::BinOp::LtEq | ast::BinOp::GtEq => Type::Bool,
                    ast::BinOp::Concat => {
                        if lt != Type::Var(0) && lt != Type::String {
                            self.errors.push(format!("type error: <> expects String, got {}", lt));
                        }
                        if rt != Type::Var(0) && rt != Type::String {
                            self.errors.push(format!("type error: <> expects String, got {}", rt));
                        }
                        Type::String
                    }
                    ast::BinOp::And | ast::BinOp::Or => {
                        if lt != Type::Var(0) && lt != Type::Bool {
                            self.errors.push(format!("type error: logical op expects Bool, got {}", lt));
                        }
                        Type::Bool
                    }
                }
            }
            ast::Expr::If(cond, then, else_) => {
                let ct = self.infer_expr(cond);
                if ct != Type::Var(0) && ct != Type::Bool {
                    self.errors.push(format!("type error: if condition expects Bool, got {}", ct));
                }
                let tt = self.infer_expr(then);
                if let Some(e) = else_ {
                    let et = self.infer_expr(e);
                    // Could check tt == et, but be lenient for now
                    tt
                } else {
                    tt
                }
            }
            ast::Expr::Block(stmts, final_expr) => {
                for stmt in stmts {
                    match stmt {
                        ast::Stmt::Let(name, e) => {
                            let ty = self.infer_expr(e);
                            self.env.insert(name.clone(), ty);
                        }
                        ast::Stmt::LetTyped(name, declared_ty, e) => {
                            let expected = ast_type_to_type(declared_ty);
                            let actual = self.infer_expr(e);
                            if actual != Type::Var(0) && expected != actual {
                                self.errors.push(format!("type error: expected {}, got {}", expected, actual));
                            }
                            self.env.insert(name.clone(), expected);
                        }
                        ast::Stmt::Expr(e) => {
                            self.infer_expr(e);
                        }
                    }
                }
                if let Some(e) = final_expr {
                    self.infer_expr(e)
                } else {
                    Type::Unit
                }
            }
            ast::Expr::Lambda(params, ret_ty, body) => {
                let param_types: Vec<Type> = params.iter()
                    .map(|p| ast_type_to_type(&p.ty))
                    .collect();
                for (p, t) in params.iter().zip(param_types.iter()) {
                    self.env.insert(p.name.clone(), t.clone());
                }
                let body_ty = self.infer_expr(body);
                Type::Fn(param_types, Box::new(body_ty))
            }
            ast::Expr::Match(scrutinee, arms) => {
                self.infer_expr(scrutinee);
                let mut result_ty = Type::Var(0);
                for arm in arms {
                    let ty = self.infer_expr(&arm.body);
                    if result_ty == Type::Var(0) {
                        result_ty = ty;
                    }
                }
                result_ty
            }
            ast::Expr::Receive(arms) => {
                self.record_effect(Effect::Process);
                let mut result_ty = Type::Var(0);
                for arm in arms {
                    let ty = self.infer_expr(&arm.body);
                    if result_ty == Type::Var(0) {
                        result_ty = ty;
                    }
                }
                result_ty
            }
            ast::Expr::Record(fields) => {
                let mut field_types = std::collections::BTreeMap::new();
                for (name, val) in fields {
                    let ty = self.infer_expr(val);
                    field_types.insert(name.clone(), ty);
                }
                Type::Record(field_types)
            }
            ast::Expr::FieldAccess(base, _field) => {
                self.infer_expr(base);
                Type::Var(0) // Can't resolve without full record typing
            }
            ast::Expr::RecordUpdate(base, fields) => {
                let base_ty = self.infer_expr(base);
                for (_, val) in fields {
                    self.infer_expr(val);
                }
                base_ty
            }
            ast::Expr::Pipe(left, right) => {
                let _lt = self.infer_expr(left);
                self.infer_expr(right)
            }
            ast::Expr::Tuple(exprs) => {
                let types: Vec<Type> = exprs.iter().map(|e| self.infer_expr(e)).collect();
                Type::Tuple(types)
            }
            ast::Expr::TupleAccess(expr, _idx) => {
                self.infer_expr(expr);
                Type::Var(0)
            }
            ast::Expr::UseExpr(name, resource, body) => {
                let rt = self.infer_expr(resource);
                self.env.insert(name.clone(), rt);
                self.infer_expr(body)
            }
        }
    }

    fn check_exhaustiveness(&mut self, scrutinee_type: &str, arms: &[ast::MatchArm]) {
        // Collect all variant names used in match
        let matched: Vec<&str> = arms.iter().filter_map(|arm| {
            match &arm.pattern {
                ast::Pattern::Variant(name, _) => Some(name.as_str()),
                ast::Pattern::Wildcard => return None, // wildcard covers all
                _ => None,
            }
        }).collect();

        // If there's a wildcard, it's exhaustive
        if arms.iter().any(|a| matches!(&a.pattern, ast::Pattern::Wildcard)) {
            return;
        }

        // Check if all variants of the type are covered
        let all_variants: Vec<&str> = self.variant_types.iter()
            .filter(|(_, (tname, _))| tname == scrutinee_type)
            .map(|(vname, _)| vname.as_str())
            .collect();

        if !all_variants.is_empty() {
            for v in &all_variants {
                if !matched.contains(v) {
                    self.errors.push(format!("non-exhaustive match: missing variant {}", v));
                }
            }
        }
    }
}

/// Check a program and return a list of error messages.
pub fn check_program(program: &ast::Program, strict: bool) -> Vec<String> {
    let mut checker = Checker::new(strict);

    // Collect type definitions
    for item in &program.items {
        if let ast::TopLevel::TypeDef(td) = item {
            for variant in &td.variants {
                let field_types: Vec<Type> = variant.fields.iter()
                    .map(|f| ast_type_to_type(f))
                    .collect();
                checker.variant_types.insert(
                    variant.name.clone(),
                    (td.name.clone(), field_types),
                );
            }
        }
    }

    // Collect function signatures
    for item in &program.items {
        if let ast::TopLevel::FnDef(fd) = item {
            let param_types: Vec<Type> = fd.params.iter()
                .map(|p| ast_type_to_type(&p.ty))
                .collect();
            let ret_ty = fd.ret_ty.as_ref()
                .map(|t| ast_type_to_type(t))
                .unwrap_or(Type::Unit);
            checker.fn_sigs.insert(fd.name.clone(), (param_types.clone(), ret_ty.clone()));

            // Add params to env
            for (p, t) in fd.params.iter().zip(param_types.iter()) {
                checker.env.insert(p.name.clone(), t.clone());
            }
        }
    }

    // Check function bodies
    for item in &program.items {
        if let ast::TopLevel::FnDef(fd) = item {
            checker.current_fn = Some(fd.name.clone());
            // Add params to env
            let param_types: Vec<Type> = fd.params.iter()
                .map(|p| ast_type_to_type(&p.ty))
                .collect();
            for (p, t) in fd.params.iter().zip(param_types.iter()) {
                checker.env.insert(p.name.clone(), t.clone());
            }

            let body_ty = checker.infer_expr(&fd.body);

            // Check return type matches
            if let Some(ref ret_ty) = fd.ret_ty {
                let expected = ast_type_to_type(ret_ty);
                if body_ty != Type::Var(0) && expected != body_ty {
                    checker.errors.push(format!(
                        "type error in fn {}: declared return type {} but body has type {}",
                        fd.name, expected, body_ty
                    ));
                }
            }

            checker.current_fn = None;
        }
    }

    // Check effects in strict mode
    if strict {
        for item in &program.items {
            if let ast::TopLevel::FnDef(fd) = item {
                if let Some(effects) = checker.fn_effects.get(&fd.name) {
                    let has_io = effects.contains(&Effect::IO);
                    let has_process = effects.contains(&Effect::Process);

                    // Check if function is annotated with effect
                    let declared_effect = fd.effect.as_deref();

                    if has_io && declared_effect != Some("IO") && declared_effect != Some("io") {
                        checker.errors.push(format!(
                            "effect error in fn {}: performs IO but not declared with IO effect",
                            fd.name
                        ));
                    }
                    if has_process && declared_effect != Some("Process") && declared_effect != Some("process") {
                        checker.errors.push(format!(
                            "effect error in fn {}: uses processes but not declared with Process effect",
                            fd.name
                        ));
                    }
                }
            }
        }
    }

    // Check exhaustiveness for match expressions
    // (This is a simplified check - full version would need type inference on scrutinee)
    for item in &program.items {
        if let ast::TopLevel::FnDef(fd) = item {
            check_exhaustiveness_in_expr(&mut checker, &fd.body);
        }
    }

    checker.errors
}

fn check_exhaustiveness_in_expr(checker: &mut Checker, expr: &ast::Expr) {
    match expr {
        ast::Expr::Match(scrutinee, arms) => {
            // Try to determine scrutinee type
            if let ast::Expr::Ident(name) = scrutinee.as_ref() {
                let type_name = checker.env.get(name).and_then(|ty| {
                    if let Type::Named(tn, _) = ty { Some(tn.clone()) } else { None }
                });
                if let Some(tn) = type_name {
                    checker.check_exhaustiveness(&tn, arms);
                }
            }
            // Recurse
            check_exhaustiveness_in_expr(checker, scrutinee);
            for arm in arms {
                check_exhaustiveness_in_expr(checker, &arm.body);
            }
        }
        ast::Expr::Block(stmts, final_expr) => {
            for stmt in stmts {
                match stmt {
                    ast::Stmt::Let(_, e) | ast::Stmt::LetTyped(_, _, e) | ast::Stmt::Expr(e) => {
                        check_exhaustiveness_in_expr(checker, e);
                    }
                }
            }
            if let Some(e) = final_expr {
                check_exhaustiveness_in_expr(checker, e);
            }
        }
        ast::Expr::If(c, t, e) => {
            check_exhaustiveness_in_expr(checker, c);
            check_exhaustiveness_in_expr(checker, t);
            if let Some(e) = e {
                check_exhaustiveness_in_expr(checker, e);
            }
        }
        ast::Expr::Call(f, args) => {
            check_exhaustiveness_in_expr(checker, f);
            for a in args {
                check_exhaustiveness_in_expr(checker, a);
            }
        }
        _ => {}
    }
}

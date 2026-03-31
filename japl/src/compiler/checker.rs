use std::collections::HashMap;
use super::ast;
use super::types::Type;

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
    warnings: Vec<String>,
    env: HashMap<String, Type>,
    variant_types: HashMap<String, (String, Vec<Type>)>, // variant_name -> (type_name, field_types)
    fn_sigs: HashMap<String, (Vec<Type>, Type)>, // fn_name -> (param_types, return_type)
    fn_type_params: HashMap<String, Vec<String>>, // fn_name -> type param names
    fn_effects: HashMap<String, Vec<Effect>>, // fn_name -> observed effects
    strict: bool,
    current_fn: Option<String>,
    /// Names declared as constants (treated as known identifiers)
    const_names: HashMap<String, Type>,
    /// Foreign function names
    foreign_fns: HashMap<String, Type>,
}

fn ast_type_to_type(t: &ast::Type) -> Type {
    ast_type_to_type_with_params(t, &[])
}

/// Convert AST type to checker type, treating names in `type_params` as TypeParam.
fn ast_type_to_type_with_params(t: &ast::Type, type_params: &[String]) -> Type {
    match t {
        ast::Type::Named(name) => {
            if type_params.contains(name) {
                return Type::TypeParam(name.clone());
            }
            match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "String" => Type::String,
                "Bool" => Type::Bool,
                "Byte" => Type::Byte,
                "Unit" => Type::Unit,
                "Pid" => Type::Int, // Pid is an alias for Int (process IDs are integers at runtime)
                _ => Type::Named(name.clone(), vec![]),
            }
        }
        ast::Type::FnType(params, ret) => {
            let pts: Vec<Type> = params.iter().map(|p| ast_type_to_type_with_params(p, type_params)).collect();
            let rt = ast_type_to_type_with_params(ret, type_params);
            Type::Fn(pts, Box::new(rt))
        }
        ast::Type::Tuple(types) => {
            Type::Tuple(types.iter().map(|t| ast_type_to_type_with_params(t, type_params)).collect())
        }
        ast::Type::Void => Type::Unit,
    }
}

/// Substitute TypeParam occurrences in a type using a substitution map.
fn substitute_type_params(ty: &Type, subst: &HashMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam(name) => {
            subst.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        Type::Fn(params, ret) => {
            let new_params: Vec<Type> = params.iter().map(|p| substitute_type_params(p, subst)).collect();
            let new_ret = substitute_type_params(ret, subst);
            Type::Fn(new_params, Box::new(new_ret))
        }
        Type::Tuple(types) => {
            Type::Tuple(types.iter().map(|t| substitute_type_params(t, subst)).collect())
        }
        Type::Named(name, args) => {
            let new_args: Vec<Type> = args.iter().map(|a| substitute_type_params(a, subst)).collect();
            Type::Named(name.clone(), new_args)
        }
        _ => ty.clone(),
    }
}

impl Checker {
    fn new(strict: bool) -> Self {
        let mut env = HashMap::new();
        // Builtins
        env.insert("println".to_string(), Type::Fn(vec![Type::String], Box::new(Type::Unit)));
        env.insert("show".to_string(), Type::Fn(vec![Type::Int], Box::new(Type::String)));
        env.insert("spawn".to_string(), Type::Fn(vec![Type::Fn(vec![], Box::new(Type::Unit))], Box::new(Type::Int)));
        // send accepts any message type (Var(0) = polymorphic)
        env.insert("send".to_string(), Type::Fn(vec![Type::Int, Type::Var(0)], Box::new(Type::Unit)));
        env.insert("self_pid".to_string(), Type::Fn(vec![], Box::new(Type::Int)));
        env.insert("receive".to_string(), Type::Fn(vec![], Box::new(Type::Var(0))));
        // LLM/AI builtins
        env.insert("llm".to_string(), Type::Fn(vec![Type::String], Box::new(Type::String)));
        // Network builtins
        env.insert("http_get".to_string(), Type::Fn(vec![Type::String], Box::new(Type::String)));
        env.insert("http_post".to_string(), Type::Fn(vec![Type::String, Type::String], Box::new(Type::String)));

        Checker {
            errors: Vec::new(),
            warnings: Vec::new(),
            env,
            variant_types: HashMap::new(),
            fn_sigs: HashMap::new(),
            fn_type_params: HashMap::new(),
            fn_effects: HashMap::new(),
            strict,
            current_fn: None,
            const_names: HashMap::new(),
            foreign_fns: HashMap::new(),
        }
    }

    fn record_effect(&mut self, effect: Effect) {
        if let Some(ref fn_name) = self.current_fn {
            self.fn_effects.entry(fn_name.clone())
                .or_insert_with(Vec::new)
                .push(effect);
        }
    }

    /// Try to unify a declared param type (which may contain TypeParam) with an actual arg type.
    /// Populates `subst` with bindings. Returns true if unification succeeds.
    fn unify(&self, declared: &Type, actual: &Type, subst: &mut HashMap<String, Type>) -> bool {
        // If actual is unknown, we can't learn anything
        if *actual == Type::Var(0) {
            return true;
        }
        match declared {
            Type::TypeParam(name) => {
                if let Some(existing) = subst.get(name) {
                    // Already bound: check consistency
                    *existing == *actual
                } else {
                    subst.insert(name.clone(), actual.clone());
                    true
                }
            }
            Type::Fn(d_params, d_ret) => {
                if let Type::Fn(a_params, a_ret) = actual {
                    if d_params.len() != a_params.len() {
                        return false;
                    }
                    for (dp, ap) in d_params.iter().zip(a_params.iter()) {
                        if !self.unify(dp, ap, subst) {
                            return false;
                        }
                    }
                    self.unify(d_ret, a_ret, subst)
                } else {
                    false
                }
            }
            Type::Tuple(d_types) => {
                if let Type::Tuple(a_types) = actual {
                    if d_types.len() != a_types.len() {
                        return false;
                    }
                    for (dt, at) in d_types.iter().zip(a_types.iter()) {
                        if !self.unify(dt, at, subst) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            _ => {
                // Concrete types: just check equality
                *declared == *actual || *declared == Type::Var(0)
            }
        }
    }

    /// Bind pattern variables into the environment so arm bodies can reference them.
    fn bind_pattern_vars(&mut self, pattern: &ast::Pattern) {
        match pattern {
            ast::Pattern::Variant(name, bindings) => {
                // Look up variant field types
                if let Some((_, field_types)) = self.variant_types.get(name) {
                    for (i, binding) in bindings.iter().enumerate() {
                        if binding != "_" {
                            let ty = field_types.get(i).cloned().unwrap_or(Type::Var(0));
                            self.env.insert(binding.clone(), ty);
                        }
                    }
                } else {
                    // Unknown variant; bind as Var(0)
                    for binding in bindings {
                        if binding != "_" {
                            self.env.insert(binding.clone(), Type::Var(0));
                        }
                    }
                }
            }
            ast::Pattern::Wildcard | ast::Pattern::IntLit(_)
            | ast::Pattern::StringLit(_) | ast::Pattern::BoolLit(_) => {}
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
                    // Zero-argument variant constructor used as value
                    let (type_name, _) = &self.variant_types[name];
                    Type::Named(type_name.clone(), vec![])
                } else if self.fn_sigs.contains_key(name) {
                    // Function used as value (e.g., passed to higher-order function)
                    let (params, ret) = &self.fn_sigs[name];
                    Type::Fn(params.clone(), Box::new(ret.clone()))
                } else if self.const_names.contains_key(name) {
                    self.const_names[name].clone()
                } else if self.foreign_fns.contains_key(name) {
                    self.foreign_fns[name].clone()
                } else {
                    // Hard error: unknown identifier
                    self.errors.push(format!("type error: unknown identifier '{}'", name));
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
                        "llm" => {
                            self.record_effect(Effect::IO);
                            for arg in args {
                                self.infer_expr(arg);
                            }
                            Type::String
                        }
                        "http_get" | "http_post" => {
                            self.record_effect(Effect::IO);
                            for arg in args {
                                self.infer_expr(arg);
                            }
                            Type::String
                        }
                        _ => {
                            // Infer arg types first
                            let arg_types: Vec<Type> = args.iter()
                                .map(|a| self.infer_expr(a))
                                .collect();

                            // Check if this is a generic function call
                            let tparams = self.fn_type_params.get(name).cloned();
                            if let Some(ref tp) = tparams {
                                if !tp.is_empty() {
                                    // Get the declared signature (with TypeParam types)
                                    if let Some((param_types, ret_ty)) = self.fn_sigs.get(name).cloned() {
                                        // Unify declared param types with actual arg types
                                        let mut subst = HashMap::new();
                                        for (declared, actual) in param_types.iter().zip(arg_types.iter()) {
                                            self.unify(declared, actual, &mut subst);
                                        }

                                        // Substitute type params in return type
                                        let resolved_ret = substitute_type_params(&ret_ty, &subst);

                                        // If return type still has unresolved type params, fall back to Var(0)
                                        if let Type::TypeParam(_) = &resolved_ret {
                                            return Type::Var(0);
                                        }
                                        return resolved_ret;
                                    }
                                }
                            }

                            // Non-generic path
                            if let Some((param_types, ret)) = self.fn_sigs.get(name).cloned() {
                                // Check argument count
                                if arg_types.len() != param_types.len() {
                                    self.errors.push(format!(
                                        "type error: function '{}' expects {} arguments, got {}",
                                        name, param_types.len(), arg_types.len()
                                    ));
                                } else {
                                    // Check argument types
                                    for (i, (expected, actual)) in param_types.iter().zip(arg_types.iter()).enumerate() {
                                        if *actual != Type::Var(0) && *expected != Type::Var(0)
                                            && !matches!(expected, Type::TypeParam(_))
                                            && !matches!(actual, Type::TypeParam(_))
                                            && *expected != *actual
                                        {
                                            // Allow variant of expected union type
                                            let is_variant = if let Type::Named(ref actual_name, _) = actual {
                                                if let Type::Named(ref expected_name, _) = expected {
                                                    self.variant_types.get(actual_name)
                                                        .map_or(false, |(parent, _)| parent == expected_name)
                                                } else { false }
                                            } else { false };
                                            if !is_variant {
                                                self.errors.push(format!(
                                                    "type error: argument {} of '{}' expects {}, got {}",
                                                    i + 1, name, expected, actual
                                                ));
                                            }
                                        }
                                    }
                                }
                                ret
                            } else if self.variant_types.contains_key(name) {
                                let (type_name, ref field_types) = self.variant_types[name].clone();
                                // Check variant constructor argument count
                                if arg_types.len() != field_types.len() {
                                    self.errors.push(format!(
                                        "type error: variant '{}' expects {} fields, got {}",
                                        name, field_types.len(), arg_types.len()
                                    ));
                                }
                                Type::Named(type_name, vec![])
                            } else if self.foreign_fns.contains_key(name) {
                                // Foreign function call -- return Var(0) since we don't track return types fully
                                Type::Var(0)
                            } else if let Some(ty) = self.env.get(name).cloned() {
                                // Variable with function type being called (e.g., closures, HOF params)
                                if let Type::Fn(param_types, ret) = ty {
                                    if arg_types.len() != param_types.len() {
                                        self.errors.push(format!(
                                            "type error: function '{}' expects {} arguments, got {}",
                                            name, param_types.len(), arg_types.len()
                                        ));
                                    }
                                    *ret
                                } else {
                                    self.errors.push(format!("type error: '{}' is not a function (has type {})", name, ty));
                                    Type::Var(0)
                                }
                            } else {
                                self.errors.push(format!("type error: unknown function '{}'", name));
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
                        if lt != Type::Var(0) && lt != Type::Int && !matches!(lt, Type::TypeParam(_)) {
                            self.errors.push(format!("type error: arithmetic expects Int, got {}", lt));
                        }
                        if rt != Type::Var(0) && rt != Type::Int && !matches!(rt, Type::TypeParam(_)) {
                            self.errors.push(format!("type error: arithmetic expects Int, got {}", rt));
                        }
                        Type::Int
                    }
                    ast::BinOp::Eq | ast::BinOp::Neq | ast::BinOp::Lt | ast::BinOp::Gt |
                    ast::BinOp::LtEq | ast::BinOp::GtEq => Type::Bool,
                    ast::BinOp::Concat => {
                        if lt != Type::Var(0) && lt != Type::String && !matches!(lt, Type::TypeParam(_)) {
                            self.errors.push(format!("type error: <> expects String, got {}", lt));
                        }
                        if rt != Type::Var(0) && rt != Type::String && !matches!(rt, Type::TypeParam(_)) {
                            self.errors.push(format!("type error: <> expects String, got {}", rt));
                        }
                        Type::String
                    }
                    ast::BinOp::And | ast::BinOp::Or => {
                        if lt != Type::Var(0) && lt != Type::Bool && !matches!(lt, Type::TypeParam(_)) {
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
                    // Both branches must have compatible types
                    if tt != Type::Var(0) && et != Type::Var(0)
                        && !matches!(tt, Type::TypeParam(_))
                        && !matches!(et, Type::TypeParam(_))
                    {
                        // Check if both are variants of the same union type
                        let tt_parent = if let Type::Named(ref n, _) = tt {
                            self.variant_types.get(n).map(|(p, _)| p.clone())
                        } else { None };
                        let et_parent = if let Type::Named(ref n, _) = et {
                            self.variant_types.get(n).map(|(p, _)| p.clone())
                        } else { None };
                        let compatible = tt == et
                            || (tt_parent.is_some() && tt_parent == et_parent)
                            // One branch returns a variant of the other's type
                            || tt_parent.as_ref().map_or(false, |p| {
                                if let Type::Named(ref n, _) = et { n == p } else { false }
                            })
                            || et_parent.as_ref().map_or(false, |p| {
                                if let Type::Named(ref n, _) = tt { n == p } else { false }
                            });
                        if !compatible {
                            self.errors.push(format!(
                                "type error: if branches have incompatible types: {} vs {}",
                                tt, et
                            ));
                        }
                    }
                    tt
                } else {
                    // No else branch: result is Unit (then branch is executed for side effects)
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
                            if actual != Type::Var(0) && expected != actual && !matches!(actual, Type::TypeParam(_)) {
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
            ast::Expr::Lambda(params, _ret_ty, body) => {
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
                    self.bind_pattern_vars(&arm.pattern);
                    if let Some(ref guard) = arm.guard {
                        self.infer_expr(guard);
                    }
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
                    self.bind_pattern_vars(&arm.pattern);
                    if let Some(ref guard) = arm.guard {
                        self.infer_expr(guard);
                    }
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
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);
                // Pipe applies the right side as a function to the left side
                if let Type::Fn(params, ret) = &rt {
                    if params.len() == 1 {
                        let expected = &params[0];
                        if lt != Type::Var(0) && *expected != Type::Var(0)
                            && !matches!(expected, Type::TypeParam(_))
                            && !matches!(lt, Type::TypeParam(_))
                            && lt != *expected
                        {
                            self.errors.push(format!(
                                "type error: pipe argument has type {}, but function expects {}",
                                lt, expected
                            ));
                        }
                        *ret.clone()
                    } else {
                        rt // Can't apply, return as-is
                    }
                } else if rt == Type::Var(0) {
                    Type::Var(0)
                } else {
                    // Right side of pipe should be a function
                    rt
                }
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
                ast::Pattern::Wildcard => None,
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
            let missing: Vec<&str> = all_variants.iter()
                .filter(|v| !matched.contains(*v))
                .copied()
                .collect();
            if !missing.is_empty() {
                self.warnings.push(format!(
                    "warning: non-exhaustive match on {}: missing variants {}",
                    scrutinee_type,
                    missing.join(", ")
                ));
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
                    .map(|f| ast_type_to_type_with_params(f, &td.type_params))
                    .collect();
                checker.variant_types.insert(
                    variant.name.clone(),
                    (td.name.clone(), field_types),
                );
            }
        }
    }

    // Collect constants
    for item in &program.items {
        if let ast::TopLevel::Const(cd) = item {
            // Infer type from constant value
            let ty = checker.infer_expr(&cd.value);
            checker.const_names.insert(cd.name.clone(), ty.clone());
            checker.env.insert(cd.name.clone(), ty);
        }
    }

    // Collect foreign function declarations
    for item in &program.items {
        if let ast::TopLevel::ForeignFn(ff) = item {
            let param_types: Vec<Type> = ff.params.iter()
                .map(|p| ast_type_to_type(&p.ty))
                .collect();
            let ret_ty = ff.ret_ty.as_ref()
                .map(|t| ast_type_to_type(t))
                .unwrap_or(Type::Unit);
            let fn_ty = Type::Fn(param_types.clone(), Box::new(ret_ty.clone()));
            checker.foreign_fns.insert(ff.name.clone(), fn_ty.clone());
            checker.env.insert(ff.name.clone(), fn_ty);
            checker.fn_sigs.insert(ff.name.clone(), (param_types, ret_ty));
        }
    }

    // Collect function signatures (with type param awareness)
    for item in &program.items {
        if let ast::TopLevel::FnDef(fd) = item {
            let tp = &fd.type_params;
            let param_types: Vec<Type> = fd.params.iter()
                .map(|p| ast_type_to_type_with_params(&p.ty, tp))
                .collect();
            let ret_ty = fd.ret_ty.as_ref()
                .map(|t| ast_type_to_type_with_params(t, tp))
                .unwrap_or(Type::Var(0)); // Unknown until body is checked
            checker.fn_sigs.insert(fd.name.clone(), (param_types.clone(), ret_ty.clone()));

            // Store type params if any
            if !fd.type_params.is_empty() {
                checker.fn_type_params.insert(fd.name.clone(), fd.type_params.clone());
            }

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
            let tp = &fd.type_params;
            // Add params to env
            let param_types: Vec<Type> = fd.params.iter()
                .map(|p| ast_type_to_type_with_params(&p.ty, tp))
                .collect();
            for (p, t) in fd.params.iter().zip(param_types.iter()) {
                checker.env.insert(p.name.clone(), t.clone());
            }

            let body_ty = checker.infer_expr(&fd.body);

            // If no declared return type, update fn_sigs with inferred body type
            if fd.ret_ty.is_none() && body_ty != Type::Var(0) {
                if let Some(sig) = checker.fn_sigs.get_mut(&fd.name) {
                    sig.1 = body_ty.clone();
                }
            }

            // Check return type matches (skip if body type is a TypeParam -- generic bodies are permissive)
            if let Some(ref ret_ty) = fd.ret_ty {
                let expected = ast_type_to_type_with_params(ret_ty, tp);
                // Check if body type is a variant of the expected union type
                let is_variant_of_expected = if let Type::Named(body_name, _) = &body_ty {
                    if let Type::Named(expected_name, _) = &expected {
                        checker.variant_types.get(body_name)
                            .map_or(false, |(parent, _)| parent == expected_name)
                    } else { false }
                } else { false };

                if body_ty != Type::Var(0) && expected != body_ty
                    && !is_variant_of_expected
                    && !matches!(body_ty, Type::TypeParam(_))
                    && !matches!(expected, Type::TypeParam(_))
                {
                    checker.errors.push(format!(
                        "type error in fn {}: declared return type {} but body has type {}",
                        fd.name, expected, body_ty
                    ));
                }
            }

            checker.current_fn = None;
        }
    }

    // Effect checking is now always on by default.
    // Pass strict=false (--no-strict) to disable effect checking for migration.
    if strict {
        for item in &program.items {
            if let ast::TopLevel::FnDef(fd) = item {
                // Skip main -- it's the entry point and implicitly effectful
                if fd.name == "main" {
                    continue;
                }
                if let Some(effects) = checker.fn_effects.get(&fd.name) {
                    let has_io = effects.contains(&Effect::IO);
                    let has_process = effects.contains(&Effect::Process);
                    let has_any_effect = has_io || has_process;

                    // Check if function is annotated with any effect
                    let declared_effect = fd.effect.as_deref();
                    let has_declared = declared_effect.is_some();

                    if has_any_effect && !has_declared {
                        // Effect mismatches are warnings, not errors, until
                        // proper effect annotation syntax is in the main compile path
                        if has_io && has_process {
                            eprintln!(
                                "warning: effect: fn {} performs IO and uses processes but not declared with effect annotation",
                                fd.name
                            );
                        } else if has_io {
                            eprintln!(
                                "warning: effect: fn {} performs IO but not declared with IO effect",
                                fd.name
                            );
                        } else {
                            eprintln!(
                                "warning: effect: fn {} uses processes but not declared with Process effect",
                                fd.name
                            );
                        }
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

    // Print warnings to stderr (non-fatal)
    for w in &checker.warnings {
        eprintln!("{}", w);
    }

    // Deduplicate errors
    let mut seen = std::collections::HashSet::new();
    checker.errors.retain(|e| seen.insert(e.clone()));
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

//! Bidirectional type checking and inference for JAPL.
//!
//! Implements the core type checking algorithm:
//! - `infer(expr)` synthesizes a type for an expression
//! - `check(expr, expected)` checks an expression against an expected type
//! - Pattern type checking and exhaustiveness checking

use japl_ast::{BinOp, Expr, Pattern, TypeExpr, UnaryOp};
use japl_common::{Diagnostic, DiagnosticSink, Span};
use japl_types::{
    Effect, EffectRow, PrimitiveTypes, Type, TypeId, TypeInterner, TypeVar, TypeVarKind,
};

use crate::env::TypeEnv;
use crate::errors::TypeError;
use crate::unify::UnificationEngine;

/// The main type checker.
pub struct TypeChecker {
    pub interner: TypeInterner,
    pub engine: UnificationEngine,
    pub env: TypeEnv,
    pub diagnostics: DiagnosticSink,
    pub primitives: PrimitiveTypes,
    /// The effect row of the current function context.
    pub current_effects: EffectRow,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut interner = TypeInterner::new();
        let primitives = interner.intern_primitives();
        TypeChecker {
            interner,
            engine: UnificationEngine::new(),
            env: TypeEnv::new(),
            diagnostics: DiagnosticSink::new(),
            primitives,
            current_effects: EffectRow::pure(),
        }
    }

    /// Infer the type of an expression, returning (TypeId, EffectRow).
    pub fn infer(&mut self, expr: &Expr) -> (TypeId, EffectRow) {
        match expr {
            Expr::IntLit { .. } => (self.primitives.int, EffectRow::pure()),
            Expr::FloatLit { .. } => (self.primitives.float, EffectRow::pure()),
            Expr::StringLit { .. } => (self.primitives.string, EffectRow::pure()),
            Expr::CharLit { .. } => (self.primitives.char_ty, EffectRow::pure()),
            Expr::BoolLit { .. } => (self.primitives.bool_ty, EffectRow::pure()),
            Expr::UnitLit { .. } => (self.primitives.unit, EffectRow::pure()),

            Expr::Var { name, span, .. } => {
                match self.env.lookup(name) {
                    Some(ty) => {
                        let ty = self.instantiate(ty);
                        (ty, EffectRow::pure())
                    }
                    None => {
                        self.emit_error(TypeError::UndefinedVariable {
                            name: name.to_string(),
                            span: *span,
                        });
                        (self.primitives.error, EffectRow::pure())
                    }
                }
            }

            Expr::Constructor { name, span, .. } => {
                let ctor_name = name.segments.last().map(|s| s.as_str()).unwrap_or("");
                match self.env.lookup_constructor(ctor_name) {
                    Some(ctor) => {
                        let result_ty = ctor.result_type;
                        if ctor.field_types.is_empty() {
                            (result_ty, EffectRow::pure())
                        } else {
                            // Constructor as a function: field_types -> result_type
                            let fn_ty = self.interner.intern(Type::Fn {
                                params: ctor.field_types.clone(),
                                return_type: result_ty,
                                effects: EffectRow::pure(),
                            });
                            (fn_ty, EffectRow::pure())
                        }
                    }
                    None => {
                        self.emit_error(TypeError::UndefinedConstructor {
                            name: ctor_name.to_string(),
                            span: *span,
                        });
                        (self.primitives.error, EffectRow::pure())
                    }
                }
            }

            Expr::App { func, args, span } => {
                let (func_ty, mut effects) = self.infer(func);
                let func_ty = self.engine.subst.apply(func_ty, &mut self.interner);

                match self.interner.resolve(func_ty).clone() {
                    Type::Fn {
                        params,
                        return_type,
                        effects: fn_effects,
                    } => {
                        if params.len() != args.len() {
                            self.emit_error(TypeError::ArityMismatch {
                                expected: params.len(),
                                found: args.len(),
                                span: *span,
                            });
                            return (self.primitives.error, effects);
                        }
                        for (arg, param_ty) in args.iter().zip(params.iter()) {
                            let (arg_ty, arg_eff) = self.infer(arg);
                            effects = effects.union(&arg_eff);
                            if let Err(e) = self.engine.unify(
                                arg_ty,
                                *param_ty,
                                arg.span(),
                                &mut self.interner,
                            ) {
                                self.emit_error(e);
                            }
                        }
                        effects = effects.union(&fn_effects);
                        (return_type, effects)
                    }
                    Type::Var(_) => {
                        // Function type is unknown; create fresh vars for params and return.
                        let param_tys: Vec<TypeId> = args
                            .iter()
                            .map(|_| self.engine.fresh_var(&mut self.interner))
                            .collect();
                        let ret_ty = self.engine.fresh_var(&mut self.interner);
                        let expected_fn = self.interner.intern(Type::Fn {
                            params: param_tys.clone(),
                            return_type: ret_ty,
                            effects: EffectRow::pure(),
                        });
                        if let Err(e) =
                            self.engine
                                .unify(func_ty, expected_fn, *span, &mut self.interner)
                        {
                            self.emit_error(e);
                        }
                        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                            let (arg_ty, arg_eff) = self.infer(arg);
                            effects = effects.union(&arg_eff);
                            if let Err(e) = self.engine.unify(
                                arg_ty,
                                *param_ty,
                                arg.span(),
                                &mut self.interner,
                            ) {
                                self.emit_error(e);
                            }
                        }
                        (ret_ty, effects)
                    }
                    Type::Error => (self.primitives.error, effects),
                    _ => {
                        self.emit_error(TypeError::NotAFunction {
                            actual_type: japl_types::display_type(func_ty, &self.interner),
                            span: *span,
                        });
                        (self.primitives.error, effects)
                    }
                }
            }

            Expr::BinOp { op, lhs, rhs, span } => self.infer_binop(*op, lhs, rhs, *span),

            Expr::UnaryOp { op, expr, span } => self.infer_unaryop(*op, expr, *span),

            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let (cond_ty, mut effects) = self.infer(condition);
                if let Err(e) = self.engine.unify(
                    cond_ty,
                    self.primitives.bool_ty,
                    condition.span(),
                    &mut self.interner,
                ) {
                    self.emit_error(e);
                }

                let (then_ty, then_eff) = self.infer(then_branch);
                effects = effects.union(&then_eff);

                let (else_ty, else_eff) = self.infer(else_branch);
                effects = effects.union(&else_eff);

                if let Err(e) =
                    self.engine
                        .unify(then_ty, else_ty, *span, &mut self.interner)
                {
                    self.emit_error(e);
                }

                (then_ty, effects)
            }

            Expr::Let {
                pattern,
                ty,
                value,
                body,
                ..
            } => {
                let (val_ty, mut effects) = self.infer(value);

                // If there's a type annotation, check it.
                if let Some(type_expr) = ty {
                    let ann_ty = self.lower_type_expr(type_expr);
                    if let Err(e) = self.engine.unify(
                        val_ty,
                        ann_ty,
                        value.span(),
                        &mut self.interner,
                    ) {
                        self.emit_error(e);
                    }
                }

                self.env.push_scope();
                self.bind_pattern(pattern, val_ty);
                let (body_ty, body_eff) = self.infer(body);
                effects = effects.union(&body_eff);
                self.env.pop_scope();

                (body_ty, effects)
            }

            Expr::Use {
                pattern,
                ty,
                value,
                body,
                ..
            } => {
                // `use` bindings work like `let` for type checking purposes.
                // The linearity checker handles the resource semantics.
                let (val_ty, mut effects) = self.infer(value);

                if let Some(type_expr) = ty {
                    let ann_ty = self.lower_type_expr(type_expr);
                    if let Err(e) = self.engine.unify(
                        val_ty,
                        ann_ty,
                        value.span(),
                        &mut self.interner,
                    ) {
                        self.emit_error(e);
                    }
                }

                self.env.push_scope();
                self.bind_pattern(pattern, val_ty);
                let (body_ty, body_eff) = self.infer(body);
                effects = effects.union(&body_eff);
                self.env.pop_scope();

                (body_ty, effects)
            }

            Expr::Lambda { params, body, span } => {
                self.env.push_scope();
                let param_tys: Vec<TypeId> = params
                    .iter()
                    .map(|p| {
                        let ty = if let Some(type_expr) = &p.ty {
                            self.lower_type_expr(type_expr)
                        } else {
                            self.engine.fresh_var(&mut self.interner)
                        };
                        self.bind_pattern(&p.pattern, ty);
                        ty
                    })
                    .collect();

                let (body_ty, body_eff) = self.infer(body);
                self.env.pop_scope();

                let fn_ty = self.interner.intern(Type::Fn {
                    params: param_tys,
                    return_type: body_ty,
                    effects: body_eff.clone(),
                });

                (fn_ty, EffectRow::pure())
            }

            Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                let (scrut_ty, mut effects) = self.infer(scrutinee);

                if arms.is_empty() {
                    return (self.primitives.never, effects);
                }

                // Infer type of first arm body; all arms must match.
                let result_ty = self.engine.fresh_var(&mut self.interner);

                for arm in arms {
                    self.env.push_scope();
                    self.check_pattern(&arm.pattern, scrut_ty);

                    if let Some(guard) = &arm.guard {
                        let (guard_ty, guard_eff) = self.infer(guard);
                        effects = effects.union(&guard_eff);
                        if let Err(e) = self.engine.unify(
                            guard_ty,
                            self.primitives.bool_ty,
                            guard.span(),
                            &mut self.interner,
                        ) {
                            self.emit_error(e);
                        }
                    }

                    let (arm_ty, arm_eff) = self.infer(&arm.body);
                    effects = effects.union(&arm_eff);
                    if let Err(e) =
                        self.engine
                            .unify(arm_ty, result_ty, arm.body.span(), &mut self.interner)
                    {
                        self.emit_error(e);
                    }
                    self.env.pop_scope();
                }

                (result_ty, effects)
            }

            Expr::FieldAccess { expr, field, span } => {
                let (expr_ty, effects) = self.infer(expr);
                let expr_ty = self.engine.subst.apply(expr_ty, &mut self.interner);

                match self.interner.resolve(expr_ty).clone() {
                    Type::Record { fields, row_var } => {
                        if let Some((_, field_ty)) = fields.iter().find(|(n, _)| n == field) {
                            (*field_ty, effects)
                        } else if row_var.is_some() {
                            // Open record: create a fresh var for the field type.
                            let field_ty = self.engine.fresh_var(&mut self.interner);
                            (field_ty, effects)
                        } else {
                            self.emit_error(TypeError::MissingField {
                                field: field.to_string(),
                                span: *span,
                            });
                            (self.primitives.error, effects)
                        }
                    }
                    Type::Error => (self.primitives.error, effects),
                    _ => {
                        self.emit_error(TypeError::NotARecord {
                            actual_type: japl_types::display_type(expr_ty, &self.interner),
                            span: *span,
                        });
                        (self.primitives.error, effects)
                    }
                }
            }

            Expr::RecordLit { fields, span } => {
                let mut field_types = Vec::new();
                let mut effects = EffectRow::pure();
                for (name, expr) in fields {
                    let (ty, eff) = self.infer(expr);
                    effects = effects.union(&eff);
                    field_types.push((name.clone(), ty));
                }
                let record_ty = self.interner.intern(Type::Record {
                    fields: field_types,
                    row_var: None,
                });
                (record_ty, effects)
            }

            Expr::RecordUpdate {
                base,
                updates,
                span,
            } => {
                let (base_ty, mut effects) = self.infer(base);
                let base_ty = self.engine.subst.apply(base_ty, &mut self.interner);

                match self.interner.resolve(base_ty).clone() {
                    Type::Record {
                        mut fields,
                        row_var,
                    } => {
                        for (name, expr) in updates {
                            let (update_ty, eff) = self.infer(expr);
                            effects = effects.union(&eff);
                            // Update the field type if it exists.
                            if let Some(field) = fields.iter_mut().find(|(n, _)| n == name) {
                                field.1 = update_ty;
                            } else {
                                self.emit_error(TypeError::MissingField {
                                    field: name.to_string(),
                                    span: *span,
                                });
                            }
                        }
                        let result_ty = self.interner.intern(Type::Record { fields, row_var });
                        (result_ty, effects)
                    }
                    _ => {
                        self.emit_error(TypeError::NotARecord {
                            actual_type: japl_types::display_type(base_ty, &self.interner),
                            span: *span,
                        });
                        (self.primitives.error, effects)
                    }
                }
            }

            Expr::TupleLit { elements, span } => {
                let mut elem_types = Vec::new();
                let mut effects = EffectRow::pure();
                for elem in elements {
                    let (ty, eff) = self.infer(elem);
                    effects = effects.union(&eff);
                    elem_types.push(ty);
                }
                let tuple_ty = self.interner.intern(Type::Tuple(elem_types));
                (tuple_ty, effects)
            }

            Expr::ListLit { elements, span } => {
                let elem_ty = self.engine.fresh_var(&mut self.interner);
                let mut effects = EffectRow::pure();
                for elem in elements {
                    let (ty, eff) = self.infer(elem);
                    effects = effects.union(&eff);
                    if let Err(e) =
                        self.engine
                            .unify(ty, elem_ty, elem.span(), &mut self.interner)
                    {
                        self.emit_error(e);
                    }
                }
                // List[elem_ty] -- we'd need a DefId for List.
                // For now, just return the element type wrapped in a placeholder.
                // In a full implementation, this would look up the List type constructor.
                (elem_ty, effects)
            }

            Expr::Pipeline { lhs, rhs, span } => {
                // a |> f  desugars to  f(a)
                let desugared = Expr::App {
                    func: rhs.clone(),
                    args: vec![*lhs.clone()],
                    span: *span,
                };
                self.infer(&desugared)
            }

            Expr::Block { exprs, span } => {
                if exprs.is_empty() {
                    return (self.primitives.unit, EffectRow::pure());
                }
                let mut effects = EffectRow::pure();
                let mut last_ty = self.primitives.unit;
                for expr in exprs {
                    let (ty, eff) = self.infer(expr);
                    effects = effects.union(&eff);
                    last_ty = ty;
                }
                (last_ty, effects)
            }

            Expr::Try { expr, span } => {
                let (inner_ty, mut effects) = self.infer(expr);
                // expr? on Result[a, e] returns a and adds Fail[e] to effects.
                // Simplified: just return a fresh var for now.
                let result_ty = self.engine.fresh_var(&mut self.interner);
                let err_ty = self.engine.fresh_var(&mut self.interner);
                effects = effects.union(&EffectRow::with_effects(vec![Effect::Fail(err_ty)]));
                (result_ty, effects)
            }

            Expr::Annotation { expr, ty, span } => {
                let ann_ty = self.lower_type_expr(ty);
                let (inferred, effects) = self.infer(expr);
                if let Err(e) =
                    self.engine
                        .unify(inferred, ann_ty, *span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                (ann_ty, effects)
            }

            Expr::Compose { lhs, rhs, span } => {
                // f >> g  means  fn(x) -> g(f(x))
                let (f_ty, mut effects) = self.infer(lhs);
                let (g_ty, g_eff) = self.infer(rhs);
                effects = effects.union(&g_eff);

                let input_ty = self.engine.fresh_var(&mut self.interner);
                let mid_ty = self.engine.fresh_var(&mut self.interner);
                let output_ty = self.engine.fresh_var(&mut self.interner);

                let expected_f = self.interner.intern(Type::Fn {
                    params: vec![input_ty],
                    return_type: mid_ty,
                    effects: EffectRow::pure(),
                });
                let expected_g = self.interner.intern(Type::Fn {
                    params: vec![mid_ty],
                    return_type: output_ty,
                    effects: EffectRow::pure(),
                });

                if let Err(e) = self.engine.unify(f_ty, expected_f, *span, &mut self.interner) {
                    self.emit_error(e);
                }
                if let Err(e) = self.engine.unify(g_ty, expected_g, *span, &mut self.interner) {
                    self.emit_error(e);
                }

                let composed = self.interner.intern(Type::Fn {
                    params: vec![input_ty],
                    return_type: output_ty,
                    effects: EffectRow::pure(),
                });
                (composed, effects)
            }

            Expr::Loop {
                bindings,
                condition,
                body,
                span,
            } => {
                self.env.push_scope();
                let mut effects = EffectRow::pure();
                for (name, init_expr) in bindings {
                    let (ty, eff) = self.infer(init_expr);
                    effects = effects.union(&eff);
                    self.env.bind(name.clone(), ty);
                }
                let (cond_ty, cond_eff) = self.infer(condition);
                effects = effects.union(&cond_eff);
                if let Err(e) = self.engine.unify(
                    cond_ty,
                    self.primitives.bool_ty,
                    condition.span(),
                    &mut self.interner,
                ) {
                    self.emit_error(e);
                }
                let (body_ty, body_eff) = self.infer(body);
                effects = effects.union(&body_eff);
                self.env.pop_scope();
                (body_ty, effects)
            }

            Expr::Continue { args, span } => {
                // Continue doesn't produce a value; it's a control-flow jump.
                (self.primitives.never, EffectRow::pure())
            }

            Expr::Receive { arms, span, .. } => {
                // Receive is similar to match but in a process context.
                let msg_ty = self.engine.fresh_var(&mut self.interner);
                let result_ty = self.engine.fresh_var(&mut self.interner);
                let mut effects = EffectRow::with_effects(vec![Effect::Process(msg_ty)]);

                for arm in arms {
                    self.env.push_scope();
                    self.check_pattern(&arm.pattern, msg_ty);
                    let (arm_ty, arm_eff) = self.infer(&arm.body);
                    effects = effects.union(&arm_eff);
                    if let Err(e) = self.engine.unify(
                        arm_ty,
                        result_ty,
                        arm.body.span(),
                        &mut self.interner,
                    ) {
                        self.emit_error(e);
                    }
                    self.env.pop_scope();
                }

                (result_ty, effects)
            }
        }
    }

    /// Check an expression against an expected type.
    pub fn check(&mut self, expr: &Expr, expected: TypeId) -> EffectRow {
        match expr {
            // For lambdas, we can push expected param types down.
            Expr::Lambda { params, body, span } => {
                let expected_resolved =
                    self.engine.subst.apply(expected, &mut self.interner);
                match self.interner.resolve(expected_resolved).clone() {
                    Type::Fn {
                        params: expected_params,
                        return_type: expected_ret,
                        effects: expected_effects,
                    } => {
                        if params.len() != expected_params.len() {
                            self.emit_error(TypeError::ArityMismatch {
                                expected: expected_params.len(),
                                found: params.len(),
                                span: *span,
                            });
                            return EffectRow::pure();
                        }

                        self.env.push_scope();
                        for (param, &expected_ty) in params.iter().zip(expected_params.iter()) {
                            let param_ty = if let Some(type_expr) = &param.ty {
                                let ann_ty = self.lower_type_expr(type_expr);
                                if let Err(e) = self.engine.unify(
                                    ann_ty,
                                    expected_ty,
                                    param.span,
                                    &mut self.interner,
                                ) {
                                    self.emit_error(e);
                                }
                                ann_ty
                            } else {
                                expected_ty
                            };
                            self.bind_pattern(&param.pattern, param_ty);
                        }

                        let body_effects = self.check(body, expected_ret);
                        self.env.pop_scope();
                        body_effects
                    }
                    _ => {
                        // Fall back to infer + unify.
                        let (inferred, effects) = self.infer(expr);
                        if let Err(e) = self.engine.unify(
                            inferred,
                            expected,
                            *span,
                            &mut self.interner,
                        ) {
                            self.emit_error(e);
                        }
                        effects
                    }
                }
            }
            // For everything else, infer and unify.
            _ => {
                let (inferred, effects) = self.infer(expr);
                if let Err(e) =
                    self.engine
                        .unify(inferred, expected, expr.span(), &mut self.interner)
                {
                    self.emit_error(e);
                }
                effects
            }
        }
    }

    // -- Binary and unary operators --

    fn infer_binop(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, span: Span) -> (TypeId, EffectRow) {
        let (lhs_ty, mut effects) = self.infer(lhs);
        let (rhs_ty, rhs_eff) = self.infer(rhs);
        effects = effects.union(&rhs_eff);

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                // Both operands must have the same numeric type.
                if let Err(e) =
                    self.engine
                        .unify(lhs_ty, rhs_ty, span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                (lhs_ty, effects)
            }
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if let Err(e) =
                    self.engine
                        .unify(lhs_ty, rhs_ty, span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                (self.primitives.bool_ty, effects)
            }
            BinOp::And | BinOp::Or => {
                if let Err(e) = self.engine.unify(
                    lhs_ty,
                    self.primitives.bool_ty,
                    lhs.span(),
                    &mut self.interner,
                ) {
                    self.emit_error(e);
                }
                if let Err(e) = self.engine.unify(
                    rhs_ty,
                    self.primitives.bool_ty,
                    rhs.span(),
                    &mut self.interner,
                ) {
                    self.emit_error(e);
                }
                (self.primitives.bool_ty, effects)
            }
            BinOp::Concat => {
                // String/List concatenation.
                if let Err(e) =
                    self.engine
                        .unify(lhs_ty, rhs_ty, span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                (lhs_ty, effects)
            }
            BinOp::Append => {
                // Semigroup append (<>).
                if let Err(e) =
                    self.engine
                        .unify(lhs_ty, rhs_ty, span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                (lhs_ty, effects)
            }
        }
    }

    fn infer_unaryop(&mut self, op: UnaryOp, expr: &Expr, span: Span) -> (TypeId, EffectRow) {
        let (ty, effects) = self.infer(expr);
        match op {
            UnaryOp::Neg => {
                // Numeric negation; type stays the same.
                (ty, effects)
            }
            UnaryOp::Not => {
                if let Err(e) = self.engine.unify(
                    ty,
                    self.primitives.bool_ty,
                    span,
                    &mut self.interner,
                ) {
                    self.emit_error(e);
                }
                (self.primitives.bool_ty, effects)
            }
        }
    }

    // -- Pattern checking --

    /// Check a pattern against an expected type, binding variables in the env.
    pub fn check_pattern(&mut self, pattern: &Pattern, expected: TypeId) {
        match pattern {
            Pattern::Wildcard { .. } => {
                // Accept any type.
            }
            Pattern::Var { name, .. } => {
                self.env.bind(name.clone(), expected);
            }
            Pattern::Literal { expr, span } => {
                let (lit_ty, _) = self.infer(expr);
                if let Err(e) =
                    self.engine
                        .unify(lit_ty, expected, *span, &mut self.interner)
                {
                    self.emit_error(e);
                }
            }
            Pattern::Constructor {
                name,
                fields,
                span,
            } => {
                let ctor_name = name.segments.last().map(|s| s.as_str()).unwrap_or("");
                match self.env.lookup_constructor(ctor_name) {
                    Some(ctor) => {
                        let ctor = ctor.clone();
                        // Unify the result type with expected.
                        if let Err(e) = self.engine.unify(
                            ctor.result_type,
                            expected,
                            *span,
                            &mut self.interner,
                        ) {
                            self.emit_error(e);
                        }
                        // Check each field pattern.
                        if fields.len() != ctor.field_types.len() {
                            self.emit_error(TypeError::ArityMismatch {
                                expected: ctor.field_types.len(),
                                found: fields.len(),
                                span: *span,
                            });
                        } else {
                            for (pat, &field_ty) in fields.iter().zip(ctor.field_types.iter()) {
                                self.check_pattern(pat, field_ty);
                            }
                        }
                    }
                    None => {
                        self.emit_error(TypeError::UndefinedConstructor {
                            name: ctor_name.to_string(),
                            span: *span,
                        });
                    }
                }
            }
            Pattern::Tuple { elements, span } => {
                let elem_tys: Vec<_> = elements
                    .iter()
                    .map(|_| self.engine.fresh_var(&mut self.interner))
                    .collect();
                let expected_tuple = self.interner.intern(Type::Tuple(elem_tys.clone()));
                if let Err(e) =
                    self.engine
                        .unify(expected_tuple, expected, *span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                for (pat, ty) in elements.iter().zip(elem_tys.iter()) {
                    self.check_pattern(pat, *ty);
                }
            }
            Pattern::Record {
                fields,
                rest,
                span,
            } => {
                let field_tys: Vec<_> = fields
                    .iter()
                    .map(|(name, _)| {
                        let ty = self.engine.fresh_var(&mut self.interner);
                        (name.clone(), ty)
                    })
                    .collect();
                let row_var = if *rest {
                    Some(self.engine.fresh_type_var())
                } else {
                    None
                };
                let expected_record = self.interner.intern(Type::Record {
                    fields: field_tys.clone(),
                    row_var,
                });
                if let Err(e) =
                    self.engine
                        .unify(expected_record, expected, *span, &mut self.interner)
                {
                    self.emit_error(e);
                }
                for ((_, pat), (_, ty)) in fields.iter().zip(field_tys.iter()) {
                    self.check_pattern(pat, *ty);
                }
            }
            Pattern::List {
                elements,
                rest,
                span,
            } => {
                let elem_ty = self.engine.fresh_var(&mut self.interner);
                for pat in elements {
                    self.check_pattern(pat, elem_ty);
                }
                if let Some(rest_pat) = rest {
                    // rest pattern should be a list of the same element type.
                    // For now, just bind it with the element type.
                    self.check_pattern(rest_pat, elem_ty);
                }
            }
            Pattern::Or { patterns, span } => {
                for pat in patterns {
                    self.check_pattern(pat, expected);
                }
            }
            Pattern::As {
                pattern,
                name,
                span,
                ..
            } => {
                self.check_pattern(pattern, expected);
                self.env.bind(name.clone(), expected);
            }
            Pattern::Pin { name, span, .. } => {
                // Pinned variable: look up existing binding and unify.
                match self.env.lookup(name) {
                    Some(ty) => {
                        if let Err(e) =
                            self.engine
                                .unify(ty, expected, *span, &mut self.interner)
                        {
                            self.emit_error(e);
                        }
                    }
                    None => {
                        self.emit_error(TypeError::UndefinedVariable {
                            name: name.to_string(),
                            span: *span,
                        });
                    }
                }
            }
        }
    }

    /// Bind variables from a pattern into the environment.
    fn bind_pattern(&mut self, pattern: &Pattern, ty: TypeId) {
        self.check_pattern(pattern, ty);
    }

    // -- Type expression lowering --

    /// Lower a syntactic TypeExpr into a semantic TypeId.
    pub fn lower_type_expr(&mut self, type_expr: &TypeExpr) -> TypeId {
        match type_expr {
            TypeExpr::Named { name, args, .. } => {
                let type_name = name
                    .segments
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(".");

                // Check for primitive types first.
                match type_name.as_str() {
                    "Int" => return self.primitives.int,
                    "Float" => return self.primitives.float,
                    "Float32" => return self.primitives.float32,
                    "Bool" => return self.primitives.bool_ty,
                    "Char" => return self.primitives.char_ty,
                    "String" => return self.primitives.string,
                    "Bytes" => return self.primitives.bytes,
                    "Unit" => return self.primitives.unit,
                    "Never" => return self.primitives.never,
                    _ => {}
                }

                // Look up user-defined type.
                if let Some(type_def) = self.env.lookup_type_def(&type_name) {
                    let def_id = type_def.id;
                    let lowered_args: Vec<_> = args.iter().map(|a| self.lower_type_expr(a)).collect();
                    self.interner.intern(Type::App {
                        constructor: def_id,
                        args: lowered_args,
                    })
                } else {
                    self.primitives.error
                }
            }
            TypeExpr::Var { name, .. } => {
                // Type variable: look up or create.
                let var = TypeVar {
                    id: self.engine.next_var_id(),
                    kind: TypeVarKind::Rigid,
                };
                self.interner.intern(Type::Var(var))
            }
            TypeExpr::Fn {
                params,
                return_type,
                effects,
                ..
            } => {
                let param_tys: Vec<_> = params.iter().map(|p| self.lower_type_expr(p)).collect();
                let ret_ty = self.lower_type_expr(return_type);
                let effect_row = self.lower_effects(effects);
                self.interner.intern(Type::Fn {
                    params: param_tys,
                    return_type: ret_ty,
                    effects: effect_row,
                })
            }
            TypeExpr::Record {
                fields, row_var, ..
            } => {
                let field_tys: Vec<_> = fields
                    .iter()
                    .map(|f| (f.name.clone(), self.lower_type_expr(&f.ty)))
                    .collect();
                let rv = row_var.as_ref().map(|_| TypeVar {
                    id: self.engine.next_var_id(),
                    kind: TypeVarKind::Unification,
                });
                self.interner.intern(Type::Record {
                    fields: field_tys,
                    row_var: rv,
                })
            }
            TypeExpr::Tuple { elements, .. } => {
                let elem_tys: Vec<_> = elements.iter().map(|e| self.lower_type_expr(e)).collect();
                self.interner.intern(Type::Tuple(elem_tys))
            }
            TypeExpr::Owned { inner, .. } => {
                let inner_ty = self.lower_type_expr(inner);
                self.interner.intern(Type::Owned(inner_ty))
            }
            TypeExpr::Borrowed { inner, .. } => {
                let inner_ty = self.lower_type_expr(inner);
                self.interner.intern(Type::Ref(inner_ty))
            }
            TypeExpr::Never { .. } => self.primitives.never,
            TypeExpr::Unit { .. } => self.primitives.unit,
            TypeExpr::Forall { params, body, .. } => {
                let vars: Vec<_> = params
                    .iter()
                    .map(|_| TypeVar {
                        id: self.engine.next_var_id(),
                        kind: TypeVarKind::Rigid,
                    })
                    .collect();
                let body_ty = self.lower_type_expr(body);
                self.interner.intern(Type::Forall {
                    vars,
                    constraints: Vec::new(),
                    body: body_ty,
                })
            }
        }
    }

    /// Lower effect expressions to an EffectRow.
    fn lower_effects(&mut self, effects: &[japl_ast::EffectExpr]) -> EffectRow {
        if effects.is_empty() {
            return EffectRow::pure();
        }

        let mut result = Vec::new();
        let mut row_var = None;

        for eff in effects {
            match eff {
                japl_ast::EffectExpr::Named { name, args, .. } => {
                    let eff_name = name
                        .segments
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    let effect = match eff_name.as_str() {
                        "Pure" => Effect::Pure,
                        "Io" => Effect::Io,
                        "Async" => Effect::Async,
                        "Net" => Effect::Net,
                        "State" => {
                            let ty = if let Some(arg) = args.first() {
                                self.lower_type_expr(arg)
                            } else {
                                self.primitives.unit
                            };
                            Effect::State(ty)
                        }
                        "Process" => {
                            let ty = if let Some(arg) = args.first() {
                                self.lower_type_expr(arg)
                            } else {
                                self.primitives.unit
                            };
                            Effect::Process(ty)
                        }
                        "Fail" => {
                            let ty = if let Some(arg) = args.first() {
                                self.lower_type_expr(arg)
                            } else {
                                self.primitives.unit
                            };
                            Effect::Fail(ty)
                        }
                        _ => continue,
                    };
                    result.push(effect);
                }
                japl_ast::EffectExpr::Var { .. } => {
                    row_var = Some(TypeVar {
                        id: self.engine.next_var_id(),
                        kind: TypeVarKind::Unification,
                    });
                }
            }
        }

        EffectRow {
            effects: result,
            row_var,
        }
    }

    /// Instantiate a polymorphic type by replacing rigid vars with fresh unification vars.
    pub fn instantiate(&mut self, ty: TypeId) -> TypeId {
        let resolved = self.interner.resolve(ty).clone();
        match resolved {
            Type::Forall { vars, body, .. } => {
                // Create a mapping from rigid vars to fresh unification vars.
                for var in &vars {
                    let fresh = self.engine.fresh_var(&mut self.interner);
                    self.engine.subst.bind(*var, fresh);
                }
                self.engine.subst.apply(body, &mut self.interner)
            }
            _ => ty,
        }
    }

    /// Generalize a type by closing over free unification variables.
    pub fn generalize(&mut self, ty: TypeId) -> TypeId {
        let ty = self.engine.subst.apply(ty, &mut self.interner);
        let free_vars = self.free_vars(ty);
        if free_vars.is_empty() {
            return ty;
        }
        self.interner.intern(Type::Forall {
            vars: free_vars,
            constraints: Vec::new(),
            body: ty,
        })
    }

    /// Collect free unification variables in a type.
    fn free_vars(&self, ty: TypeId) -> Vec<TypeVar> {
        let mut vars = Vec::new();
        self.collect_free_vars(ty, &mut vars);
        vars.sort_by_key(|v| v.id);
        vars.dedup_by_key(|v| v.id);
        vars
    }

    fn collect_free_vars(&self, ty: TypeId, vars: &mut Vec<TypeVar>) {
        match self.interner.resolve(ty) {
            Type::Var(v) if v.kind == TypeVarKind::Unification => {
                if self.engine.subst.lookup(v).is_none() {
                    vars.push(*v);
                }
            }
            Type::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    self.collect_free_vars(*p, vars);
                }
                self.collect_free_vars(*return_type, vars);
            }
            Type::Record { fields, row_var } => {
                for (_, t) in fields {
                    self.collect_free_vars(*t, vars);
                }
                if let Some(rv) = row_var {
                    if rv.kind == TypeVarKind::Unification && self.engine.subst.lookup(rv).is_none()
                    {
                        vars.push(*rv);
                    }
                }
            }
            Type::Tuple(elems) => {
                for e in elems {
                    self.collect_free_vars(*e, vars);
                }
            }
            Type::Owned(inner) | Type::Ref(inner) => {
                self.collect_free_vars(*inner, vars);
            }
            Type::App { args, .. } => {
                for a in args {
                    self.collect_free_vars(*a, vars);
                }
            }
            Type::Forall { body, .. } => {
                self.collect_free_vars(*body, vars);
            }
            _ => {}
        }
    }

    // -- Error handling --

    fn emit_error(&mut self, error: TypeError) {
        let span = error.span();
        self.diagnostics.emit(
            Diagnostic::error(error.to_string()).with_label(span, error.to_string()),
        );
    }

    /// Check if there were any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Get all diagnostics.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics.into_diagnostics()
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

// Expose next_var_id on the engine for use by lower_type_expr.
impl UnificationEngine {
    pub fn next_var_id(&mut self) -> u32 {
        let id = self.next_var;
        self.next_var += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_ast::*;
    use japl_common::{FileId, Span};
    use smol_str::SmolStr;

    fn test_span() -> Span {
        Span::new(FileId(0), 0, 0)
    }

    fn node_id(n: u32) -> NodeId {
        NodeId(n)
    }

    #[test]
    fn infer_int_literal() {
        let mut checker = TypeChecker::new();
        let expr = Expr::IntLit {
            value: SmolStr::new("42"),
            span: test_span(),
        };
        let (ty, effects) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
        assert!(effects.is_pure());
    }

    #[test]
    fn infer_bool_literal() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BoolLit {
            value: true,
            span: test_span(),
        };
        let (ty, effects) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Bool);
        assert!(effects.is_pure());
    }

    #[test]
    fn infer_string_literal() {
        let mut checker = TypeChecker::new();
        let expr = Expr::StringLit {
            segments: vec![StringSegment::Literal(SmolStr::new("hello"))],
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::String);
    }

    #[test]
    fn infer_variable_lookup() {
        let mut checker = TypeChecker::new();
        checker.env.bind(SmolStr::new("x"), checker.primitives.int);
        let expr = Expr::Var {
            name: SmolStr::new("x"),
            id: node_id(1),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
    }

    #[test]
    fn infer_undefined_variable() {
        let mut checker = TypeChecker::new();
        let expr = Expr::Var {
            name: SmolStr::new("undefined"),
            id: node_id(1),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Error);
        assert!(checker.has_errors());
    }

    #[test]
    fn infer_let_binding() {
        let mut checker = TypeChecker::new();
        let expr = Expr::Let {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: test_span(),
            },
            ty: None,
            value: Box::new(Expr::IntLit {
                value: SmolStr::new("42"),
                span: test_span(),
            }),
            body: Box::new(Expr::Var {
                name: SmolStr::new("x"),
                id: node_id(2),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
        assert!(!checker.has_errors());
    }

    #[test]
    fn infer_lambda() {
        let mut checker = TypeChecker::new();
        let expr = Expr::Lambda {
            params: vec![Param {
                id: node_id(1),
                pattern: Pattern::Var {
                    name: SmolStr::new("x"),
                    id: node_id(2),
                    span: test_span(),
                },
                ty: None,
                ownership: Ownership::Value,
                span: test_span(),
            }],
            body: Box::new(Expr::Var {
                name: SmolStr::new("x"),
                id: node_id(3),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        match checker.interner.resolve(ty) {
            Type::Fn { params, .. } => {
                assert_eq!(params.len(), 1);
            }
            other => panic!("Expected Fn, got {:?}", other),
        }
    }

    #[test]
    fn infer_if_expr() {
        let mut checker = TypeChecker::new();
        let expr = Expr::If {
            condition: Box::new(Expr::BoolLit {
                value: true,
                span: test_span(),
            }),
            then_branch: Box::new(Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }),
            else_branch: Box::new(Expr::IntLit {
                value: SmolStr::new("2"),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
        assert!(!checker.has_errors());
    }

    #[test]
    fn infer_if_branch_mismatch() {
        let mut checker = TypeChecker::new();
        let expr = Expr::If {
            condition: Box::new(Expr::BoolLit {
                value: true,
                span: test_span(),
            }),
            then_branch: Box::new(Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }),
            else_branch: Box::new(Expr::BoolLit {
                value: false,
                span: test_span(),
            }),
            span: test_span(),
        };
        let (_, _) = checker.infer(&expr);
        assert!(checker.has_errors());
    }

    #[test]
    fn infer_record_literal() {
        let mut checker = TypeChecker::new();
        let expr = Expr::RecordLit {
            fields: vec![
                (
                    SmolStr::new("x"),
                    Expr::IntLit {
                        value: SmolStr::new("1"),
                        span: test_span(),
                    },
                ),
                (
                    SmolStr::new("y"),
                    Expr::StringLit {
                        segments: vec![StringSegment::Literal(SmolStr::new("hello"))],
                        span: test_span(),
                    },
                ),
            ],
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        match checker.interner.resolve(ty) {
            Type::Record { fields, row_var } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[1].0, "y");
                assert!(row_var.is_none());
            }
            other => panic!("Expected Record, got {:?}", other),
        }
    }

    #[test]
    fn infer_tuple_literal() {
        let mut checker = TypeChecker::new();
        let expr = Expr::TupleLit {
            elements: vec![
                Expr::IntLit {
                    value: SmolStr::new("1"),
                    span: test_span(),
                },
                Expr::BoolLit {
                    value: true,
                    span: test_span(),
                },
            ],
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        match checker.interner.resolve(ty) {
            Type::Tuple(elems) => {
                assert_eq!(elems.len(), 2);
            }
            other => panic!("Expected Tuple, got {:?}", other),
        }
    }

    #[test]
    fn infer_binop_arithmetic() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: SmolStr::new("2"),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
    }

    #[test]
    fn infer_binop_comparison() {
        let mut checker = TypeChecker::new();
        let expr = Expr::BinOp {
            op: BinOp::Lt,
            lhs: Box::new(Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: SmolStr::new("2"),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Bool);
    }

    #[test]
    fn infer_pipeline() {
        let mut checker = TypeChecker::new();
        // Register a function f: Int -> Bool
        let f_ty = checker.interner.intern(Type::Fn {
            params: vec![checker.primitives.int],
            return_type: checker.primitives.bool_ty,
            effects: EffectRow::pure(),
        });
        checker.env.bind(SmolStr::new("f"), f_ty);

        let expr = Expr::Pipeline {
            lhs: Box::new(Expr::IntLit {
                value: SmolStr::new("42"),
                span: test_span(),
            }),
            rhs: Box::new(Expr::Var {
                name: SmolStr::new("f"),
                id: node_id(1),
                span: test_span(),
            }),
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        let resolved = checker.engine.subst.apply(ty, &mut checker.interner);
        assert_eq!(checker.interner.resolve(resolved), &Type::Bool);
        assert!(!checker.has_errors());
    }

    #[test]
    fn infer_function_application() {
        let mut checker = TypeChecker::new();
        // Register a function add: (Int, Int) -> Int
        let add_ty = checker.interner.intern(Type::Fn {
            params: vec![checker.primitives.int, checker.primitives.int],
            return_type: checker.primitives.int,
            effects: EffectRow::pure(),
        });
        checker.env.bind(SmolStr::new("add"), add_ty);

        let expr = Expr::App {
            func: Box::new(Expr::Var {
                name: SmolStr::new("add"),
                id: node_id(1),
                span: test_span(),
            }),
            args: vec![
                Expr::IntLit {
                    value: SmolStr::new("1"),
                    span: test_span(),
                },
                Expr::IntLit {
                    value: SmolStr::new("2"),
                    span: test_span(),
                },
            ],
            span: test_span(),
        };
        let (ty, _) = checker.infer(&expr);
        assert_eq!(checker.interner.resolve(ty), &Type::Int);
        assert!(!checker.has_errors());
    }

    #[test]
    fn infer_application_arity_mismatch() {
        let mut checker = TypeChecker::new();
        let add_ty = checker.interner.intern(Type::Fn {
            params: vec![checker.primitives.int, checker.primitives.int],
            return_type: checker.primitives.int,
            effects: EffectRow::pure(),
        });
        checker.env.bind(SmolStr::new("add"), add_ty);

        let expr = Expr::App {
            func: Box::new(Expr::Var {
                name: SmolStr::new("add"),
                id: node_id(1),
                span: test_span(),
            }),
            args: vec![Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }],
            span: test_span(),
        };
        let (_, _) = checker.infer(&expr);
        assert!(checker.has_errors());
    }

    #[test]
    fn check_lambda_against_fn_type() {
        let mut checker = TypeChecker::new();
        let expected = checker.interner.intern(Type::Fn {
            params: vec![checker.primitives.int],
            return_type: checker.primitives.int,
            effects: EffectRow::pure(),
        });

        let expr = Expr::Lambda {
            params: vec![Param {
                id: node_id(1),
                pattern: Pattern::Var {
                    name: SmolStr::new("x"),
                    id: node_id(2),
                    span: test_span(),
                },
                ty: None,
                ownership: Ownership::Value,
                span: test_span(),
            }],
            body: Box::new(Expr::Var {
                name: SmolStr::new("x"),
                id: node_id(3),
                span: test_span(),
            }),
            span: test_span(),
        };

        let effects = checker.check(&expr, expected);
        assert!(!checker.has_errors());
        assert!(effects.is_pure());
    }

    #[test]
    fn infer_match_simple() {
        let mut checker = TypeChecker::new();
        let expr = Expr::Match {
            scrutinee: Box::new(Expr::IntLit {
                value: SmolStr::new("1"),
                span: test_span(),
            }),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal {
                        expr: Box::new(Expr::IntLit {
                            value: SmolStr::new("1"),
                            span: test_span(),
                        }),
                        span: test_span(),
                    },
                    guard: None,
                    body: Expr::StringLit {
                        segments: vec![StringSegment::Literal(SmolStr::new("one"))],
                        span: test_span(),
                    },
                    span: test_span(),
                },
                MatchArm {
                    pattern: Pattern::Wildcard { span: test_span() },
                    guard: None,
                    body: Expr::StringLit {
                        segments: vec![StringSegment::Literal(SmolStr::new("other"))],
                        span: test_span(),
                    },
                    span: test_span(),
                },
            ],
            span: test_span(),
        };

        let (ty, _) = checker.infer(&expr);
        let resolved = checker.engine.subst.apply(ty, &mut checker.interner);
        assert_eq!(checker.interner.resolve(resolved), &Type::String);
        assert!(!checker.has_errors());
    }
}

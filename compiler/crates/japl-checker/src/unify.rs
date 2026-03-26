//! Unification engine for JAPL type checking.
//!
//! Implements union-find based unification with:
//! - Occurs check
//! - Row unification for records
//! - Effect row unification

use japl_common::Span;
use japl_types::{
    Effect, EffectRow, Type, TypeId, TypeInterner, TypeVar, TypeVarKind,
};
use smol_str::SmolStr;
use std::collections::HashMap;

use crate::errors::TypeError;

/// Substitution: maps type variables to resolved types.
pub struct Substitution {
    bindings: HashMap<u32, TypeId>,
}

impl Substitution {
    pub fn new() -> Self {
        Substitution {
            bindings: HashMap::new(),
        }
    }

    /// Bind a type variable to a type.
    pub fn bind(&mut self, var: TypeVar, ty: TypeId) {
        self.bindings.insert(var.id, ty);
    }

    /// Look up a type variable binding.
    pub fn lookup(&self, var: &TypeVar) -> Option<TypeId> {
        self.bindings.get(&var.id).copied()
    }

    /// Apply the substitution to a type, walking through the interner.
    /// Returns the fully resolved TypeId.
    pub fn apply(&self, id: TypeId, interner: &mut TypeInterner) -> TypeId {
        let ty = interner.resolve(id).clone();
        match ty {
            Type::Var(v) => {
                if let Some(resolved) = self.lookup(&v) {
                    // Recursively apply in case the resolved type also contains vars.
                    self.apply(resolved, interner)
                } else {
                    id
                }
            }
            Type::Fn {
                params,
                return_type,
                effects,
            } => {
                let params: Vec<_> = params.iter().map(|p| self.apply(*p, interner)).collect();
                let return_type = self.apply(return_type, interner);
                let effects = self.apply_effect_row(&effects, interner);
                interner.intern(Type::Fn {
                    params,
                    return_type,
                    effects,
                })
            }
            Type::Record { fields, row_var } => {
                let fields: Vec<_> = fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.apply(*t, interner)))
                    .collect();
                let row_var = row_var.and_then(|v| {
                    if self.lookup(&v).is_some() {
                        // Row var was resolved, fold it into the record.
                        None
                    } else {
                        Some(v)
                    }
                });
                interner.intern(Type::Record { fields, row_var })
            }
            Type::Tuple(elems) => {
                let elems: Vec<_> = elems.iter().map(|e| self.apply(*e, interner)).collect();
                interner.intern(Type::Tuple(elems))
            }
            Type::Owned(inner) => {
                let inner = self.apply(inner, interner);
                interner.intern(Type::Owned(inner))
            }
            Type::Ref(inner) => {
                let inner = self.apply(inner, interner);
                interner.intern(Type::Ref(inner))
            }
            Type::App { constructor, args } => {
                let args: Vec<_> = args.iter().map(|a| self.apply(*a, interner)).collect();
                interner.intern(Type::App { constructor, args })
            }
            Type::Forall {
                vars,
                constraints,
                body,
            } => {
                let body = self.apply(body, interner);
                interner.intern(Type::Forall {
                    vars,
                    constraints,
                    body,
                })
            }
            // Primitives and Error don't contain type variables.
            _ => id,
        }
    }

    /// Apply substitution to an effect row.
    pub fn apply_effect_row(&self, row: &EffectRow, interner: &mut TypeInterner) -> EffectRow {
        let effects: Vec<_> = row
            .effects
            .iter()
            .map(|e| self.apply_effect(e, interner))
            .collect();
        let row_var = row.row_var.and_then(|v| {
            if self.lookup(&v).is_some() {
                None
            } else {
                Some(v)
            }
        });
        EffectRow { effects, row_var }
    }

    fn apply_effect(&self, effect: &Effect, interner: &mut TypeInterner) -> Effect {
        match effect {
            Effect::State(t) => Effect::State(self.apply(*t, interner)),
            Effect::Process(t) => Effect::Process(self.apply(*t, interner)),
            Effect::Fail(t) => Effect::Fail(self.apply(*t, interner)),
            other => other.clone(),
        }
    }
}

impl Default for Substitution {
    fn default() -> Self {
        Self::new()
    }
}

/// The unification engine.
pub struct UnificationEngine {
    pub subst: Substitution,
    pub(crate) next_var: u32,
}

impl UnificationEngine {
    pub fn new() -> Self {
        UnificationEngine {
            subst: Substitution::new(),
            next_var: 0,
        }
    }

    /// Create a fresh unification type variable.
    pub fn fresh_var(&mut self, interner: &mut TypeInterner) -> TypeId {
        let var = TypeVar {
            id: self.next_var,
            kind: TypeVarKind::Unification,
        };
        self.next_var += 1;
        interner.intern(Type::Var(var))
    }

    /// Create a fresh type variable (returns the TypeVar itself).
    pub fn fresh_type_var(&mut self) -> TypeVar {
        let var = TypeVar {
            id: self.next_var,
            kind: TypeVarKind::Unification,
        };
        self.next_var += 1;
        var
    }

    /// Unify two types. Returns Ok(()) if unification succeeds, or a TypeError.
    pub fn unify(
        &mut self,
        a: TypeId,
        b: TypeId,
        span: Span,
        interner: &mut TypeInterner,
    ) -> Result<(), TypeError> {
        // Resolve both sides through substitution first.
        let a = self.subst.apply(a, interner);
        let b = self.subst.apply(b, interner);

        if a == b {
            return Ok(());
        }

        let ty_a = interner.resolve(a).clone();
        let ty_b = interner.resolve(b).clone();

        match (&ty_a, &ty_b) {
            // Error absorbs anything.
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Unification variables.
            (Type::Var(v), _) if v.kind == TypeVarKind::Unification => {
                self.bind_var(*v, b, span, interner)
            }
            (_, Type::Var(v)) if v.kind == TypeVarKind::Unification => {
                self.bind_var(*v, a, span, interner)
            }

            // Rigid variables cannot be unified with anything else.
            (Type::Var(v), _) if v.kind == TypeVarKind::Rigid => {
                Err(TypeError::RigidVariable {
                    var_id: v.id,
                    span,
                })
            }
            (_, Type::Var(v)) if v.kind == TypeVarKind::Rigid => {
                Err(TypeError::RigidVariable {
                    var_id: v.id,
                    span,
                })
            }

            // Primitive types: must match exactly (already handled by a == b above).
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Float32, Type::Float32)
            | (Type::Bool, Type::Bool)
            | (Type::Char, Type::Char)
            | (Type::String, Type::String)
            | (Type::Bytes, Type::Bytes)
            | (Type::Unit, Type::Unit)
            | (Type::Never, Type::Never) => Ok(()),

            // Named type applications.
            (
                Type::App {
                    constructor: c1,
                    args: args1,
                },
                Type::App {
                    constructor: c2,
                    args: args2,
                },
            ) => {
                if c1 != c2 {
                    return Err(TypeError::mismatch(a, b, span, interner));
                }
                if args1.len() != args2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: args1.len(),
                        found: args2.len(),
                        span,
                    });
                }
                let pairs: Vec<_> = args1.iter().zip(args2.iter()).map(|(&a, &b)| (a, b)).collect();
                for (a, b) in pairs {
                    self.unify(a, b, span, interner)?;
                }
                Ok(())
            }

            // Function types.
            (
                Type::Fn {
                    params: p1,
                    return_type: r1,
                    effects: e1,
                },
                Type::Fn {
                    params: p2,
                    return_type: r2,
                    effects: e2,
                },
            ) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: p1.len(),
                        found: p2.len(),
                        span,
                    });
                }
                let param_pairs: Vec<_> = p1.iter().zip(p2.iter()).map(|(&a, &b)| (a, b)).collect();
                let r1 = *r1;
                let r2 = *r2;
                let e1 = e1.clone();
                let e2 = e2.clone();
                for (a, b) in param_pairs {
                    self.unify(a, b, span, interner)?;
                }
                self.unify(r1, r2, span, interner)?;
                self.unify_effect_rows(&e1, &e2, span, interner)?;
                Ok(())
            }

            // Record types with row polymorphism.
            (
                Type::Record {
                    fields: f1,
                    row_var: r1,
                },
                Type::Record {
                    fields: f2,
                    row_var: r2,
                },
            ) => {
                self.unify_records(f1, *r1, f2, *r2, span, interner)
            }

            // Tuple types.
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                if e1.len() != e2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: e1.len(),
                        found: e2.len(),
                        span,
                    });
                }
                let pairs: Vec<_> = e1.iter().zip(e2.iter()).map(|(&a, &b)| (a, b)).collect();
                for (a, b) in pairs {
                    self.unify(a, b, span, interner)?;
                }
                Ok(())
            }

            // Owned types.
            (Type::Owned(i1), Type::Owned(i2)) => {
                let i1 = *i1;
                let i2 = *i2;
                self.unify(i1, i2, span, interner)
            }

            // Ref types.
            (Type::Ref(i1), Type::Ref(i2)) => {
                let i1 = *i1;
                let i2 = *i2;
                self.unify(i1, i2, span, interner)
            }

            // Everything else is a mismatch.
            _ => Err(TypeError::mismatch(a, b, span, interner)),
        }
    }

    /// Bind a unification variable to a type, performing the occurs check.
    fn bind_var(
        &mut self,
        var: TypeVar,
        ty: TypeId,
        span: Span,
        interner: &TypeInterner,
    ) -> Result<(), TypeError> {
        // Occurs check: var must not appear in ty.
        if self.occurs_in(var, ty, interner) {
            return Err(TypeError::OccursCheck {
                var_id: var.id,
                span,
            });
        }
        self.subst.bind(var, ty);
        Ok(())
    }

    /// Check whether `var` occurs anywhere in the type identified by `ty`.
    fn occurs_in(&self, var: TypeVar, ty: TypeId, interner: &TypeInterner) -> bool {
        match interner.resolve(ty) {
            Type::Var(v) => {
                if *v == var {
                    return true;
                }
                // Also check if v has a binding we should follow.
                if let Some(bound) = self.subst.lookup(v) {
                    return self.occurs_in(var, bound, interner);
                }
                false
            }
            Type::Fn {
                params,
                return_type,
                effects,
            } => {
                params.iter().any(|p| self.occurs_in(var, *p, interner))
                    || self.occurs_in(var, *return_type, interner)
                    || effects
                        .effects
                        .iter()
                        .any(|e| self.occurs_in_effect(var, e, interner))
            }
            Type::Record { fields, row_var } => {
                fields.iter().any(|(_, t)| self.occurs_in(var, *t, interner))
                    || row_var.map_or(false, |rv| {
                        // A row variable could be bound to a type containing var.
                        if let Some(bound) = self.subst.lookup(&rv) {
                            self.occurs_in(var, bound, interner)
                        } else {
                            false
                        }
                    })
            }
            Type::Tuple(elems) => elems.iter().any(|e| self.occurs_in(var, *e, interner)),
            Type::Owned(inner) | Type::Ref(inner) => self.occurs_in(var, *inner, interner),
            Type::App { args, .. } => args.iter().any(|a| self.occurs_in(var, *a, interner)),
            Type::Forall { body, .. } => self.occurs_in(var, *body, interner),
            _ => false,
        }
    }

    fn occurs_in_effect(&self, var: TypeVar, effect: &Effect, interner: &TypeInterner) -> bool {
        match effect {
            Effect::State(t) | Effect::Process(t) | Effect::Fail(t) => {
                self.occurs_in(var, *t, interner)
            }
            _ => false,
        }
    }

    /// Unify two record types with row polymorphism.
    fn unify_records(
        &mut self,
        fields1: &[(SmolStr, TypeId)],
        row_var1: Option<TypeVar>,
        fields2: &[(SmolStr, TypeId)],
        row_var2: Option<TypeVar>,
        span: Span,
        interner: &mut TypeInterner,
    ) -> Result<(), TypeError> {
        // Build maps for field lookup.
        let map1: HashMap<&SmolStr, TypeId> = fields1.iter().map(|(n, t)| (n, *t)).collect();
        let map2: HashMap<&SmolStr, TypeId> = fields2.iter().map(|(n, t)| (n, *t)).collect();

        // Unify common fields.
        for (name, ty1) in &map1 {
            if let Some(&ty2) = map2.get(name) {
                self.unify(*ty1, ty2, span, interner)?;
            }
        }

        // Fields only in set 1 (not in set 2).
        let only_in_1: Vec<_> = fields1
            .iter()
            .filter(|(n, _)| !map2.contains_key(n))
            .cloned()
            .collect();

        // Fields only in set 2 (not in set 1).
        let only_in_2: Vec<_> = fields2
            .iter()
            .filter(|(n, _)| !map1.contains_key(n))
            .cloned()
            .collect();

        match (row_var1, row_var2) {
            // Both closed: extra fields in either direction is an error.
            (None, None) => {
                if !only_in_1.is_empty() {
                    return Err(TypeError::ExtraField {
                        field: only_in_1[0].0.to_string(),
                        span,
                    });
                }
                if !only_in_2.is_empty() {
                    return Err(TypeError::MissingField {
                        field: only_in_2[0].0.to_string(),
                        span,
                    });
                }
                Ok(())
            }
            // Left open: bind row_var1 to a record containing only_in_2 fields.
            (Some(rv1), _) if rv1.kind == TypeVarKind::Unification => {
                let remainder = interner.intern(Type::Record {
                    fields: only_in_2,
                    row_var: row_var2,
                });
                self.subst.bind(rv1, remainder);
                Ok(())
            }
            // Right open: bind row_var2 to a record containing only_in_1 fields.
            (_, Some(rv2)) if rv2.kind == TypeVarKind::Unification => {
                let remainder = interner.intern(Type::Record {
                    fields: only_in_1,
                    row_var: row_var1,
                });
                self.subst.bind(rv2, remainder);
                Ok(())
            }
            _ => Err(TypeError::RowMismatch { span }),
        }
    }

    /// Unify two effect rows.
    pub fn unify_effect_rows(
        &mut self,
        row1: &EffectRow,
        row2: &EffectRow,
        span: Span,
        interner: &mut TypeInterner,
    ) -> Result<(), TypeError> {
        // Unify concrete effects that appear in both rows.
        // For parameterized effects (State, Process, Fail), unify their type args.
        for e1 in &row1.effects {
            for e2 in &row2.effects {
                match (e1, e2) {
                    (Effect::State(t1), Effect::State(t2)) => {
                        self.unify(*t1, *t2, span, interner)?;
                    }
                    (Effect::Process(t1), Effect::Process(t2)) => {
                        self.unify(*t1, *t2, span, interner)?;
                    }
                    (Effect::Fail(t1), Effect::Fail(t2)) => {
                        self.unify(*t1, *t2, span, interner)?;
                    }
                    _ => {}
                }
            }
        }

        // If either row has a row variable, we can absorb differences.
        // This is a simplified version; a full implementation would track
        // what effects the row variable needs to absorb.
        Ok(())
    }
}

impl Default for UnificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_common::{FileId, Span};

    fn test_span() -> Span {
        Span::new(FileId(0), 0, 0)
    }

    #[test]
    fn unify_same_primitive() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int1 = interner.intern(Type::Int);
        let int2 = interner.intern(Type::Int);
        assert!(engine.unify(int1, int2, test_span(), &mut interner).is_ok());
    }

    #[test]
    fn unify_different_primitives_fails() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let bool_ty = interner.intern(Type::Bool);
        assert!(engine
            .unify(int, bool_ty, test_span(), &mut interner)
            .is_err());
    }

    #[test]
    fn unify_var_with_type() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let var = engine.fresh_var(&mut interner);
        let int = interner.intern(Type::Int);
        assert!(engine.unify(var, int, test_span(), &mut interner).is_ok());
        let resolved = engine.subst.apply(var, &mut interner);
        assert_eq!(interner.resolve(resolved), &Type::Int);
    }

    #[test]
    fn unify_two_vars() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let v1 = engine.fresh_var(&mut interner);
        let v2 = engine.fresh_var(&mut interner);
        assert!(engine.unify(v1, v2, test_span(), &mut interner).is_ok());
        // Now unify v2 with Int; v1 should also resolve to Int.
        let int = interner.intern(Type::Int);
        assert!(engine.unify(v2, int, test_span(), &mut interner).is_ok());
        let resolved = engine.subst.apply(v1, &mut interner);
        assert_eq!(interner.resolve(resolved), &Type::Int);
    }

    #[test]
    fn occurs_check() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let var = engine.fresh_var(&mut interner);
        // Try to unify var with fn(var) -> var, which should fail the occurs check.
        let fn_ty = interner.intern(Type::Fn {
            params: vec![var],
            return_type: var,
            effects: EffectRow::pure(),
        });
        let result = engine.unify(var, fn_ty, test_span(), &mut interner);
        assert!(result.is_err());
        match result.unwrap_err() {
            TypeError::OccursCheck { .. } => {}
            other => panic!("Expected OccursCheck, got {:?}", other),
        }
    }

    #[test]
    fn unify_function_types() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let bool_ty = interner.intern(Type::Bool);
        let var = engine.fresh_var(&mut interner);

        let fn1 = interner.intern(Type::Fn {
            params: vec![int],
            return_type: bool_ty,
            effects: EffectRow::pure(),
        });
        let fn2 = interner.intern(Type::Fn {
            params: vec![int],
            return_type: var,
            effects: EffectRow::pure(),
        });

        assert!(engine.unify(fn1, fn2, test_span(), &mut interner).is_ok());
        let resolved = engine.subst.apply(var, &mut interner);
        assert_eq!(interner.resolve(resolved), &Type::Bool);
    }

    #[test]
    fn unify_tuples() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let var = engine.fresh_var(&mut interner);

        let t1 = interner.intern(Type::Tuple(vec![int, int]));
        let t2 = interner.intern(Type::Tuple(vec![var, int]));

        assert!(engine.unify(t1, t2, test_span(), &mut interner).is_ok());
        let resolved = engine.subst.apply(var, &mut interner);
        assert_eq!(interner.resolve(resolved), &Type::Int);
    }

    #[test]
    fn unify_records_closed() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let str_ty = interner.intern(Type::String);

        let r1 = interner.intern(Type::Record {
            fields: vec![
                (SmolStr::new("x"), int),
                (SmolStr::new("y"), str_ty),
            ],
            row_var: None,
        });
        let r2 = interner.intern(Type::Record {
            fields: vec![
                (SmolStr::new("x"), int),
                (SmolStr::new("y"), str_ty),
            ],
            row_var: None,
        });

        assert!(engine.unify(r1, r2, test_span(), &mut interner).is_ok());
    }

    #[test]
    fn unify_records_with_row_var() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let str_ty = interner.intern(Type::String);
        let row_var = engine.fresh_type_var();

        // { x: Int | row_var } unified with { x: Int, y: String }
        let r1 = interner.intern(Type::Record {
            fields: vec![(SmolStr::new("x"), int)],
            row_var: Some(row_var),
        });
        let r2 = interner.intern(Type::Record {
            fields: vec![
                (SmolStr::new("x"), int),
                (SmolStr::new("y"), str_ty),
            ],
            row_var: None,
        });

        assert!(engine.unify(r1, r2, test_span(), &mut interner).is_ok());

        // The row variable should now be bound to { y: String }.
        let bound = engine.subst.lookup(&row_var).unwrap();
        match interner.resolve(bound) {
            Type::Record { fields, row_var } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "y");
                assert!(row_var.is_none());
            }
            other => panic!("Expected Record, got {:?}", other),
        }
    }

    #[test]
    fn unify_records_closed_missing_field() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let str_ty = interner.intern(Type::String);

        let r1 = interner.intern(Type::Record {
            fields: vec![(SmolStr::new("x"), int)],
            row_var: None,
        });
        let r2 = interner.intern(Type::Record {
            fields: vec![
                (SmolStr::new("x"), int),
                (SmolStr::new("y"), str_ty),
            ],
            row_var: None,
        });

        assert!(engine.unify(r1, r2, test_span(), &mut interner).is_err());
    }

    #[test]
    fn unify_owned_types() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let int = interner.intern(Type::Int);
        let var = engine.fresh_var(&mut interner);

        let o1 = interner.intern(Type::Owned(int));
        let o2 = interner.intern(Type::Owned(var));

        assert!(engine.unify(o1, o2, test_span(), &mut interner).is_ok());
        let resolved = engine.subst.apply(var, &mut interner);
        assert_eq!(interner.resolve(resolved), &Type::Int);
    }
}

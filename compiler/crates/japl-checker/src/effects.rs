//! Effect checking for JAPL.
//!
//! Verifies that effectful operations are only performed in contexts
//! that allow those effects. Pure functions must not contain effectful operations.

use japl_common::Span;
use japl_types::{Effect, EffectRow, TypeInterner};

use crate::errors::EffectError;
use crate::unify::UnificationEngine;

/// The effect checker verifies effect annotations against inferred effects.
pub struct EffectChecker<'a> {
    interner: &'a mut TypeInterner,
    engine: &'a mut UnificationEngine,
}

impl<'a> EffectChecker<'a> {
    pub fn new(interner: &'a mut TypeInterner, engine: &'a mut UnificationEngine) -> Self {
        EffectChecker { interner, engine }
    }

    /// Check that `callee_effects` is permitted within `context_effects`.
    ///
    /// Every concrete effect in the callee must appear in the context,
    /// or the context must have an open row variable that can absorb it.
    /// Pure is always allowed anywhere.
    pub fn check_effects(
        &mut self,
        callee_effects: &EffectRow,
        context_effects: &EffectRow,
        span: Span,
    ) -> Result<(), EffectError> {
        for effect in &callee_effects.effects {
            if matches!(effect, Effect::Pure) {
                continue;
            }

            if !self.effect_permitted(effect, context_effects) {
                if context_effects.is_pure() {
                    return Err(EffectError::PurityViolation {
                        effect: format!("{}", effect),
                        span,
                    });
                }
                return Err(EffectError::EffectNotAllowed {
                    effect: format!("{}", effect),
                    span,
                });
            }
        }
        Ok(())
    }

    /// Check whether a single effect is permitted in the given effect row.
    fn effect_permitted(&self, effect: &Effect, context: &EffectRow) -> bool {
        // Direct membership check.
        if context.contains(effect) {
            return true;
        }

        // Check for structural matches (e.g., State[_] matches State[Int]).
        for ctx_eff in &context.effects {
            if self.effects_structurally_match(effect, ctx_eff) {
                return true;
            }
        }

        // If the context has an open row variable, it can absorb any effect.
        if context.row_var.is_some() {
            return true;
        }

        // Check effect hierarchy: State < Io, Net < Io, etc.
        if self.effect_subsumed_by_hierarchy(effect, context) {
            return true;
        }

        false
    }

    /// Check if two effects match structurally (ignoring type parameters).
    fn effects_structurally_match(&self, a: &Effect, b: &Effect) -> bool {
        matches!(
            (a, b),
            (Effect::Io, Effect::Io)
                | (Effect::Async, Effect::Async)
                | (Effect::Net, Effect::Net)
                | (Effect::Pure, Effect::Pure)
                | (Effect::State(_), Effect::State(_))
                | (Effect::Process(_), Effect::Process(_))
                | (Effect::Fail(_), Effect::Fail(_))
        )
    }

    /// Check effect hierarchy:
    /// - State[s] < Io
    /// - Net < Io
    /// - Process < Async
    fn effect_subsumed_by_hierarchy(&self, effect: &Effect, context: &EffectRow) -> bool {
        match effect {
            Effect::State(_) => context.contains(&Effect::Io),
            Effect::Net => context.contains(&Effect::Io),
            Effect::Process(_) => context.contains(&Effect::Async),
            _ => false,
        }
    }

    /// Infer the combined effect row from calling a function with
    /// `callee_effects` in a context that already has `current_effects`.
    pub fn combine_effects(
        &self,
        current: &EffectRow,
        callee: &EffectRow,
    ) -> EffectRow {
        current.union(callee)
    }

    /// Check whether an effect row is a subset of another (for annotation checking).
    pub fn is_subset(
        &self,
        declared: &EffectRow,
        inferred: &EffectRow,
        span: Span,
    ) -> Result<(), EffectError> {
        // Every effect in the inferred row must appear in the declared row.
        for effect in &inferred.effects {
            if matches!(effect, Effect::Pure) {
                continue;
            }
            if !self.effect_permitted(effect, declared) {
                return Err(EffectError::RowMismatch {
                    expected: format!("{:?}", declared),
                    found: format!("{:?}", inferred),
                    span,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_common::{FileId, Span};
    use japl_types::TypeInterner;
    use crate::unify::UnificationEngine;

    fn test_span() -> Span {
        Span::new(FileId(0), 0, 0)
    }

    #[test]
    fn pure_allows_pure() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let mut checker = EffectChecker::new(&mut interner, &mut engine);

        let callee = EffectRow::pure();
        let context = EffectRow::pure();
        assert!(checker.check_effects(&callee, &context, test_span()).is_ok());
    }

    #[test]
    fn pure_rejects_io() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let mut checker = EffectChecker::new(&mut interner, &mut engine);

        let callee = EffectRow::with_effects(vec![Effect::Io]);
        let context = EffectRow::pure();
        assert!(checker.check_effects(&callee, &context, test_span()).is_err());
    }

    #[test]
    fn io_allows_io() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let mut checker = EffectChecker::new(&mut interner, &mut engine);

        let callee = EffectRow::with_effects(vec![Effect::Io]);
        let context = EffectRow::with_effects(vec![Effect::Io]);
        assert!(checker.check_effects(&callee, &context, test_span()).is_ok());
    }

    #[test]
    fn io_allows_net_via_hierarchy() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let mut checker = EffectChecker::new(&mut interner, &mut engine);

        let callee = EffectRow::with_effects(vec![Effect::Net]);
        let context = EffectRow::with_effects(vec![Effect::Io]);
        assert!(checker.check_effects(&callee, &context, test_span()).is_ok());
    }

    #[test]
    fn effect_row_union() {
        let a = EffectRow::with_effects(vec![Effect::Io]);
        let b = EffectRow::with_effects(vec![Effect::Net]);
        let combined = a.union(&b);
        assert!(combined.contains(&Effect::Io));
        assert!(combined.contains(&Effect::Net));
        assert_eq!(combined.effects.len(), 2);
    }

    #[test]
    fn effect_row_union_dedup() {
        let a = EffectRow::with_effects(vec![Effect::Io, Effect::Net]);
        let b = EffectRow::with_effects(vec![Effect::Io]);
        let combined = a.union(&b);
        assert_eq!(combined.effects.len(), 2);
    }

    #[test]
    fn net_not_allowed_in_pure() {
        let mut interner = TypeInterner::new();
        let mut engine = UnificationEngine::new();
        let mut checker = EffectChecker::new(&mut interner, &mut engine);

        let callee = EffectRow::with_effects(vec![Effect::Net]);
        let context = EffectRow::pure();
        let result = checker.check_effects(&callee, &context, test_span());
        assert!(result.is_err());
        match result.unwrap_err() {
            EffectError::PurityViolation { .. } => {}
            other => panic!("Expected PurityViolation, got {:?}", other),
        }
    }
}

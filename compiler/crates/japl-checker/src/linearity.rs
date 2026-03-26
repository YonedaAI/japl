//! Linearity checker for JAPL.
//!
//! Verifies that linear resources (`Owned<T>`, bound with `use`) are consumed
//! exactly once. Runs after type checking on the typed AST.

use japl_ast::{Expr, Pattern};
use japl_common::Span;
use japl_types::{Type, TypeId, TypeInterner};
use smol_str::SmolStr;
use std::collections::HashMap;

use crate::errors::LinearityError;

/// Status of a linear resource.
#[derive(Debug, Clone)]
pub enum ResourceStatus {
    /// Available for use.
    Available,
    /// Has been consumed (moved or closed).
    Consumed { at: Span },
    /// Has been borrowed (temporarily unavailable).
    Borrowed { at: Span, count: u32 },
}

/// A tracked linear resource.
#[derive(Debug, Clone)]
struct Resource {
    name: SmolStr,
    ty: TypeId,
    declared_at: Span,
    status: ResourceStatus,
}

/// The linearity checker.
pub struct LinearityChecker {
    /// Map from variable name to resource info.
    resources: HashMap<SmolStr, Resource>,
    /// Accumulated errors.
    errors: Vec<LinearityError>,
    /// Reference to the type interner for checking types.
    interner_snapshot: Vec<Type>,
}

impl LinearityChecker {
    pub fn new(interner: &TypeInterner) -> Self {
        // Take a snapshot of types for checking Owned/Ref.
        let mut snapshot = Vec::new();
        for i in 0..interner.len() {
            snapshot.push(interner.resolve(TypeId(i as u32)).clone());
        }
        LinearityChecker {
            resources: HashMap::new(),
            errors: Vec::new(),
            interner_snapshot: snapshot,
        }
    }

    /// Check an expression for linearity violations.
    pub fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Var { name, span, .. } => {
                self.use_resource(name, *span);
            }

            Expr::Let {
                pattern,
                value,
                body,
                span,
                ..
            } => {
                self.check_expr(value);
                // Check if the value has a linear type (Owned); if so, error.
                // The linearity checker relies on the type checker having annotated
                // types. In a full implementation, we'd check the type here.
                // For now, just check the body.
                self.check_expr(body);
            }

            Expr::Use {
                pattern,
                value,
                body,
                span,
                ..
            } => {
                self.check_expr(value);
                // Register the pattern bindings as linear resources.
                self.register_pattern_resources(pattern, *span);
                self.check_expr(body);
                // Check that all resources from this `use` are consumed.
                self.check_all_consumed(pattern);
            }

            Expr::App { func, args, span } => {
                self.check_expr(func);
                for arg in args {
                    self.check_expr(arg);
                }
            }

            Expr::BinOp { lhs, rhs, .. } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }

            Expr::UnaryOp { expr, .. } => {
                self.check_expr(expr);
            }

            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                self.check_expr(condition);
                // Both branches must consume the same set of resources.
                let snapshot = self.resources.clone();

                self.check_expr(then_branch);
                let then_state = self.resources.clone();

                // Restore and check else branch.
                self.resources = snapshot;
                self.check_expr(else_branch);
                let else_state = self.resources.clone();

                // Verify both branches consumed the same resources.
                self.check_branch_consistency(&then_state, &else_state, *span);
                // Use the then_state as the final state (they should be equivalent).
                self.resources = then_state;
            }

            Expr::Match {
                scrutinee,
                arms,
                span,
            } => {
                self.check_expr(scrutinee);

                if arms.is_empty() {
                    return;
                }

                let snapshot = self.resources.clone();
                let mut arm_states: Vec<HashMap<SmolStr, Resource>> = Vec::new();

                for arm in arms {
                    self.resources = snapshot.clone();
                    if let Some(guard) = &arm.guard {
                        self.check_expr(guard);
                    }
                    self.check_expr(&arm.body);
                    arm_states.push(self.resources.clone());
                }

                // All arms must consume the same resources.
                if arm_states.len() > 1 {
                    for i in 1..arm_states.len() {
                        self.check_branch_consistency(&arm_states[0], &arm_states[i], *span);
                    }
                }

                if let Some(final_state) = arm_states.into_iter().next() {
                    self.resources = final_state;
                }
            }

            Expr::Lambda { body, .. } => {
                // Lambdas capture resources; check the body.
                self.check_expr(body);
            }

            Expr::Block { exprs, .. } => {
                for expr in exprs {
                    self.check_expr(expr);
                }
            }

            Expr::FieldAccess { expr, .. } => {
                self.check_expr(expr);
            }

            Expr::RecordLit { fields, .. } => {
                for (_, expr) in fields {
                    self.check_expr(expr);
                }
            }

            Expr::RecordUpdate { base, updates, .. } => {
                self.check_expr(base);
                for (_, expr) in updates {
                    self.check_expr(expr);
                }
            }

            Expr::TupleLit { elements, .. } | Expr::ListLit { elements, .. } => {
                for elem in elements {
                    self.check_expr(elem);
                }
            }

            Expr::Pipeline { lhs, rhs, .. } | Expr::Compose { lhs, rhs, .. } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }

            Expr::Try { expr, .. } => {
                self.check_expr(expr);
            }

            Expr::Loop {
                bindings,
                condition,
                body,
                ..
            } => {
                for (_, expr) in bindings {
                    self.check_expr(expr);
                }
                self.check_expr(condition);
                self.check_expr(body);
            }

            Expr::Receive { arms, .. } => {
                for arm in arms {
                    self.check_expr(&arm.body);
                }
            }

            Expr::Annotation { expr, .. } => {
                self.check_expr(expr);
            }

            Expr::Continue { args, .. } => {
                for arg in args {
                    self.check_expr(arg);
                }
            }

            Expr::StringLit { segments, .. } => {
                for seg in segments {
                    if let japl_ast::StringSegment::Interpolation(expr) = seg {
                        self.check_expr(expr);
                    }
                }
            }

            // Literals don't use resources.
            Expr::IntLit { .. }
            | Expr::FloatLit { .. }
            | Expr::CharLit { .. }
            | Expr::BoolLit { .. }
            | Expr::UnitLit { .. }
            | Expr::Constructor { .. } => {}
        }
    }

    /// Record a use of a resource.
    fn use_resource(&mut self, name: &SmolStr, span: Span) {
        if let Some(resource) = self.resources.get_mut(name) {
            match &resource.status {
                ResourceStatus::Available => {
                    resource.status = ResourceStatus::Consumed { at: span };
                }
                ResourceStatus::Consumed { at } => {
                    self.errors.push(LinearityError::DoubleConsume {
                        name: name.to_string(),
                        first: *at,
                        second: span,
                    });
                }
                ResourceStatus::Borrowed { at, .. } => {
                    self.errors.push(LinearityError::ConsumedWhileBorrowed {
                        name: name.to_string(),
                        borrow_at: *at,
                        consume_at: span,
                    });
                }
            }
        }
        // Non-linear variables are not tracked.
    }

    /// Register pattern bindings as linear resources.
    fn register_pattern_resources(&mut self, pattern: &Pattern, span: Span) {
        match pattern {
            Pattern::Var { name, .. } => {
                self.resources.insert(
                    name.clone(),
                    Resource {
                        name: name.clone(),
                        ty: TypeId(0), // placeholder
                        declared_at: span,
                        status: ResourceStatus::Available,
                    },
                );
            }
            Pattern::Tuple { elements, .. } => {
                for elem in elements {
                    self.register_pattern_resources(elem, span);
                }
            }
            Pattern::Record { fields, .. } => {
                for (_, pat) in fields {
                    self.register_pattern_resources(pat, span);
                }
            }
            Pattern::Constructor { fields, .. } => {
                for field in fields {
                    self.register_pattern_resources(field, span);
                }
            }
            Pattern::As { pattern, name, .. } => {
                self.register_pattern_resources(pattern, span);
                self.resources.insert(
                    name.clone(),
                    Resource {
                        name: name.clone(),
                        ty: TypeId(0),
                        declared_at: span,
                        status: ResourceStatus::Available,
                    },
                );
            }
            Pattern::Wildcard { .. }
            | Pattern::Literal { .. }
            | Pattern::Pin { .. }
            | Pattern::List { .. }
            | Pattern::Or { .. } => {}
        }
    }

    /// Check that all resources bound by a pattern have been consumed.
    fn check_all_consumed(&mut self, pattern: &Pattern) {
        let names = self.collect_pattern_names(pattern);
        for name in names {
            if let Some(resource) = self.resources.get(&name) {
                match &resource.status {
                    ResourceStatus::Available => {
                        self.errors.push(LinearityError::NotConsumed {
                            name: name.to_string(),
                            declared_at: resource.declared_at,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    /// Collect variable names bound by a pattern.
    fn collect_pattern_names(&self, pattern: &Pattern) -> Vec<SmolStr> {
        let mut names = Vec::new();
        match pattern {
            Pattern::Var { name, .. } => names.push(name.clone()),
            Pattern::Tuple { elements, .. } => {
                for elem in elements {
                    names.extend(self.collect_pattern_names(elem));
                }
            }
            Pattern::Record { fields, .. } => {
                for (_, pat) in fields {
                    names.extend(self.collect_pattern_names(pat));
                }
            }
            Pattern::Constructor { fields, .. } => {
                for field in fields {
                    names.extend(self.collect_pattern_names(field));
                }
            }
            Pattern::As { pattern, name, .. } => {
                names.extend(self.collect_pattern_names(pattern));
                names.push(name.clone());
            }
            _ => {}
        }
        names
    }

    /// Check that two branch states consumed the same set of resources.
    fn check_branch_consistency(
        &mut self,
        state1: &HashMap<SmolStr, Resource>,
        state2: &HashMap<SmolStr, Resource>,
        span: Span,
    ) {
        for (name, res1) in state1 {
            if let Some(res2) = state2.get(name) {
                let consumed1 = matches!(&res1.status, ResourceStatus::Consumed { .. });
                let consumed2 = matches!(&res2.status, ResourceStatus::Consumed { .. });
                if consumed1 != consumed2 {
                    self.errors.push(LinearityError::BranchMismatch { span });
                }
            }
        }
    }

    /// Borrow a resource (for `ref` parameters).
    pub fn borrow_resource(&mut self, name: &SmolStr, span: Span) {
        if let Some(resource) = self.resources.get_mut(name) {
            match &resource.status {
                ResourceStatus::Available => {
                    resource.status = ResourceStatus::Borrowed { at: span, count: 1 };
                }
                ResourceStatus::Borrowed { at, count } => {
                    resource.status = ResourceStatus::Borrowed {
                        at: *at,
                        count: count + 1,
                    };
                }
                ResourceStatus::Consumed { at } => {
                    self.errors.push(LinearityError::UseAfterMove {
                        name: name.to_string(),
                        consumed_at: *at,
                        use_at: span,
                    });
                }
            }
        }
    }

    /// Return a borrowed resource (end of borrow region).
    pub fn return_borrow(&mut self, name: &SmolStr) {
        if let Some(resource) = self.resources.get_mut(name) {
            match &resource.status {
                ResourceStatus::Borrowed { count, .. } if *count <= 1 => {
                    resource.status = ResourceStatus::Available;
                }
                ResourceStatus::Borrowed { at, count } => {
                    resource.status = ResourceStatus::Borrowed {
                        at: *at,
                        count: count - 1,
                    };
                }
                _ => {}
            }
        }
    }

    /// Return all accumulated errors.
    pub fn into_errors(self) -> Vec<LinearityError> {
        self.errors
    }

    /// Check if there were any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Return a reference to accumulated errors.
    pub fn errors(&self) -> &[LinearityError] {
        &self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_ast::*;
    use japl_common::{FileId, Span};
    use japl_types::TypeInterner;

    fn test_span() -> Span {
        Span::new(FileId(0), 0, 0)
    }

    fn span_at(offset: u32) -> Span {
        Span::new(FileId(0), offset, offset + 1)
    }

    fn node_id(n: u32) -> NodeId {
        NodeId(n)
    }

    #[test]
    fn resource_used_once_ok() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // use x = acquire()
        // consume(x)
        let expr = Expr::Use {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: span_at(0),
            },
            ty: None,
            value: Box::new(Expr::UnitLit { span: span_at(5) }),
            body: Box::new(Expr::Var {
                name: SmolStr::new("x"),
                id: node_id(2),
                span: span_at(10),
            }),
            span: test_span(),
        };

        checker.check_expr(&expr);
        assert!(!checker.has_errors());
    }

    #[test]
    fn resource_double_consume_rejected() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // use x = acquire()
        // block { consume(x); consume(x) }
        let expr = Expr::Use {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: span_at(0),
            },
            ty: None,
            value: Box::new(Expr::UnitLit { span: span_at(5) }),
            body: Box::new(Expr::Block {
                exprs: vec![
                    Expr::Var {
                        name: SmolStr::new("x"),
                        id: node_id(2),
                        span: span_at(10),
                    },
                    Expr::Var {
                        name: SmolStr::new("x"),
                        id: node_id(3),
                        span: span_at(20),
                    },
                ],
                span: test_span(),
            }),
            span: test_span(),
        };

        checker.check_expr(&expr);
        assert!(checker.has_errors());
        assert!(checker.errors().iter().any(|e| matches!(e, LinearityError::DoubleConsume { .. })));
    }

    #[test]
    fn resource_not_consumed_rejected() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // use x = acquire()
        // ()
        let expr = Expr::Use {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: span_at(0),
            },
            ty: None,
            value: Box::new(Expr::UnitLit { span: span_at(5) }),
            body: Box::new(Expr::UnitLit { span: span_at(10) }),
            span: test_span(),
        };

        checker.check_expr(&expr);
        assert!(checker.has_errors());
        assert!(checker
            .errors()
            .iter()
            .any(|e| matches!(e, LinearityError::NotConsumed { .. })));
    }

    #[test]
    fn branch_mismatch_rejected() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // use x = acquire()
        // if True then consume(x) else ()
        let expr = Expr::Use {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: span_at(0),
            },
            ty: None,
            value: Box::new(Expr::UnitLit { span: span_at(5) }),
            body: Box::new(Expr::If {
                condition: Box::new(Expr::BoolLit {
                    value: true,
                    span: span_at(10),
                }),
                then_branch: Box::new(Expr::Var {
                    name: SmolStr::new("x"),
                    id: node_id(2),
                    span: span_at(15),
                }),
                else_branch: Box::new(Expr::UnitLit { span: span_at(20) }),
                span: span_at(10),
            }),
            span: test_span(),
        };

        checker.check_expr(&expr);
        assert!(checker.has_errors());
        assert!(checker
            .errors()
            .iter()
            .any(|e| matches!(e, LinearityError::BranchMismatch { .. })));
    }

    #[test]
    fn branch_both_consume_ok() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // use x = acquire()
        // if True then consume(x) else consume(x)
        let expr = Expr::Use {
            pattern: Pattern::Var {
                name: SmolStr::new("x"),
                id: node_id(1),
                span: span_at(0),
            },
            ty: None,
            value: Box::new(Expr::UnitLit { span: span_at(5) }),
            body: Box::new(Expr::If {
                condition: Box::new(Expr::BoolLit {
                    value: true,
                    span: span_at(10),
                }),
                then_branch: Box::new(Expr::Var {
                    name: SmolStr::new("x"),
                    id: node_id(2),
                    span: span_at(15),
                }),
                else_branch: Box::new(Expr::Var {
                    name: SmolStr::new("x"),
                    id: node_id(3),
                    span: span_at(20),
                }),
                span: span_at(10),
            }),
            span: test_span(),
        };

        checker.check_expr(&expr);
        assert!(!checker.has_errors());
    }

    #[test]
    fn borrow_then_consume_ok() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        // Register a resource manually.
        checker.resources.insert(
            SmolStr::new("buf"),
            Resource {
                name: SmolStr::new("buf"),
                ty: TypeId(0),
                declared_at: span_at(0),
                status: ResourceStatus::Available,
            },
        );

        // Borrow it.
        checker.borrow_resource(&SmolStr::new("buf"), span_at(10));
        assert!(matches!(
            checker.resources.get("buf").unwrap().status,
            ResourceStatus::Borrowed { count: 1, .. }
        ));

        // Return the borrow.
        checker.return_borrow(&SmolStr::new("buf"));
        assert!(matches!(
            checker.resources.get("buf").unwrap().status,
            ResourceStatus::Available
        ));

        // Consume it.
        checker.use_resource(&SmolStr::new("buf"), span_at(20));
        assert!(matches!(
            checker.resources.get("buf").unwrap().status,
            ResourceStatus::Consumed { .. }
        ));

        assert!(!checker.has_errors());
    }

    #[test]
    fn use_after_move_via_borrow() {
        let interner = TypeInterner::new();
        let mut checker = LinearityChecker::new(&interner);

        checker.resources.insert(
            SmolStr::new("buf"),
            Resource {
                name: SmolStr::new("buf"),
                ty: TypeId(0),
                declared_at: span_at(0),
                status: ResourceStatus::Available,
            },
        );

        // Consume.
        checker.use_resource(&SmolStr::new("buf"), span_at(10));

        // Try to borrow after move.
        checker.borrow_resource(&SmolStr::new("buf"), span_at(20));

        assert!(checker.has_errors());
        assert!(checker
            .errors()
            .iter()
            .any(|e| matches!(e, LinearityError::UseAfterMove { .. })));
    }
}

//! AST-to-IR lowering pass.
//!
//! Translates the japl-ast types into the simplified IR representation
//! suitable for tree-walking interpretation.

use std::sync::Arc;

use japl_ast::{self as ast, SourceFile};
use japl_runtime::Value;

use crate::{
    IrBinOp, IrExpr, IrFnDef, IrPattern, IrProgram, IrTypeDef, IrUnaryOp, IrVariant, LowerError,
};

/// Lower a parsed AST into an IR program.
pub fn lower(source_file: &SourceFile) -> Result<IrProgram, LowerError> {
    let mut functions = Vec::new();
    let mut type_defs = Vec::new();

    for item in &source_file.items {
        match item {
            ast::Item::FnDef(fn_def) => {
                functions.push(lower_fn_def(fn_def)?);
            }
            ast::Item::TypeDef(type_def) => {
                if let Some(ir_td) = lower_type_def(type_def)? {
                    type_defs.push(ir_td);
                }
            }
            // Other top-level items are not yet supported in the interpreter
            _ => {}
        }
    }

    Ok(IrProgram {
        functions,
        type_defs,
    })
}

/// Lower a function definition.
fn lower_fn_def(fn_def: &ast::FnDef) -> Result<IrFnDef, LowerError> {
    let name = fn_def.name.to_string();
    let params: Vec<String> = fn_def
        .params
        .iter()
        .map(|p| param_name(&p.pattern))
        .collect();
    let body = lower_expr(&fn_def.body)?;
    Ok(IrFnDef { name, params, body })
}

/// Extract the name from a parameter pattern.
fn param_name(pattern: &ast::Pattern) -> String {
    match pattern {
        ast::Pattern::Var { name, .. } => name.to_string(),
        ast::Pattern::Wildcard { .. } => "_".to_string(),
        _ => "_".to_string(),
    }
}

/// Lower a type definition (sum types become constructor registrations).
fn lower_type_def(type_def: &ast::TypeDef) -> Result<Option<IrTypeDef>, LowerError> {
    match &type_def.body {
        ast::TypeBody::Sum(variants) => {
            let ir_variants = variants
                .iter()
                .map(|v| IrVariant {
                    name: v.name.to_string(),
                    arity: v.fields.len(),
                })
                .collect();
            Ok(Some(IrTypeDef {
                name: type_def.name.to_string(),
                variants: ir_variants,
            }))
        }
        _ => Ok(None),
    }
}

/// Lower an AST expression to IR.
fn lower_expr(expr: &ast::Expr) -> Result<IrExpr, LowerError> {
    match expr {
        ast::Expr::IntLit { value, .. } => {
            let n: i64 = value
                .parse()
                .map_err(|_| LowerError::InvalidLiteral(value.to_string()))?;
            Ok(IrExpr::Lit(Value::Int(n)))
        }

        ast::Expr::FloatLit { value, .. } => {
            let f: f64 = value
                .parse()
                .map_err(|_| LowerError::InvalidLiteral(value.to_string()))?;
            Ok(IrExpr::Lit(Value::Float(f)))
        }

        ast::Expr::StringLit { segments, .. } => {
            // For simple string literals (single literal segment), produce a Lit.
            // For interpolated strings, produce concatenation.
            if segments.len() == 1 {
                if let ast::StringSegment::Literal(s) = &segments[0] {
                    return Ok(IrExpr::Lit(Value::String(Arc::from(s.as_str()))));
                }
            }
            // Build a chain of string concatenations
            let mut parts = Vec::new();
            for seg in segments {
                match seg {
                    ast::StringSegment::Literal(s) => {
                        parts.push(IrExpr::Lit(Value::String(Arc::from(s.as_str()))));
                    }
                    ast::StringSegment::Interpolation(e) => {
                        // Wrap in a show() call to convert to string
                        let inner = lower_expr(e)?;
                        parts.push(IrExpr::App(
                            Box::new(IrExpr::Var("show".to_string())),
                            vec![inner],
                        ));
                    }
                }
            }
            // Fold parts with Concat
            let mut result = parts.remove(0);
            for part in parts {
                result = IrExpr::Concat(Box::new(result), Box::new(part));
            }
            Ok(result)
        }

        ast::Expr::CharLit { value, .. } => {
            Ok(IrExpr::Lit(Value::String(Arc::from(value.to_string().as_str()))))
        }

        ast::Expr::BoolLit { value, .. } => Ok(IrExpr::Lit(Value::Bool(*value))),

        ast::Expr::UnitLit { .. } => Ok(IrExpr::Lit(Value::Unit)),

        ast::Expr::Var { name, .. } => Ok(IrExpr::Var(name.to_string())),

        ast::Expr::Constructor { name, .. } => {
            // A bare constructor reference (no args yet).
            // Use the last segment as the constructor name.
            let ctor_name = name
                .segments
                .last()
                .map(|s| s.to_string())
                .unwrap_or_default();
            Ok(IrExpr::Var(ctor_name))
        }

        ast::Expr::FieldAccess { expr, field, .. } => {
            let ir_expr = lower_expr(expr)?;
            Ok(IrExpr::FieldAccess(Box::new(ir_expr), field.to_string()))
        }

        ast::Expr::App { func, args, .. } => {
            // Check if func is a constructor
            if let ast::Expr::Constructor { name, .. } = func.as_ref() {
                let ctor_name = name
                    .segments
                    .last()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let ir_args: Vec<IrExpr> = args
                    .iter()
                    .map(lower_expr)
                    .collect::<Result<_, _>>()?;
                return Ok(IrExpr::Constructor(ctor_name, ir_args));
            }
            let ir_func = lower_expr(func)?;
            let ir_args: Vec<IrExpr> = args
                .iter()
                .map(lower_expr)
                .collect::<Result<_, _>>()?;
            Ok(IrExpr::App(Box::new(ir_func), ir_args))
        }

        ast::Expr::BinOp { op, lhs, rhs, .. } => {
            let ir_lhs = lower_expr(lhs)?;
            let ir_rhs = lower_expr(rhs)?;
            match op {
                ast::BinOp::Concat => Ok(IrExpr::Concat(Box::new(ir_lhs), Box::new(ir_rhs))),
                ast::BinOp::Append => {
                    // list append
                    Ok(IrExpr::App(
                        Box::new(IrExpr::Var("list_append".to_string())),
                        vec![ir_lhs, ir_rhs],
                    ))
                }
                _ => {
                    let ir_op = lower_binop(*op)?;
                    Ok(IrExpr::BinOp(ir_op, Box::new(ir_lhs), Box::new(ir_rhs)))
                }
            }
        }

        ast::Expr::UnaryOp { op, expr, .. } => {
            let ir_expr = lower_expr(expr)?;
            let ir_op = match op {
                ast::UnaryOp::Neg => IrUnaryOp::Neg,
                ast::UnaryOp::Not => IrUnaryOp::Not,
            };
            Ok(IrExpr::UnaryOp(ir_op, Box::new(ir_expr)))
        }

        ast::Expr::Pipeline { lhs, rhs, .. } => {
            let ir_lhs = lower_expr(lhs)?;
            let ir_rhs = lower_expr(rhs)?;
            Ok(IrExpr::Pipeline(Box::new(ir_lhs), Box::new(ir_rhs)))
        }

        ast::Expr::Compose { lhs, rhs, .. } => {
            // f >> g becomes fn(x) -> g(f(x))
            let ir_lhs = lower_expr(lhs)?;
            let ir_rhs = lower_expr(rhs)?;
            Ok(IrExpr::Lambda(
                vec!["__x".to_string()],
                Box::new(IrExpr::App(
                    Box::new(ir_rhs),
                    vec![IrExpr::App(
                        Box::new(ir_lhs),
                        vec![IrExpr::Var("__x".to_string())],
                    )],
                )),
            ))
        }

        ast::Expr::Lambda { params, body, .. } => {
            let ir_params: Vec<String> = params
                .iter()
                .map(|p| param_name(&p.pattern))
                .collect();
            let ir_body = lower_expr(body)?;
            Ok(IrExpr::Lambda(ir_params, Box::new(ir_body)))
        }

        ast::Expr::Let {
            pattern,
            value,
            body,
            ..
        } => {
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;
            let ir_body = lower_expr(body)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }

        ast::Expr::Use {
            pattern,
            value,
            body,
            ..
        } => {
            // For the interpreter, `use` behaves like `let`
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;
            let ir_body = lower_expr(body)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }

        ast::Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let ir_cond = lower_expr(condition)?;
            let ir_then = lower_expr(then_branch)?;
            let ir_else = lower_expr(else_branch)?;
            Ok(IrExpr::If(
                Box::new(ir_cond),
                Box::new(ir_then),
                Box::new(ir_else),
            ))
        }

        ast::Expr::Match {
            scrutinee, arms, ..
        } => {
            let ir_scrutinee = lower_expr(scrutinee)?;
            let ir_arms: Vec<(IrPattern, Option<IrExpr>, IrExpr)> = arms
                .iter()
                .map(|arm| {
                    let pat = lower_pattern(&arm.pattern)?;
                    let guard = arm
                        .guard
                        .as_ref()
                        .map(|g| lower_expr(g))
                        .transpose()?;
                    let body = lower_expr(&arm.body)?;
                    Ok((pat, guard, body))
                })
                .collect::<Result<_, LowerError>>()?;
            Ok(IrExpr::Match(Box::new(ir_scrutinee), ir_arms))
        }

        ast::Expr::Block { exprs, .. } => {
            if exprs.len() == 1 {
                return lower_expr(&exprs[0]);
            }
            lower_block(exprs)
        }

        ast::Expr::RecordLit { fields, .. } => {
            let ir_fields: Vec<(String, IrExpr)> = fields
                .iter()
                .map(|(name, expr)| {
                    let ir_expr = lower_expr(expr)?;
                    Ok((name.to_string(), ir_expr))
                })
                .collect::<Result<_, LowerError>>()?;
            Ok(IrExpr::Record(ir_fields))
        }

        ast::Expr::RecordUpdate {
            base, updates, ..
        } => {
            // Lower record update as: creating a new record by merging base with updates.
            // We'll represent this as a special builtin call.
            let ir_base = lower_expr(base)?;
            let ir_updates: Vec<(String, IrExpr)> = updates
                .iter()
                .map(|(name, expr)| {
                    let ir_expr = lower_expr(expr)?;
                    Ok((name.to_string(), ir_expr))
                })
                .collect::<Result<_, LowerError>>()?;
            // Desugar as: let __base = base in { ...base_fields, ...updates }
            // For the interpreter, we'll use a special App to a builtin
            Ok(IrExpr::App(
                Box::new(IrExpr::Var("__record_update".to_string())),
                vec![
                    ir_base,
                    IrExpr::Record(ir_updates),
                ],
            ))
        }

        ast::Expr::ListLit { elements, .. } => {
            let ir_elems: Vec<IrExpr> = elements
                .iter()
                .map(lower_expr)
                .collect::<Result<_, _>>()?;
            Ok(IrExpr::List(ir_elems))
        }

        ast::Expr::TupleLit { elements, .. } => {
            let ir_elems: Vec<IrExpr> = elements
                .iter()
                .map(lower_expr)
                .collect::<Result<_, _>>()?;
            Ok(IrExpr::Tuple(ir_elems))
        }

        ast::Expr::Try { expr, .. } => {
            // For the interpreter, just evaluate the inner expression
            lower_expr(expr)
        }

        ast::Expr::Loop { .. } => Err(LowerError::Unsupported("loop expressions".to_string())),
        ast::Expr::Continue { .. } => {
            Err(LowerError::Unsupported("continue expressions".to_string()))
        }
        ast::Expr::Receive { .. } => {
            Err(LowerError::Unsupported("receive expressions (yet)".to_string()))
        }
        ast::Expr::Annotation { expr, .. } => {
            // Type annotations are erased at runtime
            lower_expr(expr)
        }
    }
}

/// Lower a block of expressions, properly nesting let bindings.
///
/// When a block contains let bindings, all subsequent expressions become
/// the body of that let, ensuring proper scoping. For example:
///   [let x = 1, let y = 2, x + y]
/// becomes:
///   Let("x", 1, Let("y", 2, x + y))
fn lower_block(exprs: &[ast::Expr]) -> Result<IrExpr, LowerError> {
    if exprs.is_empty() {
        return Ok(IrExpr::Lit(japl_runtime::Value::Unit));
    }
    if exprs.len() == 1 {
        return lower_expr(&exprs[0]);
    }

    // Process from right to left to build nested lets
    lower_block_from(exprs, 0)
}

/// Recursively lower a block starting from index `start`.
fn lower_block_from(exprs: &[ast::Expr], start: usize) -> Result<IrExpr, LowerError> {
    if start >= exprs.len() {
        return Ok(IrExpr::Lit(japl_runtime::Value::Unit));
    }
    if start == exprs.len() - 1 {
        return lower_expr(&exprs[start]);
    }

    let current = &exprs[start];

    // Check if current expression is a Let binding
    // If so, extract the let and make the rest of the block its body
    match current {
        ast::Expr::Let { pattern, value, body, .. } => {
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;

            // The parser already set a body for this let. We need to combine
            // the existing body with the remaining block expressions.
            // The existing body might itself contain nested lets from the parser.
            //
            // Collect all expressions: the existing body (which may be a chain of
            // lets) plus the remaining block expressions.
            let existing_body_exprs = flatten_let_chain_tail(body);
            let remaining = &exprs[start + 1..];

            // Combine: existing body expressions + remaining block expressions
            let mut all_body_exprs: Vec<&ast::Expr> = Vec::new();
            all_body_exprs.extend(existing_body_exprs.iter());
            all_body_exprs.extend(remaining.iter());

            let ir_body = lower_block_refs(&all_body_exprs)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }
        ast::Expr::Use { pattern, value, body, .. } => {
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;

            let existing_body_exprs = flatten_let_chain_tail(body);
            let remaining = &exprs[start + 1..];

            let mut all_body_exprs: Vec<&ast::Expr> = Vec::new();
            all_body_exprs.extend(existing_body_exprs.iter());
            all_body_exprs.extend(remaining.iter());

            let ir_body = lower_block_refs(&all_body_exprs)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }
        _ => {
            // Non-let expression in a block: evaluate it, then continue
            let ir_current = lower_expr(current)?;
            let ir_rest = lower_block_from(exprs, start + 1)?;
            Ok(IrExpr::Block(vec![ir_current, ir_rest]))
        }
    }
}

/// Lower a block given as a slice of expression references.
fn lower_block_refs(exprs: &[&ast::Expr]) -> Result<IrExpr, LowerError> {
    if exprs.is_empty() {
        return Ok(IrExpr::Lit(japl_runtime::Value::Unit));
    }
    if exprs.len() == 1 {
        return lower_expr(exprs[0]);
    }

    let current = exprs[0];
    match current {
        ast::Expr::Let { pattern, value, body, .. } => {
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;

            let existing_body_exprs = flatten_let_chain_tail(body);
            let remaining = &exprs[1..];

            let mut all_body_exprs: Vec<&ast::Expr> = Vec::new();
            all_body_exprs.extend(existing_body_exprs.iter());
            all_body_exprs.extend(remaining.iter());

            let ir_body = lower_block_refs(&all_body_exprs)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }
        ast::Expr::Use { pattern, value, body, .. } => {
            let name = param_name(pattern);
            let ir_value = lower_expr(value)?;

            let existing_body_exprs = flatten_let_chain_tail(body);
            let remaining = &exprs[1..];

            let mut all_body_exprs: Vec<&ast::Expr> = Vec::new();
            all_body_exprs.extend(existing_body_exprs.iter());
            all_body_exprs.extend(remaining.iter());

            let ir_body = lower_block_refs(&all_body_exprs)?;
            Ok(IrExpr::Let(name, Box::new(ir_value), Box::new(ir_body)))
        }
        _ => {
            let ir_current = lower_expr(current)?;
            let ir_rest = lower_block_refs(&exprs[1..])?;
            Ok(IrExpr::Block(vec![ir_current, ir_rest]))
        }
    }
}

/// Given a let-chain's tail body, flatten it into a list of expressions.
/// This extracts the innermost (non-let) expression from a chain of nested lets.
///
/// For example, for `let a = 1 in (let b = 2 in (expr))`, calling this on
/// the entire Let returns the chain as [Let(a, 1, Let(b, 2, expr))].
/// But when called on just the body of the outermost let, it returns [Let(b, 2, expr)].
///
/// We only flatten if the body is itself a let chain that was already processed.
/// Actually, the simplest approach: just return the body expression as-is in a vec.
fn flatten_let_chain_tail(body: &ast::Expr) -> Vec<&ast::Expr> {
    vec![body]
}

/// Lower a binary operator.
fn lower_binop(op: ast::BinOp) -> Result<IrBinOp, LowerError> {
    match op {
        ast::BinOp::Add => Ok(IrBinOp::Add),
        ast::BinOp::Sub => Ok(IrBinOp::Sub),
        ast::BinOp::Mul => Ok(IrBinOp::Mul),
        ast::BinOp::Div => Ok(IrBinOp::Div),
        ast::BinOp::Mod => Ok(IrBinOp::Mod),
        ast::BinOp::Eq => Ok(IrBinOp::Eq),
        ast::BinOp::Neq => Ok(IrBinOp::Neq),
        ast::BinOp::Lt => Ok(IrBinOp::Lt),
        ast::BinOp::Gt => Ok(IrBinOp::Gt),
        ast::BinOp::LtEq => Ok(IrBinOp::LtEq),
        ast::BinOp::GtEq => Ok(IrBinOp::GtEq),
        ast::BinOp::And => Ok(IrBinOp::And),
        ast::BinOp::Or => Ok(IrBinOp::Or),
        ast::BinOp::Concat | ast::BinOp::Append => {
            // These should be handled before calling lower_binop
            unreachable!()
        }
    }
}

/// Lower an AST pattern to an IR pattern.
fn lower_pattern(pattern: &ast::Pattern) -> Result<IrPattern, LowerError> {
    match pattern {
        ast::Pattern::Wildcard { .. } => Ok(IrPattern::Wildcard),

        ast::Pattern::Var { name, .. } => Ok(IrPattern::Var(name.to_string())),

        ast::Pattern::Pin { name, .. } => {
            // Pin patterns compare against an existing binding.
            // For simplicity, we treat them as variable patterns in the IR
            // and let the interpreter handle the pinning semantics.
            Ok(IrPattern::Var(format!("^{}", name)))
        }

        ast::Pattern::Constructor { name, fields, .. } => {
            let ctor_name = name
                .segments
                .last()
                .map(|s| s.to_string())
                .unwrap_or_default();
            let ir_fields: Vec<IrPattern> = fields
                .iter()
                .map(lower_pattern)
                .collect::<Result<_, _>>()?;
            Ok(IrPattern::Constructor(ctor_name, ir_fields))
        }

        ast::Pattern::Literal { expr, .. } => {
            // Extract the literal value from the expression
            match expr.as_ref() {
                ast::Expr::IntLit { value, .. } => {
                    let n: i64 = value
                        .parse()
                        .map_err(|_| LowerError::InvalidLiteral(value.to_string()))?;
                    Ok(IrPattern::Literal(Value::Int(n)))
                }
                ast::Expr::FloatLit { value, .. } => {
                    let f: f64 = value
                        .parse()
                        .map_err(|_| LowerError::InvalidLiteral(value.to_string()))?;
                    Ok(IrPattern::Literal(Value::Float(f)))
                }
                ast::Expr::StringLit { segments, .. } => {
                    if let Some(ast::StringSegment::Literal(s)) = segments.first() {
                        Ok(IrPattern::Literal(Value::String(Arc::from(s.as_str()))))
                    } else {
                        Err(LowerError::Unsupported(
                            "interpolated string in pattern".to_string(),
                        ))
                    }
                }
                ast::Expr::BoolLit { value, .. } => Ok(IrPattern::Literal(Value::Bool(*value))),
                ast::Expr::UnitLit { .. } => Ok(IrPattern::Literal(Value::Unit)),
                _ => Err(LowerError::Unsupported(
                    "complex expression in literal pattern".to_string(),
                )),
            }
        }

        ast::Pattern::Record { fields, .. } => {
            let ir_fields: Vec<(String, IrPattern)> = fields
                .iter()
                .map(|(name, pat)| {
                    let ir_pat = lower_pattern(pat)?;
                    Ok((name.to_string(), ir_pat))
                })
                .collect::<Result<_, LowerError>>()?;
            Ok(IrPattern::Record(ir_fields))
        }

        ast::Pattern::List { elements, rest, .. } => {
            let ir_elems: Vec<IrPattern> = elements
                .iter()
                .map(lower_pattern)
                .collect::<Result<_, _>>()?;
            let ir_rest = rest
                .as_ref()
                .map(|r| lower_pattern(r).map(Box::new))
                .transpose()?;
            Ok(IrPattern::List(ir_elems, ir_rest))
        }

        ast::Pattern::Tuple { elements, .. } => {
            let ir_elems: Vec<IrPattern> = elements
                .iter()
                .map(lower_pattern)
                .collect::<Result<_, _>>()?;
            Ok(IrPattern::Tuple(ir_elems))
        }

        ast::Pattern::Or { patterns, .. } => {
            let ir_pats: Vec<IrPattern> = patterns
                .iter()
                .map(lower_pattern)
                .collect::<Result<_, _>>()?;
            Ok(IrPattern::Or(ir_pats))
        }

        ast::Pattern::As {
            pattern, name, ..
        } => {
            // As patterns: match the inner pattern, also bind to name.
            // For simplicity, we lower as just the variable binding.
            // A full implementation would need a compound pattern.
            let _inner = lower_pattern(pattern)?;
            Ok(IrPattern::Var(name.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use japl_common::FileId;
    use japl_parser::parse;

    fn parse_and_lower(source: &str) -> Result<IrProgram, String> {
        let (ast, diags) = parse(source, FileId(0));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == japl_common::Severity::Error)
            .collect();
        if !errors.is_empty() {
            return Err(format!("Parse errors: {:?}", errors));
        }
        lower(&ast).map_err(|e| format!("Lower error: {}", e))
    }

    #[test]
    fn test_lower_simple_fn() {
        let prog = parse_and_lower("fn add(x: Int, y: Int) -> Int = x + y").unwrap();
        assert_eq!(prog.functions.len(), 1);
        assert_eq!(prog.functions[0].name, "add");
        assert_eq!(prog.functions[0].params, vec!["x", "y"]);
    }

    #[test]
    fn test_lower_let_binding() {
        let prog = parse_and_lower(
            "fn main() -> Int =\n  let x = 42\n  x",
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
        match &prog.functions[0].body {
            IrExpr::Let(name, _, _) => assert_eq!(name, "x"),
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_if_expr() {
        let prog = parse_and_lower(
            "fn abs(n: Int) -> Int = if n < 0 then 0 - n else n",
        )
        .unwrap();
        assert_eq!(prog.functions.len(), 1);
        match &prog.functions[0].body {
            IrExpr::If(_, _, _) => {}
            other => panic!("expected If, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_string_lit() {
        let prog = parse_and_lower(
            r#"fn hello() -> String = "hello world""#,
        )
        .unwrap();
        match &prog.functions[0].body {
            IrExpr::Lit(Value::String(s)) => assert_eq!(&**s, "hello world"),
            other => panic!("expected String lit, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_list() {
        let prog = parse_and_lower(
            "fn nums() -> List[Int] = [1, 2, 3]",
        )
        .unwrap();
        match &prog.functions[0].body {
            IrExpr::List(elems) => assert_eq!(elems.len(), 3),
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_lower_type_def() {
        let prog = parse_and_lower(
            "type Option[a] =\n  | Some(a)\n  | None",
        )
        .unwrap();
        assert_eq!(prog.type_defs.len(), 1);
        assert_eq!(prog.type_defs[0].name, "Option");
        assert_eq!(prog.type_defs[0].variants.len(), 2);
        assert_eq!(prog.type_defs[0].variants[0].name, "Some");
        assert_eq!(prog.type_defs[0].variants[0].arity, 1);
        assert_eq!(prog.type_defs[0].variants[1].name, "None");
        assert_eq!(prog.type_defs[0].variants[1].arity, 0);
    }
}

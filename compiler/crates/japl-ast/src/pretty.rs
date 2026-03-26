//! Simple AST pretty printer for debugging.

use crate::*;

/// Pretty-print a source file AST for debugging.
pub fn pretty_print(file: &SourceFile) -> String {
    let mut out = String::new();

    if let Some(ref m) = file.module_decl {
        out.push_str(&format!(
            "module {}\n\n",
            m.name.segments.join(".")
        ));
    }

    for imp in &file.imports {
        out.push_str(&format!("import {}", imp.path.segments.join(".")));
        if let Some(ref items) = imp.items {
            out.push_str(".{");
            let names: Vec<&str> = items
                .iter()
                .map(|i| match i {
                    ImportItem::Name(n) => n.as_str(),
                    ImportItem::Type(n) => n.as_str(),
                })
                .collect();
            out.push_str(&names.join(", "));
            out.push('}');
        }
        out.push('\n');
    }
    if !file.imports.is_empty() {
        out.push('\n');
    }

    for item in &file.items {
        pretty_print_item(&mut out, item, 0);
        out.push('\n');
    }

    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn pretty_print_item(out: &mut String, item: &Item, level: usize) {
    match item {
        Item::FnDef(f) => {
            indent(out, level);
            out.push_str(&format!("fn {}(", f.name));
            for (i, p) in f.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_pattern(out, &p.pattern);
                if let Some(ref ty) = p.ty {
                    out.push_str(": ");
                    pretty_print_type(out, ty);
                }
            }
            out.push(')');
            if let Some(ref ret) = f.return_type {
                out.push_str(" -> ");
                pretty_print_type(out, ret);
            }
            out.push_str(" = ");
            pretty_print_expr(out, &f.body, level + 1);
            out.push('\n');
        }
        Item::TypeDef(t) => {
            indent(out, level);
            out.push_str(&format!("type {}", t.name));
            if !t.type_params.is_empty() {
                out.push('[');
                let names: Vec<&str> = t.type_params.iter().map(|p| p.name.as_str()).collect();
                out.push_str(&names.join(", "));
                out.push(']');
            }
            out.push_str(" =\n");
            match &t.body {
                TypeBody::Sum(variants) => {
                    for v in variants {
                        indent(out, level + 1);
                        out.push_str(&format!("| {}", v.name));
                        if !v.fields.is_empty() {
                            out.push('(');
                            for (i, f) in v.fields.iter().enumerate() {
                                if i > 0 {
                                    out.push_str(", ");
                                }
                                pretty_print_type(out, f);
                            }
                            out.push(')');
                        }
                        out.push('\n');
                    }
                }
                TypeBody::Record(fields) => {
                    indent(out, level + 1);
                    out.push_str("{\n");
                    for f in fields {
                        indent(out, level + 2);
                        out.push_str(&format!("{}: ", f.name));
                        pretty_print_type(out, &f.ty);
                        out.push_str(",\n");
                    }
                    indent(out, level + 1);
                    out.push_str("}\n");
                }
                TypeBody::Capability(_) => {
                    indent(out, level + 1);
                    out.push_str("<capability>\n");
                }
            }
        }
        Item::TypeAlias(a) => {
            indent(out, level);
            out.push_str(&format!("type alias {} = ", a.name));
            pretty_print_type(out, &a.target);
            out.push('\n');
        }
        Item::TestDef(t) => {
            indent(out, level);
            out.push_str(&format!("test \"{}\" = ...\n", t.name));
        }
        Item::TraitDef(t) => {
            indent(out, level);
            out.push_str(&format!("trait {}", t.name));
            if !t.type_params.is_empty() {
                out.push('[');
                let names: Vec<&str> = t.type_params.iter().map(|p| p.name.as_str()).collect();
                out.push_str(&names.join(", "));
                out.push(']');
            }
            out.push_str(" = ...\n");
        }
        Item::ImplBlock(i) => {
            indent(out, level);
            out.push_str(&format!("impl {}", i.trait_name.segments.join(".")));
            out.push_str("[...] = ...\n");
        }
        Item::SupervisorDef(s) => {
            indent(out, level);
            out.push_str(&format!("supervisor {} = ...\n", s.name));
        }
        Item::ForeignBlock(f) => {
            indent(out, level);
            out.push_str(&format!("foreign \"{}\" ...\n", f.abi));
        }
        _ => {
            indent(out, level);
            out.push_str("<item>\n");
        }
    }
}

pub fn pretty_print_expr(out: &mut String, expr: &Expr, level: usize) {
    match expr {
        Expr::IntLit { value, .. } => out.push_str(value),
        Expr::FloatLit { value, .. } => out.push_str(value),
        Expr::StringLit { segments, .. } => {
            out.push('"');
            for seg in segments {
                match seg {
                    StringSegment::Literal(s) => out.push_str(s),
                    StringSegment::Interpolation(e) => {
                        out.push_str("${");
                        pretty_print_expr(out, e, level);
                        out.push('}');
                    }
                }
            }
            out.push('"');
        }
        Expr::CharLit { value, .. } => {
            out.push('\'');
            out.push(*value);
            out.push('\'');
        }
        Expr::BoolLit { value, .. } => {
            out.push_str(if *value { "True" } else { "False" });
        }
        Expr::UnitLit { .. } => out.push_str("()"),
        Expr::Var { name, .. } => out.push_str(name),
        Expr::Constructor { name, .. } => {
            out.push_str(&name.segments.join("."));
        }
        Expr::App { func, args, .. } => {
            pretty_print_expr(out, func, level);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_expr(out, a, level);
            }
            out.push(')');
        }
        Expr::BinOp { op, lhs, rhs, .. } => {
            pretty_print_expr(out, lhs, level);
            let op_str = match op {
                BinOp::Add => " + ",
                BinOp::Sub => " - ",
                BinOp::Mul => " * ",
                BinOp::Div => " / ",
                BinOp::Mod => " % ",
                BinOp::Eq => " == ",
                BinOp::Neq => " != ",
                BinOp::Lt => " < ",
                BinOp::Gt => " > ",
                BinOp::LtEq => " <= ",
                BinOp::GtEq => " >= ",
                BinOp::And => " && ",
                BinOp::Or => " || ",
                BinOp::Concat => " ++ ",
                BinOp::Append => " <> ",
            };
            out.push_str(op_str);
            pretty_print_expr(out, rhs, level);
        }
        Expr::UnaryOp { op, expr: e, .. } => {
            match op {
                UnaryOp::Neg => out.push('-'),
                UnaryOp::Not => out.push('!'),
            }
            pretty_print_expr(out, e, level);
        }
        Expr::Let {
            pattern,
            value,
            body,
            ..
        } => {
            out.push_str("let ");
            pretty_print_pattern(out, pattern);
            out.push_str(" = ");
            pretty_print_expr(out, value, level);
            out.push('\n');
            indent(out, level);
            pretty_print_expr(out, body, level);
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            out.push_str("if ");
            pretty_print_expr(out, condition, level);
            out.push_str(" then ");
            pretty_print_expr(out, then_branch, level + 1);
            out.push_str(" else ");
            pretty_print_expr(out, else_branch, level + 1);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            out.push_str("match ");
            pretty_print_expr(out, scrutinee, level);
            out.push_str(" with\n");
            for arm in arms {
                indent(out, level + 1);
                out.push_str("| ");
                pretty_print_pattern(out, &arm.pattern);
                if let Some(ref g) = arm.guard {
                    out.push_str(" if ");
                    pretty_print_expr(out, g, level);
                }
                out.push_str(" -> ");
                pretty_print_expr(out, &arm.body, level + 2);
                out.push('\n');
            }
        }
        Expr::Pipeline { lhs, rhs, .. } => {
            pretty_print_expr(out, lhs, level);
            out.push_str(" |> ");
            pretty_print_expr(out, rhs, level);
        }
        Expr::Compose { lhs, rhs, .. } => {
            pretty_print_expr(out, lhs, level);
            out.push_str(" >> ");
            pretty_print_expr(out, rhs, level);
        }
        Expr::Block { exprs, .. } => {
            for (i, e) in exprs.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                    indent(out, level);
                }
                pretty_print_expr(out, e, level);
            }
        }
        Expr::ListLit { elements, .. } => {
            out.push('[');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_expr(out, e, level);
            }
            out.push(']');
        }
        Expr::RecordLit { fields, .. } => {
            out.push_str("{ ");
            for (i, (name, val)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                out.push_str(" = ");
                pretty_print_expr(out, val, level);
            }
            out.push_str(" }");
        }
        Expr::RecordUpdate {
            base, updates, ..
        } => {
            out.push_str("{ ");
            pretty_print_expr(out, base, level);
            out.push_str(" | ");
            for (i, (name, val)) in updates.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                out.push_str(" = ");
                pretty_print_expr(out, val, level);
            }
            out.push_str(" }");
        }
        Expr::Lambda { params, body, .. } => {
            out.push_str("fn(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_pattern(out, &p.pattern);
            }
            out.push_str(") -> ");
            pretty_print_expr(out, body, level);
        }
        Expr::FieldAccess { expr: e, field, .. } => {
            pretty_print_expr(out, e, level);
            out.push('.');
            out.push_str(field);
        }
        Expr::Try { expr: e, .. } => {
            pretty_print_expr(out, e, level);
            out.push('?');
        }
        Expr::TupleLit { elements, .. } => {
            out.push('(');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_expr(out, e, level);
            }
            out.push(')');
        }
        Expr::Receive { arms, .. } => {
            out.push_str("receive\n");
            for arm in arms {
                indent(out, level + 1);
                out.push_str("| ");
                pretty_print_pattern(out, &arm.pattern);
                out.push_str(" -> ");
                pretty_print_expr(out, &arm.body, level + 2);
                out.push('\n');
            }
        }
        Expr::Annotation { expr: e, ty, .. } => {
            out.push('(');
            pretty_print_expr(out, e, level);
            out.push_str(" : ");
            pretty_print_type(out, ty);
            out.push(')');
        }
        _ => out.push_str("<expr>"),
    }
}

pub fn pretty_print_pattern(out: &mut String, pat: &Pattern) {
    match pat {
        Pattern::Wildcard { .. } => out.push('_'),
        Pattern::Var { name, .. } => out.push_str(name),
        Pattern::Constructor { name, fields, .. } => {
            out.push_str(&name.segments.join("."));
            if !fields.is_empty() {
                out.push('(');
                for (i, f) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    pretty_print_pattern(out, f);
                }
                out.push(')');
            }
        }
        Pattern::Literal { expr, .. } => {
            pretty_print_expr(out, expr, 0);
        }
        Pattern::Record {
            fields, rest, ..
        } => {
            out.push_str("{ ");
            for (i, (name, pat)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(name);
                out.push_str(" = ");
                pretty_print_pattern(out, pat);
            }
            if *rest {
                out.push_str(", ..");
            }
            out.push_str(" }");
        }
        Pattern::List {
            elements, rest, ..
        } => {
            out.push('[');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_pattern(out, e);
            }
            if let Some(r) = rest {
                out.push_str(", ..");
                pretty_print_pattern(out, r);
            }
            out.push(']');
        }
        Pattern::Tuple { elements, .. } => {
            out.push('(');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_pattern(out, e);
            }
            out.push(')');
        }
        Pattern::Pin { name, .. } => {
            out.push('^');
            out.push_str(name);
        }
        Pattern::Or { patterns, .. } => {
            for (i, p) in patterns.iter().enumerate() {
                if i > 0 {
                    out.push_str(" | ");
                }
                pretty_print_pattern(out, p);
            }
        }
        Pattern::As {
            pattern, name, ..
        } => {
            pretty_print_pattern(out, pattern);
            out.push_str(" as ");
            out.push_str(name);
        }
    }
}

pub fn pretty_print_type(out: &mut String, ty: &TypeExpr) {
    match ty {
        TypeExpr::Named { name, args, .. } => {
            out.push_str(&name.segments.join("."));
            if !args.is_empty() {
                out.push('[');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    pretty_print_type(out, a);
                }
                out.push(']');
            }
        }
        TypeExpr::Var { name, .. } => out.push_str(name),
        TypeExpr::Fn {
            params,
            return_type,
            ..
        } => {
            out.push_str("fn(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_type(out, p);
            }
            out.push_str(") -> ");
            pretty_print_type(out, return_type);
        }
        TypeExpr::Record { fields, row_var, .. } => {
            out.push_str("{ ");
            for (i, f) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{}: ", f.name));
                pretty_print_type(out, &f.ty);
            }
            if let Some(rv) = row_var {
                out.push_str(" | ");
                out.push_str(rv);
            }
            out.push_str(" }");
        }
        TypeExpr::Tuple { elements, .. } => {
            out.push('(');
            for (i, e) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                pretty_print_type(out, e);
            }
            out.push(')');
        }
        TypeExpr::Owned { inner, .. } => {
            out.push_str("own ");
            pretty_print_type(out, inner);
        }
        TypeExpr::Borrowed { inner, .. } => {
            out.push_str("ref ");
            pretty_print_type(out, inner);
        }
        TypeExpr::Unit { .. } => out.push_str("()"),
        TypeExpr::Never { .. } => out.push_str("Never"),
        TypeExpr::Forall { params, body, .. } => {
            out.push_str("forall ");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str(&p.name);
            }
            out.push_str(". ");
            pretty_print_type(out, body);
        }
    }
}

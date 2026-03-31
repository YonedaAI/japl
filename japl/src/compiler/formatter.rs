use super::ast::*;

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    for (i, item) in program.items.iter().enumerate() {
        if i > 0 { out.push('\n'); }
        format_top_level(&mut out, item, 0);
    }
    out
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn format_top_level(out: &mut String, item: &TopLevel, level: usize) {
    match item {
        TopLevel::FnDef(fd) => format_fn_def(out, fd, level),
        TopLevel::TypeDef(td) => format_type_def(out, td, level),
        TopLevel::ForeignFn(ff) => format_foreign_fn(out, ff, level),
        TopLevel::Import(imp) => format_import(out, imp, level),
        TopLevel::Const(cd) => format_const(out, cd, level),
        TopLevel::TraitDef(td) => format_trait(out, td, level),
        TopLevel::OpaqueType(od) => format_opaque(out, od, level),
    }
}

fn format_fn_def(out: &mut String, fd: &FnDef, level: usize) {
    indent(out, level);
    if fd.is_pub { out.push_str("pub "); }
    out.push_str("fn ");
    out.push_str(&fd.name);
    if !fd.type_params.is_empty() {
        out.push('<');
        for (i, tp) in fd.type_params.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(tp);
        }
        out.push('>');
    }
    out.push('(');
    for (i, p) in fd.params.iter().enumerate() {
        if i > 0 { out.push_str(", "); }
        out.push_str(&p.name);
        out.push_str(": ");
        format_type(out, &p.ty);
    }
    out.push(')');
    if let Some(ref ret) = fd.ret_ty {
        out.push_str(" -> ");
        format_type(out, ret);
    }
    out.push_str(" {\n");
    format_expr(out, &fd.body, level + 1);
    out.push('\n');
    indent(out, level);
    out.push_str("}\n");
}

fn format_type_def(out: &mut String, td: &TypeDef, level: usize) {
    indent(out, level);
    out.push_str("type ");
    out.push_str(&td.name);
    if !td.type_params.is_empty() {
        out.push('<');
        for (i, tp) in td.type_params.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(tp);
        }
        out.push('>');
    }
    out.push_str(" =");
    for v in &td.variants {
        out.push_str("\n");
        indent(out, level + 1);
        out.push_str("| ");
        out.push_str(&v.name);
        if !v.fields.is_empty() {
            out.push('(');
            for (i, f) in v.fields.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(out, f);
            }
            out.push(')');
        }
    }
    out.push('\n');
}

fn format_foreign_fn(out: &mut String, ff: &ForeignFnDef, level: usize) {
    indent(out, level);
    out.push_str(&format!("foreign \"{}\" fn {}(", ff.module, ff.name));
    for (i, p) in ff.params.iter().enumerate() {
        if i > 0 { out.push_str(", "); }
        out.push_str(&p.name);
        out.push_str(": ");
        format_type(out, &p.ty);
    }
    out.push(')');
    if let Some(ref ret) = ff.ret_ty {
        out.push_str(" -> ");
        format_type(out, ret);
    }
    out.push('\n');
}

fn format_import(out: &mut String, imp: &ImportDef, level: usize) {
    indent(out, level);
    out.push_str("import ");
    out.push_str(&imp.module_path.join("."));
    if !imp.names.is_empty() {
        out.push_str(".{");
        out.push_str(&imp.names.join(", "));
        out.push('}');
    }
    out.push('\n');
}

fn format_const(out: &mut String, cd: &ConstDef, level: usize) {
    indent(out, level);
    out.push_str("const ");
    out.push_str(&cd.name);
    out.push_str(" = ");
    format_expr(out, &cd.value, 0);
    out.push('\n');
}

fn format_trait(out: &mut String, td: &TraitDef, level: usize) {
    indent(out, level);
    out.push_str(&format!("trait {}({}) {{\n", td.name, td.type_param));
    for m in &td.methods {
        indent(out, level + 1);
        out.push_str(&format!("fn {}(", m.name));
        for (i, p) in m.params.iter().enumerate() {
            if i > 0 { out.push_str(", "); }
            out.push_str(&p.name);
            out.push_str(": ");
            format_type(out, &p.ty);
        }
        out.push_str(") -> ");
        format_type(out, &m.ret_ty);
        out.push('\n');
    }
    indent(out, level);
    out.push_str("}\n");
}

fn format_opaque(out: &mut String, od: &OpaqueTypeDef, level: usize) {
    indent(out, level);
    out.push_str("opaque type ");
    out.push_str(&od.name);
    out.push_str(" = ");
    format_type(out, &od.inner);
    out.push('\n');
}

fn format_type(out: &mut String, ty: &Type) {
    match ty {
        Type::Named(name) => out.push_str(name),
        Type::FnType(params, ret) => {
            out.push_str("fn(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(out, p);
            }
            out.push_str(") -> ");
            format_type(out, ret);
        }
        Type::Tuple(types) => {
            out.push('(');
            for (i, t) in types.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(out, t);
            }
            out.push(')');
        }
        Type::Void => out.push_str("Unit"),
    }
}

fn format_expr(out: &mut String, expr: &Expr, level: usize) {
    match expr {
        Expr::IntLit(n) => out.push_str(&n.to_string()),
        Expr::FloatLit(f) => out.push_str(&f.to_string()),
        Expr::StringLit(s) => {
            out.push('"');
            for ch in s.chars() {
                match ch {
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    _ => out.push(ch),
                }
            }
            out.push('"');
        }
        Expr::BoolLit(b) => out.push_str(if *b { "True" } else { "False" }),
        Expr::ByteLit(b) => out.push_str(&b.to_string()),
        Expr::Ident(name) => out.push_str(name),
        Expr::Call(func, args) => {
            format_expr(out, func, level);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_expr(out, a, level);
            }
            out.push(')');
        }
        Expr::BinOp(op, l, r) => {
            format_expr(out, l, level);
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
                BinOp::Concat => " <> ",
                BinOp::And => " && ",
                BinOp::Or => " || ",
            };
            out.push_str(op_str);
            format_expr(out, r, level);
        }
        Expr::If(cond, then, else_) => {
            out.push_str("if ");
            format_expr(out, cond, level);
            out.push_str(" {\n");
            indent(out, level + 1);
            format_expr(out, then, level + 1);
            out.push('\n');
            indent(out, level);
            out.push('}');
            if let Some(e) = else_ {
                out.push_str(" else {\n");
                indent(out, level + 1);
                format_expr(out, e, level + 1);
                out.push('\n');
                indent(out, level);
                out.push('}');
            }
        }
        Expr::Block(stmts, final_expr) => {
            for stmt in stmts {
                indent(out, level);
                match stmt {
                    Stmt::Let(name, e) => {
                        out.push_str("let ");
                        out.push_str(name);
                        out.push_str(" = ");
                        format_expr(out, e, level);
                    }
                    Stmt::LetTyped(name, ty, e) => {
                        out.push_str("let ");
                        out.push_str(name);
                        out.push_str(": ");
                        format_type(out, ty);
                        out.push_str(" = ");
                        format_expr(out, e, level);
                    }
                    Stmt::Expr(e) => {
                        format_expr(out, e, level);
                    }
                }
                out.push('\n');
            }
            if let Some(e) = final_expr {
                indent(out, level);
                format_expr(out, e, level);
            }
        }
        Expr::Lambda(params, ret_ty, body) => {
            out.push_str("fn(");
            for (i, p) in params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&p.name);
                out.push_str(": ");
                format_type(out, &p.ty);
            }
            out.push(')');
            if let Some(ref ret) = ret_ty {
                out.push_str(" -> ");
                format_type(out, ret);
            }
            out.push_str(" { ");
            format_expr(out, body, level);
            out.push_str(" }");
        }
        Expr::Match(scrutinee, arms) => {
            out.push_str("match ");
            format_expr(out, scrutinee, level);
            out.push_str(" {\n");
            for arm in arms {
                indent(out, level + 1);
                format_pattern(out, &arm.pattern);
                if let Some(ref guard) = arm.guard {
                    out.push_str(" if ");
                    format_expr(out, guard, level);
                }
                out.push_str(" => ");
                format_expr(out, &arm.body, level + 1);
                out.push_str(",\n");
            }
            indent(out, level);
            out.push('}');
        }
        Expr::Receive(arms) => {
            out.push_str("receive {\n");
            for arm in arms {
                indent(out, level + 1);
                format_pattern(out, &arm.pattern);
                if let Some(ref guard) = arm.guard {
                    out.push_str(" if ");
                    format_expr(out, guard, level);
                }
                out.push_str(" => ");
                format_expr(out, &arm.body, level + 1);
                out.push_str(",\n");
            }
            indent(out, level);
            out.push('}');
        }
        Expr::Record(fields) => {
            out.push_str("{ ");
            for (i, (name, val)) in fields.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(name);
                out.push_str(": ");
                format_expr(out, val, level);
            }
            out.push_str(" }");
        }
        Expr::FieldAccess(base, field) => {
            format_expr(out, base, level);
            out.push('.');
            out.push_str(field);
        }
        Expr::RecordUpdate(base, fields) => {
            out.push_str("{ ");
            format_expr(out, base, level);
            out.push_str(" | ");
            for (i, (name, val)) in fields.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(name);
                out.push_str(": ");
                format_expr(out, val, level);
            }
            out.push_str(" }");
        }
        Expr::Pipe(left, right) => {
            format_expr(out, left, level);
            out.push_str(" |> ");
            format_expr(out, right, level);
        }
        Expr::Tuple(exprs) => {
            out.push('(');
            for (i, e) in exprs.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_expr(out, e, level);
            }
            out.push(')');
        }
        Expr::TupleAccess(expr, idx) => {
            format_expr(out, expr, level);
            out.push('.');
            out.push_str(&idx.to_string());
        }
        Expr::UseExpr(name, resource, body) => {
            out.push_str("use ");
            out.push_str(name);
            out.push_str(" = ");
            format_expr(out, resource, level);
            out.push('\n');
            indent(out, level);
            format_expr(out, body, level);
        }
    }
}

fn format_pattern(out: &mut String, pat: &Pattern) {
    match pat {
        Pattern::Variant(name, bindings) => {
            out.push_str(name);
            if !bindings.is_empty() {
                out.push('(');
                out.push_str(&bindings.join(", "));
                out.push(')');
            }
        }
        Pattern::Wildcard => out.push('_'),
        Pattern::IntLit(n) => out.push_str(&n.to_string()),
        Pattern::StringLit(s) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
        Pattern::BoolLit(b) => out.push_str(if *b { "True" } else { "False" }),
    }
}

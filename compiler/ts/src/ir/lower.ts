// ─── AST → IR Lowering ───
// Transforms the parsed AST into a simplified IR suitable for codegen.
// Strips spans, desugars pipes, and simplifies structure.

import * as AST from '../parser/ast.js';
import * as IR from './ir.js';

export function lowerModule(module: AST.Module): IR.IrModule {
  const decls: IR.IrDecl[] = [];
  for (const decl of module.decls) {
    const lowered = lowerDecl(decl);
    if (lowered !== null) {
      decls.push(lowered);
    }
  }
  return { decls };
}

function lowerDecl(decl: AST.Decl): IR.IrDecl | null {
  switch (decl.kind) {
    case "fn":
      return {
        kind: "fn",
        name: decl.name,
        params: decl.params.map(p => p.name),
        body: lowerExpr(decl.body),
        exported: decl.pub,
      };

    case "type":
      return {
        kind: "type",
        name: decl.name,
        variants: decl.variants.map(v => ({
          name: v.name,
          fields: v.fields.length,
        })),
      };

    case "record_type":
      return {
        kind: "record_type",
        name: decl.name,
        fields: decl.fields.map(f => [f.name, lowerTypeExprToString(f.type)] as [string, string]),
      };

    case "test":
      return {
        kind: "test",
        name: decl.name,
        body: lowerExpr(decl.body),
      };

    case "import":
      return {
        kind: "import",
        path: decl.path,
        items: decl.items,
      };

    case "trait":
    case "impl":
    case "module":
    case "supervisor":
    case "foreign":
      // These are either not yet supported in codegen or handled differently
      return null;
  }
}

function lowerTypeExprToString(type: AST.TypeExpr): string {
  switch (type.kind) {
    case "tnamed":
      if (type.args.length === 0) return type.name;
      return `${type.name}<${type.args.map(lowerTypeExprToString).join(", ")}>`;
    case "tfn":
      return `(${type.params.map(lowerTypeExprToString).join(", ")}) => ${lowerTypeExprToString(type.ret)}`;
    case "trecord":
      return `{ ${type.fields.map(([n, t]) => `${n}: ${lowerTypeExprToString(t)}`).join("; ")} }`;
    case "ttuple":
      return `[${type.elements.map(lowerTypeExprToString).join(", ")}]`;
    case "tunit":
      return "void";
    case "tvar":
      return type.name;
  }
}

function lowerExpr(expr: AST.Expr): IR.IrExpr {
  switch (expr.kind) {
    case "int":
      return { kind: "int", value: expr.value };
    case "float":
      return { kind: "float", value: expr.value };
    case "string":
      return { kind: "string", value: expr.value };
    case "bool":
      return { kind: "bool", value: expr.value };
    case "unit":
      return { kind: "unit" };
    case "var":
      return { kind: "var", name: expr.name };

    case "constructor":
      return {
        kind: "construct",
        tag: expr.name,
        args: expr.args.map(lowerExpr),
      };

    case "app":
      return {
        kind: "app",
        fn: lowerExpr(expr.fn),
        args: expr.args.map(lowerExpr),
      };

    case "lambda":
      return {
        kind: "lambda",
        params: expr.params.map(p => p.name),
        body: lowerExpr(expr.body),
      };

    case "let":
      return {
        kind: "let",
        name: expr.name,
        value: lowerExpr(expr.value),
        body: lowerExpr(expr.body),
      };

    case "match":
      return {
        kind: "match",
        scrutinee: lowerExpr(expr.scrutinee),
        arms: expr.arms.map(lowerMatchArm),
      };

    case "if":
      return {
        kind: "if",
        cond: lowerExpr(expr.condition),
        then: lowerExpr(expr.then),
        else: expr.else ? lowerExpr(expr.else) : { kind: "unit" },
      };

    case "pipe":
      return lowerPipe(expr);

    case "binop":
      if (expr.op === "<>") {
        return {
          kind: "concat",
          left: lowerExpr(expr.left),
          right: lowerExpr(expr.right),
        };
      }
      return {
        kind: "binop",
        op: expr.op,
        left: lowerExpr(expr.left),
        right: lowerExpr(expr.right),
      };

    case "unaryop":
      return {
        kind: "unaryop",
        op: expr.op,
        operand: lowerExpr(expr.operand),
      };

    case "record":
      return {
        kind: "record",
        fields: expr.fields.map(([name, val]) => [name, lowerExpr(val)] as [string, IR.IrExpr]),
      };

    case "field_access":
      return {
        kind: "field_access",
        expr: lowerExpr(expr.expr),
        field: expr.field,
      };

    case "record_update":
      return {
        kind: "record_update",
        record: lowerExpr(expr.record),
        updates: expr.fields.map(([name, val]) => [name, lowerExpr(val)] as [string, IR.IrExpr]),
      };

    case "list":
      return {
        kind: "list",
        elements: expr.elements.map(lowerExpr),
      };

    case "block":
      return lowerBlock(expr.exprs);

    case "spawn":
      return {
        kind: "spawn",
        fn: lowerExpr(expr.expr),
      };

    case "send":
      return {
        kind: "send",
        pid: lowerExpr(expr.target),
        msg: lowerExpr(expr.message),
      };

    case "receive":
      return {
        kind: "receive",
        arms: expr.arms.map(lowerMatchArm),
      };

    case "try":
      return {
        kind: "try",
        expr: lowerExpr(expr.expr),
      };

    case "return":
      return {
        kind: "return",
        expr: expr.expr ? lowerExpr(expr.expr) : { kind: "unit" },
      };
  }
}

function lowerPipe(expr: AST.Expr & { kind: "pipe" }): IR.IrExpr {
  const arg = lowerExpr(expr.left);
  const right = expr.right;

  // a |> f(b) → f(a, b)  (partial application style)
  if (right.kind === "app") {
    return {
      kind: "app",
      fn: lowerExpr(right.fn),
      args: [arg, ...right.args.map(lowerExpr)],
    };
  }

  // a |> f → f(a)
  return {
    kind: "app",
    fn: lowerExpr(right),
    args: [arg],
  };
}

function lowerBlock(exprs: AST.Expr[]): IR.IrExpr {
  if (exprs.length === 0) {
    return { kind: "unit" };
  }
  if (exprs.length === 1) {
    return lowerExpr(exprs[0]);
  }

  // Convert block of expressions into nested lets for side-effecting exprs,
  // with the last expression as the final value.
  // If an expr is a "let", it naturally chains.
  // Otherwise, wrap as let _ = expr; rest
  const lowered = exprs.map(lowerExpr);
  return { kind: "block", exprs: lowered };
}

function lowerMatchArm(arm: AST.MatchArm): IR.IrMatchArm {
  return {
    pattern: lowerPattern(arm.pattern),
    guard: arm.guard ? lowerExpr(arm.guard) : undefined,
    body: lowerExpr(arm.body),
  };
}

function lowerPattern(pat: AST.Pattern): IR.IrPattern {
  switch (pat.kind) {
    case "pvar":
      return { kind: "pvar", name: pat.name };

    case "pconstructor":
      return {
        kind: "pconstructor",
        tag: pat.name,
        args: pat.args.map(lowerPattern),
      };

    case "pliteral":
      return {
        kind: "pliteral",
        value: lowerExpr(pat.value),
      };

    case "pwildcard":
      return { kind: "pwildcard" };

    case "plist":
      return {
        kind: "plist",
        elements: pat.elements.map(lowerPattern),
        rest: pat.rest,
      };

    case "precord":
    case "ptuple":
      // For now, lower record/tuple patterns as wildcards
      // Full support can be added later
      return { kind: "pwildcard" };
  }
}

// ─── Linearity Checker ───
// Checks that Owned<T> values are used exactly once (linear types).
// A variable is considered "owned" if its type annotation includes Owned.
// Double use of an owned variable is an error.

import * as AST from '../parser/ast.js';
import { TypeError } from './errors.js';
import { Span } from '../lexer/token.js';

export class LinearityChecker {
  private errors: TypeError[] = [];

  /**
   * Check a module for linearity violations.
   * Returns errors for any Owned<T> values used more than once.
   */
  checkModule(mod: AST.Module): TypeError[] {
    this.errors = [];
    for (const decl of mod.decls) {
      if (decl.kind === 'fn') {
        this.checkFnDecl(decl);
      }
    }
    return this.errors;
  }

  private checkFnDecl(decl: Extract<AST.Decl, { kind: 'fn' }>): void {
    // Collect owned parameters
    const ownedParams = new Set<string>();
    for (const param of decl.params) {
      if (param.type && this.isOwnedType(param.type)) {
        ownedParams.add(param.name);
      }
    }

    if (ownedParams.size === 0) return;

    // Count uses of each owned param in the body
    const useCounts = new Map<string, { count: number; spans: Span[] }>();
    for (const name of ownedParams) {
      useCounts.set(name, { count: 0, spans: [] });
    }

    this.countVarUses(decl.body, ownedParams, useCounts);

    // Check for violations
    for (const [name, info] of useCounts) {
      if (info.count > 1) {
        this.errors.push(new TypeError(
          `Owned value '${name}' used more than once (${info.count} uses). Owned values must be used exactly once`,
          info.spans[1] ?? decl.span,
          ['Consider cloning the value or restructuring to use it only once'],
        ));
      } else if (info.count === 0) {
        this.errors.push(new TypeError(
          `Owned value '${name}' is never used. Owned values must be used exactly once`,
          decl.span,
          ['Use the value or explicitly drop it'],
        ));
      }
    }
  }

  private isOwnedType(type: AST.TypeExpr): boolean {
    if (type.kind === 'tnamed' && type.name === 'Owned') {
      return true;
    }
    return false;
  }

  private countVarUses(
    expr: AST.Expr,
    tracked: Set<string>,
    counts: Map<string, { count: number; spans: Span[] }>,
  ): void {
    switch (expr.kind) {
      case 'var':
        if (tracked.has(expr.name)) {
          const entry = counts.get(expr.name)!;
          entry.count++;
          entry.spans.push(expr.span);
        }
        break;
      case 'app':
        this.countVarUses(expr.fn, tracked, counts);
        for (const arg of expr.args) this.countVarUses(arg, tracked, counts);
        break;
      case 'lambda':
        this.countVarUses(expr.body, tracked, counts);
        break;
      case 'let':
        this.countVarUses(expr.value, tracked, counts);
        this.countVarUses(expr.body, tracked, counts);
        break;
      case 'if':
        this.countVarUses(expr.condition, tracked, counts);
        this.countVarUses(expr.then, tracked, counts);
        if (expr.else) this.countVarUses(expr.else, tracked, counts);
        break;
      case 'match':
        this.countVarUses(expr.scrutinee, tracked, counts);
        for (const arm of expr.arms) {
          this.countVarUses(arm.body, tracked, counts);
        }
        break;
      case 'block':
        for (const e of expr.exprs) this.countVarUses(e, tracked, counts);
        break;
      case 'binop':
        this.countVarUses(expr.left, tracked, counts);
        this.countVarUses(expr.right, tracked, counts);
        break;
      case 'unaryop':
        this.countVarUses(expr.operand, tracked, counts);
        break;
      case 'pipe':
        this.countVarUses(expr.left, tracked, counts);
        this.countVarUses(expr.right, tracked, counts);
        break;
      case 'record':
        for (const [, val] of expr.fields) this.countVarUses(val, tracked, counts);
        break;
      case 'field_access':
        this.countVarUses(expr.expr, tracked, counts);
        break;
      case 'record_update':
        this.countVarUses(expr.record, tracked, counts);
        for (const [, val] of expr.fields) this.countVarUses(val, tracked, counts);
        break;
      case 'list':
        for (const e of expr.elements) this.countVarUses(e, tracked, counts);
        break;
      case 'constructor':
        for (const arg of expr.args) this.countVarUses(arg, tracked, counts);
        break;
      case 'spawn':
        this.countVarUses(expr.expr, tracked, counts);
        break;
      case 'send':
        this.countVarUses(expr.target, tracked, counts);
        this.countVarUses(expr.message, tracked, counts);
        break;
      case 'receive':
        for (const arm of expr.arms) this.countVarUses(arm.body, tracked, counts);
        break;
      case 'try':
        this.countVarUses(expr.expr, tracked, counts);
        break;
      case 'return':
        if (expr.expr) this.countVarUses(expr.expr, tracked, counts);
        break;
      case 'int':
      case 'float':
      case 'string':
      case 'bool':
      case 'unit':
        break;
    }
  }
}

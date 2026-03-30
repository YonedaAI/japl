// ─── JAPL Intermediate Representation ───
// A simplified representation close to TypeScript output.
// Strips spans/types from AST, keeps only what codegen needs.

export type IrModule = { decls: IrDecl[] };

export type IrDecl =
  | { kind: "fn"; name: string; params: string[]; body: IrExpr; exported: boolean }
  | { kind: "type"; name: string; variants: IrVariant[] }
  | { kind: "record_type"; name: string; fields: [string, string][] }
  | { kind: "test"; name: string; body: IrExpr }
  | { kind: "import"; path: string[]; items: string[] }
  | { kind: "foreign"; module?: string; name: string; jsName?: string; params: string[] };

export type IrVariant = { name: string; fields: number };

export type IrExpr =
  | { kind: "int"; value: number }
  | { kind: "float"; value: number }
  | { kind: "string"; value: string }
  | { kind: "bool"; value: boolean }
  | { kind: "unit" }
  | { kind: "var"; name: string }
  | { kind: "let"; name: string; value: IrExpr; body: IrExpr }
  | { kind: "app"; fn: IrExpr; args: IrExpr[] }
  | { kind: "lambda"; params: string[]; body: IrExpr }
  | { kind: "if"; cond: IrExpr; then: IrExpr; else: IrExpr }
  | { kind: "match"; scrutinee: IrExpr; arms: IrMatchArm[] }
  | { kind: "binop"; op: string; left: IrExpr; right: IrExpr }
  | { kind: "unaryop"; op: string; operand: IrExpr }
  | { kind: "record"; fields: [string, IrExpr][] }
  | { kind: "field_access"; expr: IrExpr; field: string }
  | { kind: "record_update"; record: IrExpr; updates: [string, IrExpr][] }
  | { kind: "list"; elements: IrExpr[] }
  | { kind: "construct"; tag: string; args: IrExpr[] }
  | { kind: "block"; exprs: IrExpr[] }
  | { kind: "spawn"; fn: IrExpr }
  | { kind: "send"; pid: IrExpr; msg: IrExpr }
  | { kind: "receive"; arms: IrMatchArm[] }
  | { kind: "try"; expr: IrExpr }
  | { kind: "return"; expr: IrExpr }
  | { kind: "concat"; left: IrExpr; right: IrExpr }
  | { kind: "tail_loop"; params: string[]; body: IrExpr }
  | { kind: "tail_continue"; args: IrExpr[] };

export type IrMatchArm = { pattern: IrPattern; guard?: IrExpr; body: IrExpr };

export type IrPattern =
  | { kind: "pvar"; name: string }
  | { kind: "pconstructor"; tag: string; args: IrPattern[] }
  | { kind: "pliteral"; value: IrExpr }
  | { kind: "pwildcard" }
  | { kind: "plist"; elements: IrPattern[]; rest?: string };

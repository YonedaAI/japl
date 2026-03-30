// ─── Type Representations ───

export type Type =
  | { kind: "int" }
  | { kind: "float" }
  | { kind: "byte" }
  | { kind: "string" }
  | { kind: "bool" }
  | { kind: "unit" }
  | { kind: "never" }
  | { kind: "var"; id: number }
  | { kind: "named"; name: string; args: Type[] }
  | { kind: "fn"; params: Type[]; ret: Type; effects: EffectRow }
  | { kind: "record"; fields: Map<string, Type>; row?: number }
  | { kind: "tuple"; elements: Type[] }
  | { kind: "list"; element: Type }
  | { kind: "process"; msg: Type }
  | { kind: "pid"; msg: Type }
  | { kind: "result"; ok: Type; err: Type }
  | { kind: "option"; some: Type };

export type Effect = "pure" | "io" | "async" | "process" | "fail" | "llm";

export type EffectRow = {
  effects: Set<Effect>;
  open: boolean;
};

export const PURE: EffectRow = { effects: new Set(), open: false };
export const IO: EffectRow = { effects: new Set(["io"]), open: false };

export type TypeScheme = {
  vars: number[];
  type: Type;
};

// ─── Singleton type constants ───

export const INT: Type = { kind: "int" };
export const FLOAT: Type = { kind: "float" };
export const BYTE: Type = { kind: "byte" };
export const STRING: Type = { kind: "string" };
export const BOOL: Type = { kind: "bool" };
export const UNIT: Type = { kind: "unit" };
export const NEVER: Type = { kind: "never" };

// ─── Fresh variable counter ───

let nextVarId = 0;

export function resetVarCounter(): void {
  nextVarId = 0;
}

export function freshVar(): Type {
  return { kind: "var", id: nextVarId++ };
}

export function currentVarId(): number {
  return nextVarId;
}

// ─── Pretty printing ───

export function typeToString(t: Type): string {
  switch (t.kind) {
    case "int": return "Int";
    case "float": return "Float";
    case "byte": return "Byte";
    case "string": return "String";
    case "bool": return "Bool";
    case "unit": return "Unit";
    case "never": return "Never";
    case "var": return `?${t.id}`;
    case "named":
      if (t.args.length === 0) return t.name;
      return `${t.name}[${t.args.map(typeToString).join(", ")}]`;
    case "fn": {
      const params = t.params.map(typeToString).join(", ");
      const ret = typeToString(t.ret);
      const eff = effectRowToString(t.effects);
      return eff ? `fn(${params}) -> ${eff} ${ret}` : `fn(${params}) -> ${ret}`;
    }
    case "record": {
      const entries: string[] = [];
      for (const [k, v] of t.fields) {
        entries.push(`${k}: ${typeToString(v)}`);
      }
      if (t.row !== undefined) {
        entries.push(`| ?${t.row}`);
      }
      return `{ ${entries.join(", ")} }`;
    }
    case "tuple":
      return `(${t.elements.map(typeToString).join(", ")})`;
    case "list":
      return `List[${typeToString(t.element)}]`;
    case "process":
      return `Process[${typeToString(t.msg)}]`;
    case "pid":
      return `Pid[${typeToString(t.msg)}]`;
    case "result":
      return `Result[${typeToString(t.ok)}, ${typeToString(t.err)}]`;
    case "option":
      return `Option[${typeToString(t.some)}]`;
  }
}

export function effectRowToString(e: EffectRow): string {
  if (e.effects.size === 0 && !e.open) return "";
  const effs: string[] = [...e.effects].map(eff => {
    switch (eff) {
      case "io": return "IO";
      case "async": return "Async";
      case "process": return "Process";
      case "fail": return "Fail";
      case "llm": return "LLM";
      case "pure": return "Pure";
      default: return eff;
    }
  });
  if (e.open) effs.push("..");
  return `![${effs.join(", ")}]`;
}

// ─── Helpers ───

export function monotype(t: Type): TypeScheme {
  return { vars: [], type: t };
}

/** Collect all free type variable ids in a type. */
export function freeVars(t: Type): Set<number> {
  const result = new Set<number>();
  collectFreeVars(t, result);
  return result;
}

function collectFreeVars(t: Type, out: Set<number>): void {
  switch (t.kind) {
    case "int": case "float": case "byte": case "string": case "bool":
    case "unit": case "never":
      break;
    case "var":
      out.add(t.id);
      break;
    case "named":
      for (const a of t.args) collectFreeVars(a, out);
      break;
    case "fn":
      for (const p of t.params) collectFreeVars(p, out);
      collectFreeVars(t.ret, out);
      break;
    case "record":
      for (const v of t.fields.values()) collectFreeVars(v, out);
      if (t.row !== undefined) out.add(t.row);
      break;
    case "tuple":
      for (const e of t.elements) collectFreeVars(e, out);
      break;
    case "list":
      collectFreeVars(t.element, out);
      break;
    case "process":
      collectFreeVars(t.msg, out);
      break;
    case "pid":
      collectFreeVars(t.msg, out);
      break;
    case "result":
      collectFreeVars(t.ok, out);
      collectFreeVars(t.err, out);
      break;
    case "option":
      collectFreeVars(t.some, out);
      break;
  }
}

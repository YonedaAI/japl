// ─── Type Representations ───
export const PURE = { effects: new Set(), open: false };
export const IO = { effects: new Set(["io"]), open: false };
// ─── Singleton type constants ───
export const INT = { kind: "int" };
export const FLOAT = { kind: "float" };
export const BYTE = { kind: "byte" };
export const STRING = { kind: "string" };
export const BOOL = { kind: "bool" };
export const UNIT = { kind: "unit" };
export const NEVER = { kind: "never" };
// ─── Fresh variable counter ───
let nextVarId = 0;
export function resetVarCounter() {
    nextVarId = 0;
}
export function freshVar() {
    return { kind: "var", id: nextVarId++ };
}
export function currentVarId() {
    return nextVarId;
}
// ─── Pretty printing ───
export function typeToString(t) {
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
            if (t.args.length === 0)
                return t.name;
            return `${t.name}[${t.args.map(typeToString).join(", ")}]`;
        case "fn": {
            const params = t.params.map(typeToString).join(", ");
            const ret = typeToString(t.ret);
            const eff = effectRowToString(t.effects);
            return eff ? `fn(${params}) -> ${eff} ${ret}` : `fn(${params}) -> ${ret}`;
        }
        case "record": {
            const entries = [];
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
export function effectRowToString(e) {
    if (e.effects.size === 0 && !e.open)
        return "";
    const effs = [...e.effects].map(eff => {
        switch (eff) {
            case "io": return "IO";
            case "async": return "Async";
            case "process": return "Process";
            case "fail": return "Fail";
            case "pure": return "Pure";
            default: return eff;
        }
    });
    if (e.open)
        effs.push("..");
    return `![${effs.join(", ")}]`;
}
// ─── Helpers ───
export function monotype(t) {
    return { vars: [], type: t };
}
/** Collect all free type variable ids in a type. */
export function freeVars(t) {
    const result = new Set();
    collectFreeVars(t, result);
    return result;
}
function collectFreeVars(t, out) {
    switch (t.kind) {
        case "int":
        case "float":
        case "byte":
        case "string":
        case "bool":
        case "unit":
        case "never":
            break;
        case "var":
            out.add(t.id);
            break;
        case "named":
            for (const a of t.args)
                collectFreeVars(a, out);
            break;
        case "fn":
            for (const p of t.params)
                collectFreeVars(p, out);
            collectFreeVars(t.ret, out);
            break;
        case "record":
            for (const v of t.fields.values())
                collectFreeVars(v, out);
            if (t.row !== undefined)
                out.add(t.row);
            break;
        case "tuple":
            for (const e of t.elements)
                collectFreeVars(e, out);
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
//# sourceMappingURL=types.js.map
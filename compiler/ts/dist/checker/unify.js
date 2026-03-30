import { typeToString } from './types.js';
import { TypeError } from './errors.js';
export class UnificationEngine {
    substitution = new Map();
    /** Unify two types, or throw TypeError on failure. */
    unify(a, b, span) {
        const ra = this.resolve(a);
        const rb = this.resolve(b);
        // Same concrete kind check
        if (ra.kind === rb.kind) {
            switch (ra.kind) {
                case "int":
                case "float":
                case "byte":
                case "string":
                case "bool":
                case "unit":
                case "never":
                    return; // identical primitives
                default:
                    break;
            }
        }
        // Var on either side: bind
        if (ra.kind === "var") {
            if (rb.kind === "var" && ra.id === rb.id)
                return;
            if (this.occursCheck(ra.id, rb)) {
                throw new TypeError(`Infinite type: ${typeToString(ra)} occurs in ${typeToString(rb)}`, span);
            }
            this.substitution.set(ra.id, rb);
            return;
        }
        if (rb.kind === "var") {
            if (this.occursCheck(rb.id, ra)) {
                throw new TypeError(`Infinite type: ${typeToString(rb)} occurs in ${typeToString(ra)}`, span);
            }
            this.substitution.set(rb.id, ra);
            return;
        }
        // Never unifies with anything (bottom type)
        if (ra.kind === "never" || rb.kind === "never")
            return;
        // Same structural kind: recurse
        if (ra.kind !== rb.kind) {
            throw new TypeError(`Expected ${typeToString(ra)} but got ${typeToString(rb)}`, span);
        }
        switch (ra.kind) {
            case "fn": {
                const rbFn = rb;
                if (ra.params.length !== rbFn.params.length) {
                    throw new TypeError(`Function expects ${ra.params.length} arguments but got ${rbFn.params.length}`, span);
                }
                for (let i = 0; i < ra.params.length; i++) {
                    this.unify(ra.params[i], rbFn.params[i], span);
                }
                this.unify(ra.ret, rbFn.ret, span);
                // Effects: we merge/unify effect rows
                this.unifyEffects(ra.effects, rbFn.effects, span);
                break;
            }
            case "named": {
                const rbNamed = rb;
                if (ra.name !== rbNamed.name) {
                    throw new TypeError(`Expected ${typeToString(ra)} but got ${typeToString(rb)}`, span);
                }
                if (ra.args.length !== rbNamed.args.length) {
                    throw new TypeError(`Type ${ra.name} expects ${ra.args.length} type arguments but got ${rbNamed.args.length}`, span);
                }
                for (let i = 0; i < ra.args.length; i++) {
                    this.unify(ra.args[i], rbNamed.args[i], span);
                }
                break;
            }
            case "record": {
                const rbRec = rb;
                this.unifyRecord(ra, rbRec, span);
                break;
            }
            case "tuple": {
                const rbTup = rb;
                if (ra.elements.length !== rbTup.elements.length) {
                    throw new TypeError(`Tuple has ${ra.elements.length} elements but expected ${rbTup.elements.length}`, span);
                }
                for (let i = 0; i < ra.elements.length; i++) {
                    this.unify(ra.elements[i], rbTup.elements[i], span);
                }
                break;
            }
            case "list": {
                const rbList = rb;
                this.unify(ra.element, rbList.element, span);
                break;
            }
            case "process": {
                const rbProc = rb;
                this.unify(ra.msg, rbProc.msg, span);
                break;
            }
            case "pid": {
                const rbPid = rb;
                this.unify(ra.msg, rbPid.msg, span);
                break;
            }
            case "result": {
                const rbRes = rb;
                this.unify(ra.ok, rbRes.ok, span);
                this.unify(ra.err, rbRes.err, span);
                break;
            }
            case "option": {
                const rbOpt = rb;
                this.unify(ra.some, rbOpt.some, span);
                break;
            }
            default:
                throw new TypeError(`Expected ${typeToString(ra)} but got ${typeToString(rb)}`, span);
        }
    }
    /** Follow substitution chain to resolve a type. */
    resolve(t) {
        if (t.kind === "var") {
            const bound = this.substitution.get(t.id);
            if (bound) {
                const resolved = this.resolve(bound);
                // Path compression
                this.substitution.set(t.id, resolved);
                return resolved;
            }
        }
        return t;
    }
    /** Deeply resolve all variables in a type. */
    deepResolve(t) {
        const resolved = this.resolve(t);
        switch (resolved.kind) {
            case "int":
            case "float":
            case "byte":
            case "string":
            case "bool":
            case "unit":
            case "never":
                return resolved;
            case "var":
                return resolved;
            case "named":
                return { kind: "named", name: resolved.name, args: resolved.args.map(a => this.deepResolve(a)) };
            case "fn":
                return {
                    kind: "fn",
                    params: resolved.params.map(p => this.deepResolve(p)),
                    ret: this.deepResolve(resolved.ret),
                    effects: resolved.effects,
                };
            case "record": {
                const fields = new Map();
                for (const [k, v] of resolved.fields) {
                    fields.set(k, this.deepResolve(v));
                }
                return { kind: "record", fields, row: resolved.row };
            }
            case "tuple":
                return { kind: "tuple", elements: resolved.elements.map(e => this.deepResolve(e)) };
            case "list":
                return { kind: "list", element: this.deepResolve(resolved.element) };
            case "process":
                return { kind: "process", msg: this.deepResolve(resolved.msg) };
            case "pid":
                return { kind: "pid", msg: this.deepResolve(resolved.msg) };
            case "result":
                return { kind: "result", ok: this.deepResolve(resolved.ok), err: this.deepResolve(resolved.err) };
            case "option":
                return { kind: "option", some: this.deepResolve(resolved.some) };
        }
    }
    unifyRecord(a, b, span) {
        // Unify shared fields
        for (const [key, aType] of a.fields) {
            const bType = b.fields.get(key);
            if (bType) {
                this.unify(aType, bType, span);
            }
            else if (b.row === undefined) {
                throw new TypeError(`Record is missing field '${key}'`, span);
            }
        }
        for (const [key] of b.fields) {
            if (!a.fields.has(key) && a.row === undefined) {
                throw new TypeError(`Record has unexpected field '${key}'`, span);
            }
        }
        // If both have row variables, unify them to absorb extras
        if (a.row !== undefined && b.row !== undefined) {
            // Build remaining fields from each side
            const aExtras = new Map();
            for (const [k, v] of a.fields) {
                if (!b.fields.has(k))
                    aExtras.set(k, v);
            }
            const bExtras = new Map();
            for (const [k, v] of b.fields) {
                if (!a.fields.has(k))
                    bExtras.set(k, v);
            }
            // Bind row vars to absorb extra fields from the other side
            if (bExtras.size > 0) {
                this.substitution.set(a.row, { kind: "record", fields: bExtras });
            }
            if (aExtras.size > 0) {
                this.substitution.set(b.row, { kind: "record", fields: aExtras });
            }
        }
    }
    unifyEffects(a, b, _span) {
        // For now, effect unification is lenient — we merge
        // A more sophisticated system would track effect row variables
    }
    occursCheck(id, t) {
        const resolved = this.resolve(t);
        switch (resolved.kind) {
            case "var":
                return resolved.id === id;
            case "fn":
                return resolved.params.some(p => this.occursCheck(id, p))
                    || this.occursCheck(id, resolved.ret);
            case "named":
                return resolved.args.some(a => this.occursCheck(id, a));
            case "record":
                for (const v of resolved.fields.values()) {
                    if (this.occursCheck(id, v))
                        return true;
                }
                return resolved.row === id;
            case "tuple":
                return resolved.elements.some(e => this.occursCheck(id, e));
            case "list":
                return this.occursCheck(id, resolved.element);
            case "process":
                return this.occursCheck(id, resolved.msg);
            case "pid":
                return this.occursCheck(id, resolved.msg);
            case "result":
                return this.occursCheck(id, resolved.ok) || this.occursCheck(id, resolved.err);
            case "option":
                return this.occursCheck(id, resolved.some);
            default:
                return false;
        }
    }
}
//# sourceMappingURL=unify.js.map
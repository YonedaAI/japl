import { monotype, freshVar, PURE, INT, STRING, UNIT } from './types.js';
export class TypeEnv {
    scopes = [new Map()];
    typeDefinitions = new Map();
    constructors = new Map();
    traits = new Map();
    pushScope() {
        this.scopes.push(new Map());
    }
    popScope() {
        if (this.scopes.length > 1) {
            this.scopes.pop();
        }
    }
    bind(name, scheme) {
        const top = this.scopes[this.scopes.length - 1];
        top.set(name, scheme);
    }
    lookup(name) {
        for (let i = this.scopes.length - 1; i >= 0; i--) {
            const found = this.scopes[i].get(name);
            if (found)
                return found;
        }
        return undefined;
    }
    lookupConstructor(name) {
        return this.constructors.get(name);
    }
    defineType(name, def) {
        this.typeDefinitions.set(name, def);
    }
    lookupType(name) {
        return this.typeDefinitions.get(name);
    }
    defineConstructor(name, info) {
        this.constructors.set(name, info);
    }
    defineTrait(name, def) {
        this.traits.set(name, def);
    }
    lookupTrait(name) {
        return this.traits.get(name);
    }
    /** Seed the environment with built-in types and constructors. */
    seedBuiltins() {
        // Option[a] = Some(a) | None
        this.defineType("Option", {
            name: "Option",
            typeParams: ["a"],
            variants: [
                { name: "Some", fields: [{ kind: "var", id: -1 }] },
                { name: "None", fields: [] },
            ],
        });
        // Result[a, e] = Ok(a) | Err(e)
        this.defineType("Result", {
            name: "Result",
            typeParams: ["a", "e"],
            variants: [
                { name: "Ok", fields: [{ kind: "var", id: -1 }] },
                { name: "Err", fields: [{ kind: "var", id: -2 }] },
            ],
        });
        // Constructors for Option
        this.defineConstructor("Some", {
            typeName: "Option",
            typeParams: ["a"],
            fieldTypes: [{ kind: "var", id: -1 }],
            resultType: { kind: "option", some: { kind: "var", id: -1 } },
        });
        this.defineConstructor("None", {
            typeName: "Option",
            typeParams: ["a"],
            fieldTypes: [],
            resultType: { kind: "option", some: { kind: "var", id: -1 } },
        });
        // Constructors for Result
        this.defineConstructor("Ok", {
            typeName: "Result",
            typeParams: ["a", "e"],
            fieldTypes: [{ kind: "var", id: -1 }],
            resultType: { kind: "result", ok: { kind: "var", id: -1 }, err: { kind: "var", id: -2 } },
        });
        this.defineConstructor("Err", {
            typeName: "Result",
            typeParams: ["a", "e"],
            fieldTypes: [{ kind: "var", id: -2 }],
            resultType: { kind: "result", ok: { kind: "var", id: -1 }, err: { kind: "var", id: -2 } },
        });
        // Bool constructors (True/False are also constructors in JAPL)
        this.defineConstructor("True", {
            typeName: "Bool",
            typeParams: [],
            fieldTypes: [],
            resultType: { kind: "bool" },
        });
        this.defineConstructor("False", {
            typeName: "Bool",
            typeParams: [],
            fieldTypes: [],
            resultType: { kind: "bool" },
        });
        // IO builtins — println, print have IO effect
        const ioEffect = { effects: new Set(["io"]), open: false };
        const anyVar = freshVar();
        this.bind("println", monotype({
            kind: "fn",
            params: [anyVar],
            ret: UNIT,
            effects: ioEffect,
        }));
        const anyVar2 = freshVar();
        this.bind("print", monotype({
            kind: "fn",
            params: [anyVar2],
            ret: UNIT,
            effects: ioEffect,
        }));
        // LLM builtin — llm(prompt: String) -> String with LLM effect
        const llmEffect = { effects: new Set(["llm"]), open: false };
        this.bind("llm", monotype({
            kind: "fn",
            params: [STRING],
            ret: STRING,
            effects: llmEffect,
        }));
        // Pure builtins — show, int_to_string, string_length
        const showVar = freshVar();
        this.bind("show", monotype({
            kind: "fn",
            params: [showVar],
            ret: STRING,
            effects: PURE,
        }));
        this.bind("int_to_string", monotype({
            kind: "fn",
            params: [INT],
            ret: STRING,
            effects: PURE,
        }));
        this.bind("string_length", monotype({
            kind: "fn",
            params: [STRING],
            ret: INT,
            effects: PURE,
        }));
        // Num trait — generic numeric operations
        const typeVar = { kind: "var", id: -100 };
        this.defineTrait('Num', {
            name: 'Num',
            typeParam: 'a',
            supertraits: [],
            methods: [
                { name: 'add', params: [typeVar, typeVar], ret: typeVar },
                { name: 'sub', params: [typeVar, typeVar], ret: typeVar },
                { name: 'mul', params: [typeVar, typeVar], ret: typeVar },
                { name: 'zero', params: [], ret: typeVar },
            ],
        });
        // impl Num for Int (registered as trait impl)
        // impl Num for Float (registered as trait impl)
        // impl Num for Byte (registered as trait impl)
    }
}
//# sourceMappingURL=env.js.map
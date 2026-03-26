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
    }
}
//# sourceMappingURL=env.js.map
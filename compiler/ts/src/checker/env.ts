import { Type, TypeScheme, monotype, PURE } from './types.js';

export type TypeDef = {
  name: string;
  typeParams: string[];
  variants: VariantDef[];
};

export type VariantDef = {
  name: string;
  fields: Type[];
};

export type ConstructorInfo = {
  typeName: string;
  typeParams: string[];
  fieldTypes: Type[];
  /** The full return type when all type params are fresh vars */
  resultType: Type;
};

export type TraitDef = {
  name: string;
  typeParam: string;
  supertraits: string[];
  methods: TraitMethodSig[];
};

export type TraitMethodSig = {
  name: string;
  params: Type[];
  ret: Type;
};

export class TypeEnv {
  private scopes: Map<string, TypeScheme>[] = [new Map()];
  private typeDefinitions: Map<string, TypeDef> = new Map();
  private constructors: Map<string, ConstructorInfo> = new Map();
  private traits: Map<string, TraitDef> = new Map();

  pushScope(): void {
    this.scopes.push(new Map());
  }

  popScope(): void {
    if (this.scopes.length > 1) {
      this.scopes.pop();
    }
  }

  bind(name: string, scheme: TypeScheme): void {
    const top = this.scopes[this.scopes.length - 1];
    top.set(name, scheme);
  }

  lookup(name: string): TypeScheme | undefined {
    for (let i = this.scopes.length - 1; i >= 0; i--) {
      const found = this.scopes[i].get(name);
      if (found) return found;
    }
    return undefined;
  }

  lookupConstructor(name: string): ConstructorInfo | undefined {
    return this.constructors.get(name);
  }

  defineType(name: string, def: TypeDef): void {
    this.typeDefinitions.set(name, def);
  }

  lookupType(name: string): TypeDef | undefined {
    return this.typeDefinitions.get(name);
  }

  defineConstructor(name: string, info: ConstructorInfo): void {
    this.constructors.set(name, info);
  }

  defineTrait(name: string, def: TraitDef): void {
    this.traits.set(name, def);
  }

  lookupTrait(name: string): TraitDef | undefined {
    return this.traits.get(name);
  }

  /** Seed the environment with built-in types and constructors. */
  seedBuiltins(): void {
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

import { Type, TypeScheme } from './types.js';
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
export declare class TypeEnv {
    private scopes;
    private typeDefinitions;
    private constructors;
    private traits;
    pushScope(): void;
    popScope(): void;
    bind(name: string, scheme: TypeScheme): void;
    lookup(name: string): TypeScheme | undefined;
    lookupConstructor(name: string): ConstructorInfo | undefined;
    defineType(name: string, def: TypeDef): void;
    lookupType(name: string): TypeDef | undefined;
    defineConstructor(name: string, info: ConstructorInfo): void;
    defineTrait(name: string, def: TraitDef): void;
    lookupTrait(name: string): TraitDef | undefined;
    /** Seed the environment with built-in types and constructors. */
    seedBuiltins(): void;
}

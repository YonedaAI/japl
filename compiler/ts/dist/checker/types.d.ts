export type Type = {
    kind: "int";
} | {
    kind: "float";
} | {
    kind: "byte";
} | {
    kind: "string";
} | {
    kind: "bool";
} | {
    kind: "unit";
} | {
    kind: "never";
} | {
    kind: "var";
    id: number;
} | {
    kind: "named";
    name: string;
    args: Type[];
} | {
    kind: "fn";
    params: Type[];
    ret: Type;
    effects: EffectRow;
} | {
    kind: "record";
    fields: Map<string, Type>;
    row?: number;
} | {
    kind: "tuple";
    elements: Type[];
} | {
    kind: "list";
    element: Type;
} | {
    kind: "process";
    msg: Type;
} | {
    kind: "pid";
    msg: Type;
} | {
    kind: "result";
    ok: Type;
    err: Type;
} | {
    kind: "option";
    some: Type;
};
export type Effect = "pure" | "io" | "async" | "process" | "fail" | "llm";
export type EffectRow = {
    effects: Set<Effect>;
    open: boolean;
};
export declare const PURE: EffectRow;
export declare const IO: EffectRow;
export type TypeScheme = {
    vars: number[];
    type: Type;
};
export declare const INT: Type;
export declare const FLOAT: Type;
export declare const BYTE: Type;
export declare const STRING: Type;
export declare const BOOL: Type;
export declare const UNIT: Type;
export declare const NEVER: Type;
export declare function resetVarCounter(): void;
export declare function freshVar(): Type;
export declare function currentVarId(): number;
export declare function typeToString(t: Type): string;
export declare function effectRowToString(e: EffectRow): string;
export declare function monotype(t: Type): TypeScheme;
/** Collect all free type variable ids in a type. */
export declare function freeVars(t: Type): Set<number>;

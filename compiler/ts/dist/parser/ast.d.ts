import { Span } from '../lexer/token.js';
export type Module = {
    decls: Decl[];
    span: Span;
};
export type Decl = {
    kind: "fn";
    name: string;
    params: Param[];
    returnType?: TypeExpr;
    effects?: EffectExpr;
    body: Expr;
    pub: boolean;
    span: Span;
} | {
    kind: "type";
    name: string;
    typeParams: string[];
    variants: Variant[];
    span: Span;
} | {
    kind: "record_type";
    name: string;
    typeParams: string[];
    fields: Field[];
    span: Span;
} | {
    kind: "trait";
    name: string;
    typeParam: string;
    supertraits: string[];
    methods: FnSig[];
    span: Span;
} | {
    kind: "impl";
    traitName: string;
    typeName: string;
    methods: Decl[];
    span: Span;
} | {
    kind: "module";
    name: string;
    decls: Decl[];
    span: Span;
} | {
    kind: "import";
    path: string[];
    items: string[];
    span: Span;
} | {
    kind: "test";
    name: string;
    body: Expr;
    span: Span;
} | {
    kind: "supervisor";
    name: string;
    strategy: string;
    children: Expr[];
    span: Span;
} | {
    kind: "foreign";
    name: string;
    params: Param[];
    returnType: TypeExpr;
    span: Span;
};
export type Param = {
    name: string;
    type?: TypeExpr;
    span: Span;
};
export type Variant = {
    name: string;
    fields: TypeExpr[];
    span: Span;
};
export type Field = {
    name: string;
    type: TypeExpr;
    span: Span;
};
export type FnSig = {
    name: string;
    params: Param[];
    returnType?: TypeExpr;
    span: Span;
};
export type Expr = {
    kind: "int";
    value: number;
    span: Span;
} | {
    kind: "float";
    value: number;
    span: Span;
} | {
    kind: "string";
    value: string;
    span: Span;
} | {
    kind: "bool";
    value: boolean;
    span: Span;
} | {
    kind: "unit";
    span: Span;
} | {
    kind: "var";
    name: string;
    span: Span;
} | {
    kind: "constructor";
    name: string;
    args: Expr[];
    span: Span;
} | {
    kind: "app";
    fn: Expr;
    args: Expr[];
    span: Span;
} | {
    kind: "lambda";
    params: Param[];
    body: Expr;
    span: Span;
} | {
    kind: "let";
    name: string;
    type?: TypeExpr;
    value: Expr;
    body: Expr;
    span: Span;
} | {
    kind: "match";
    scrutinee: Expr;
    arms: MatchArm[];
    span: Span;
} | {
    kind: "if";
    condition: Expr;
    then: Expr;
    else?: Expr;
    span: Span;
} | {
    kind: "pipe";
    left: Expr;
    right: Expr;
    span: Span;
} | {
    kind: "binop";
    op: string;
    left: Expr;
    right: Expr;
    span: Span;
} | {
    kind: "unaryop";
    op: string;
    operand: Expr;
    span: Span;
} | {
    kind: "record";
    fields: [string, Expr][];
    span: Span;
} | {
    kind: "field_access";
    expr: Expr;
    field: string;
    span: Span;
} | {
    kind: "record_update";
    record: Expr;
    fields: [string, Expr][];
    span: Span;
} | {
    kind: "list";
    elements: Expr[];
    span: Span;
} | {
    kind: "block";
    exprs: Expr[];
    span: Span;
} | {
    kind: "spawn";
    expr: Expr;
    span: Span;
} | {
    kind: "send";
    target: Expr;
    message: Expr;
    span: Span;
} | {
    kind: "receive";
    arms: MatchArm[];
    span: Span;
} | {
    kind: "try";
    expr: Expr;
    span: Span;
} | {
    kind: "return";
    expr?: Expr;
    span: Span;
};
export type MatchArm = {
    pattern: Pattern;
    guard?: Expr;
    body: Expr;
    span: Span;
};
export type Pattern = {
    kind: "pvar";
    name: string;
    span: Span;
} | {
    kind: "pconstructor";
    name: string;
    args: Pattern[];
    span: Span;
} | {
    kind: "pliteral";
    value: Expr;
    span: Span;
} | {
    kind: "pwildcard";
    span: Span;
} | {
    kind: "precord";
    fields: [string, Pattern][];
    span: Span;
} | {
    kind: "plist";
    elements: Pattern[];
    rest?: string;
    span: Span;
} | {
    kind: "ptuple";
    elements: Pattern[];
    span: Span;
};
export type TypeExpr = {
    kind: "tnamed";
    name: string;
    args: TypeExpr[];
    span: Span;
} | {
    kind: "tfn";
    params: TypeExpr[];
    ret: TypeExpr;
    span: Span;
} | {
    kind: "trecord";
    fields: [string, TypeExpr][];
    row?: string;
    span: Span;
} | {
    kind: "ttuple";
    elements: TypeExpr[];
    span: Span;
} | {
    kind: "tunit";
    span: Span;
} | {
    kind: "tvar";
    name: string;
    span: Span;
};
export type EffectExpr = {
    effects: string[];
    span: Span;
};

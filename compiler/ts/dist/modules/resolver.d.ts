import * as AST from '../parser/ast.js';
export interface ResolvedModule {
    name: string;
    path: string;
    ast: AST.Module;
    exports: Map<string, AST.Decl>;
    allDecls: Map<string, AST.Decl>;
}
export declare class ModuleResolver {
    private cache;
    private searchPaths;
    constructor(searchPaths: string[]);
    resolve(moduleName: string, fromFile: string): ResolvedModule;
}
export declare class ModuleError extends Error {
    constructor(message: string);
}

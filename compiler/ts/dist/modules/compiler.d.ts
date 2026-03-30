export interface CompiledFile {
    /** Original .japl source path */
    sourcePath: string;
    /** Module name (e.g., "Math") */
    moduleName: string;
    /** Generated TypeScript code */
    code: string;
    /** Whether this is the entry file */
    isEntry: boolean;
}
export interface CompileResult {
    files: CompiledFile[];
    errors: string[];
}
export declare class MultiFileCompiler {
    private resolver;
    private modules;
    private errors;
    constructor(searchPaths?: string[]);
    compile(entryFile: string): CompileResult;
    private discoverModules;
    private topologicalSort;
}

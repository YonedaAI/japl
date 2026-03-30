import { CompileResult } from '../modules/compiler.js';
export type Target = 'ts' | 'c';
export interface BuildOptions {
    target?: Target;
    strict?: boolean;
}
export declare function buildToString(source: string, targetOrOpts?: Target | BuildOptions): string;
/**
 * Multi-file build: resolves imports, compiles all dependencies.
 * Returns the CompileResult with all generated files.
 */
export declare function buildMultiFile(inputPath: string, extraSearchPaths?: string[]): CompileResult;
/**
 * Multi-file build that writes all output files to a directory.
 * Returns the path to the entry file's generated .ts file.
 */
export declare function buildMultiFileTo(inputPath: string, outDir: string, extraSearchPaths?: string[]): string;
export declare function buildFile(inputPath: string, outputPath?: string, target?: Target): void;

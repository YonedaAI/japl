export interface BuildOptions {
    strict?: boolean;
    emitWat?: boolean;
}
export declare function buildToWat(source: string, opts?: BuildOptions): string;
export declare function buildFile(inputPath: string, outputPath?: string): void;
export declare function checkTools(tools: string[]): void;
export declare function buildToWasm(inputPath: string, outputDir?: string): string;
export declare function runWasm(wasmPath: string): void;

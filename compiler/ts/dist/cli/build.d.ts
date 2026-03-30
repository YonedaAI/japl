export interface BuildOptions {
    strict?: boolean;
    emitWat?: boolean;
}
export declare function buildToWat(source: string, opts?: BuildOptions): string;
export declare function buildFile(inputPath: string, outputPath?: string): void;

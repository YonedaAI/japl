export type Target = 'ts' | 'c';
export declare function buildToString(source: string, target?: Target): string;
export declare function buildFile(inputPath: string, outputPath?: string, target?: Target): void;

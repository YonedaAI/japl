export interface JaplConfig {
    package?: {
        name?: string;
        version?: string;
        entry?: string;
    };
    dependencies?: Record<string, string>;
    'dev-dependencies'?: Record<string, string>;
    [section: string]: Record<string, string> | undefined;
}
export declare function parseConfig(source: string): JaplConfig;
export declare function loadConfig(filePath: string): JaplConfig;

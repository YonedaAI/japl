export interface NodeConfig {
    name?: string;
    listen?: string;
    cookie?: string;
    connect?: string[];
}
export interface JaplConfig {
    package?: {
        name?: string;
        version?: string;
        entry?: string;
    };
    dependencies?: Record<string, string>;
    'dev-dependencies'?: Record<string, string>;
    node?: NodeConfig;
    [section: string]: Record<string, string> | NodeConfig | undefined;
}
export declare function parseConfig(source: string): JaplConfig;
export declare function loadConfig(filePath: string): JaplConfig;

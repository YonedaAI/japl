#!/usr/bin/env node
export interface DistributedNodeConfig {
    name: string;
    listen?: string;
    connect?: string[];
    cookie: string;
}
export declare function parseRunArgs(args: string[]): {
    flags: Record<string, string>;
    positional: string[];
};
export declare function extractNodeConfig(flags: Record<string, string>): DistributedNodeConfig | null;

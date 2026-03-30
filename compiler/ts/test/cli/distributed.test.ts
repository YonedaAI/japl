import { describe, it, expect } from 'vitest';
import { parseConfig } from '../../src/cli/config.js';
import type { NodeConfig } from '../../src/cli/config.js';

// NOTE: parseRunArgs and extractNodeConfig were removed from the CLI
// when the TS/C run command was removed. The distributed runtime will
// be re-implemented on top of the WASM backend.
// Config parsing tests are kept since they test the TOML parser.

describe('japl.toml [node] section', () => {
  it('parses [node] section with all fields', () => {
    const toml = `[package]
name = "myapp"
version = "0.1.0"

[node]
name = "alpha"
listen = ":9000"
cookie = "secret"
connect = ["beta:9001"]
`;
    const config = parseConfig(toml);
    const node = config.node as NodeConfig;
    expect(node).toBeDefined();
    expect(node.name).toBe('alpha');
    expect(node.listen).toBe(':9000');
    expect(node.cookie).toBe('secret');
    expect(node.connect).toEqual(['beta:9001']);
  });

  it('parses [node] section with multiple connect peers', () => {
    const toml = `[node]
name = "alpha"
listen = ":9000"
cookie = "secret"
connect = ["beta:9001", "gamma:9002"]
`;
    const config = parseConfig(toml);
    const node = config.node as NodeConfig;
    expect(node.connect).toEqual(['beta:9001', 'gamma:9002']);
  });

  it('parses [node] section without connect', () => {
    const toml = `[node]
name = "alpha"
listen = ":9000"
cookie = "secret"
`;
    const config = parseConfig(toml);
    const node = config.node as NodeConfig;
    expect(node.name).toBe('alpha');
    expect(node.listen).toBe(':9000');
    expect(node.cookie).toBe('secret');
    expect(node.connect).toBeUndefined();
  });

  it('handles empty connect array', () => {
    const toml = `[node]
name = "alpha"
connect = []
`;
    const config = parseConfig(toml);
    const node = config.node as NodeConfig;
    expect(node.connect).toEqual([]);
  });

  it('config without [node] section has no node key', () => {
    const toml = `[package]
name = "myapp"
version = "0.1.0"
`;
    const config = parseConfig(toml);
    expect(config.node).toBeUndefined();
  });
});

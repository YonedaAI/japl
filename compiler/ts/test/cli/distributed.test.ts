import { describe, it, expect } from 'vitest';
import { parseRunArgs, extractNodeConfig } from '../../src/index.js';
import { parseConfig } from '../../src/cli/config.js';
import type { NodeConfig } from '../../src/cli/config.js';

describe('CLI distributed flags', () => {
  it('parses --node flag', () => {
    const { flags, positional } = parseRunArgs(['--node', 'alpha', 'main.japl']);
    expect(flags['node']).toBe('alpha');
    expect(positional).toEqual(['main.japl']);
  });

  it('parses --listen flag', () => {
    const { flags } = parseRunArgs(['--node', 'alpha', '--listen', ':9000', 'main.japl']);
    expect(flags['listen']).toBe(':9000');
  });

  it('parses --connect flag', () => {
    const { flags } = parseRunArgs(['--node', 'alpha', '--connect', 'beta:9001,gamma:9002', 'main.japl']);
    expect(flags['connect']).toBe('beta:9001,gamma:9002');

    const config = extractNodeConfig(flags);
    expect(config).not.toBeNull();
    expect(config!.connect).toEqual(['beta:9001', 'gamma:9002']);
  });

  it('parses --cookie flag', () => {
    const { flags } = parseRunArgs(['--node', 'alpha', '--cookie', 'my-secret', 'main.japl']);
    const config = extractNodeConfig(flags);
    expect(config).not.toBeNull();
    expect(config!.cookie).toBe('my-secret');
  });

  it('uses default cookie when --cookie not provided', () => {
    const { flags } = parseRunArgs(['--node', 'alpha', 'main.japl']);
    const config = extractNodeConfig(flags);
    expect(config).not.toBeNull();
    expect(config!.cookie).toBe('japl-default-cookie');
  });

  it('returns null config when --node not present', () => {
    const { flags } = parseRunArgs(['main.japl']);
    const config = extractNodeConfig(flags);
    expect(config).toBeNull();
  });

  it('extractNodeConfig builds full config', () => {
    const { flags } = parseRunArgs([
      '--node', 'alpha',
      '--listen', ':9000',
      '--connect', 'beta:9001',
      '--cookie', 'secret',
      'main.japl',
    ]);
    const config = extractNodeConfig(flags);
    expect(config).toEqual({
      name: 'alpha',
      listen: ':9000',
      connect: ['beta:9001'],
      cookie: 'secret',
    });
  });
});

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

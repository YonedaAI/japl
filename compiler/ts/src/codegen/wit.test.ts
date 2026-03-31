import { describe, it, expect } from 'vitest';
import { generateWit } from './wit.js';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import type { Module } from '../parser/ast.js';

function witFromSource(source: string, pkg = 'test'): string {
  const tokens = new Lexer(source).tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  return generateWit(ast, pkg);
}

describe('WIT generator — sum types', () => {
  it('generates WIT for simple enum sum type', () => {
    const wit = witFromSource(`
      type Color = | Red | Green | Blue
    `);
    expect(wit).toContain('variant color {');
    expect(wit).toContain('red,');
    expect(wit).toContain('green,');
    expect(wit).toContain('blue,');
  });

  it('generates WIT for sum type with fields', () => {
    const wit = witFromSource(`
      type Result = | Ok(Int) | Err(String)
    `);
    expect(wit).toContain('variant result {');
    expect(wit).toContain('ok(s64),');
    expect(wit).toContain('err(string),');
  });

  it('generates WIT for sum type with multi-field variant', () => {
    const wit = witFromSource(`
      type Shape = | Circle(Float) | Rect(Float, Float)
    `);
    expect(wit).toContain('variant shape {');
    expect(wit).toContain('circle(f64),');
    expect(wit).toContain('rect(tuple<f64, f64>),');
  });
});

describe('WIT generator — record types', () => {
  it('generates WIT for record type', () => {
    const wit = witFromSource(`
      type User = { name: String, age: Int }
    `);
    expect(wit).toContain('record user {');
    expect(wit).toContain('name: string,');
    expect(wit).toContain('age: s64,');
  });

  it('generates WIT for record with Bool and Float fields', () => {
    const wit = witFromSource(`
      type Config = { enabled: Bool, rate: Float }
    `);
    expect(wit).toContain('record config {');
    expect(wit).toContain('enabled: bool,');
    expect(wit).toContain('rate: f64,');
  });
});

describe('WIT generator — functions', () => {
  it('generates WIT for pub function', () => {
    const wit = witFromSource(`
      pub fn handle(req: String) -> String { req }
    `);
    expect(wit).toContain('handle: func(req: string) -> string;');
  });

  it('skips non-pub functions', () => {
    const wit = witFromSource(`
      fn private_fn(x: Int) -> Int { x }
    `);
    expect(wit).not.toContain('private-fn');
    // Should still have world even with no exports in interface
    expect(wit).toContain('world test-world {');
  });

  it('generates WIT for function with no return type', () => {
    const wit = witFromSource(`
      pub fn log(msg: String) { msg }
    `);
    expect(wit).toContain('log: func(msg: string);');
  });

  it('generates WIT for function with multiple params', () => {
    const wit = witFromSource(`
      pub fn add(a: Int, b: Int) -> Int { a }
    `);
    expect(wit).toContain('add: func(a: s64, b: s64) -> s64;');
  });
});

describe('WIT generator — type mapping', () => {
  it('maps all JAPL primitive types to WIT types', () => {
    const wit = witFromSource(`
      pub fn transform(a: Int, b: Float, c: Bool, d: Byte) -> String { "ok" }
    `);
    expect(wit).toContain('a: s64');
    expect(wit).toContain('b: f64');
    expect(wit).toContain('c: bool');
    expect(wit).toContain('d: u8');
    expect(wit).toContain('-> string');
  });
});

describe('WIT generator — naming conventions', () => {
  it('converts PascalCase to kebab-case', () => {
    const wit = witFromSource(`
      type MyComplexType = | VariantOne | VariantTwo
    `);
    expect(wit).toContain('variant my-complex-type');
    expect(wit).toContain('variant-one');
    expect(wit).toContain('variant-two');
  });

  it('converts camelCase function names to kebab-case', () => {
    const wit = witFromSource(`
      pub fn handleRequest(r: String) -> String { r }
    `);
    expect(wit).toContain('handle-request: func');
  });
});

describe('WIT generator — world and package', () => {
  it('generates WIT world', () => {
    const wit = witFromSource(`
      pub fn hello() -> String { "hi" }
    `, 'myapp');
    expect(wit).toContain('package japl:myapp;');
    expect(wit).toContain('world myapp-world {');
    expect(wit).toContain('export myapp;');
  });

  it('generates empty world when no exports', () => {
    const wit = witFromSource(`
      fn internal() -> Int { 42 }
    `, 'lib');
    expect(wit).toContain('package japl:lib;');
    expect(wit).toContain('world lib-world {');
    expect(wit).not.toContain('export');
  });
});

describe('WIT generator — KV store integration', () => {
  it('handles KV store types', () => {
    const wit = witFromSource(`
      type KVRequest = | Get(String) | Put(String, String) | Delete(String)
      type KVResponse = | Found(String) | NotFound | Stored
      pub fn handle(req: KVRequest) -> KVResponse { Found("test") }
    `, 'kvstore');
    expect(wit).toContain('package japl:kvstore;');
    expect(wit).toContain('variant kv-request');
    expect(wit).toContain('get(string)');
    expect(wit).toContain('put(tuple<string, string>)');
    expect(wit).toContain('delete(string)');
    expect(wit).toContain('variant kv-response');
    expect(wit).toContain('found(string)');
    expect(wit).toContain('not-found,');
    expect(wit).toContain('stored,');
    expect(wit).toContain('handle: func(req: kv-request) -> kv-response;');
  });
});

describe('WIT generator — mixed declarations', () => {
  it('handles types and functions together', () => {
    const wit = witFromSource(`
      type Status = | Active | Inactive
      type User = { name: String, status: Status }
      pub fn create(name: String) -> User { { name: name, status: Active } }
      pub fn deactivate(u: User) -> User { u }
    `, 'users');
    expect(wit).toContain('package japl:users;');
    expect(wit).toContain('interface users {');
    expect(wit).toContain('variant status {');
    expect(wit).toContain('record user {');
    expect(wit).toContain('create: func(name: string) -> user;');
    expect(wit).toContain('deactivate: func(u: user) -> user;');
    expect(wit).toContain('world users-world {');
    expect(wit).toContain('export users;');
  });
});

import { describe, it, expect } from 'vitest';
import { Lexer } from '../../src/lexer/lexer.js';
import { Parser } from '../../src/parser/parser.js';

// NOTE: FFI end-to-end tests removed — they tested TS-specific codegen and
// Node.js runtime execution. These will be rewritten for the WASM backend.
// The parsing tests below verify that FFI syntax is still correctly parsed.

function parse(source: string) {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(", ")}`);
  }
  return ast;
}

describe("FFI: parsing", () => {

  it("parses foreign fn with module", () => {
    const ast = parse(`foreign "node:fs" fn readFileSync(path: String, encoding: String) -> String`);
    expect(ast.decls.length).toBe(1);
    expect(ast.decls[0].kind).toBe('foreign');
  });

  it("parses foreign fn without module", () => {
    const ast = parse(`foreign fn customBuiltin(x: String) -> String`);
    expect(ast.decls.length).toBe(1);
    expect(ast.decls[0].kind).toBe('foreign');
  });

  it("parses foreign fn with as alias", () => {
    const ast = parse(`foreign "node:fs" fn read_file as "readFileSync"(path: String, encoding: String) -> String`);
    expect(ast.decls.length).toBe(1);
    expect(ast.decls[0].kind).toBe('foreign');
  });

  it("parses multiple foreign declarations", () => {
    const ast = parse(`
foreign "node:fs" fn readFileSync(path: String, encoding: String) -> String
foreign "node:fs" fn existsSync(path: String) -> Bool
foreign "node:os" fn hostname() -> String
`);
    expect(ast.decls.length).toBe(3);
    for (const decl of ast.decls) {
      expect(decl.kind).toBe('foreign');
    }
  });

  it("parses scoped package imports", () => {
    const ast = parse(`foreign "@some/pkg" fn doSomething() -> String`);
    expect(ast.decls.length).toBe(1);
    expect(ast.decls[0].kind).toBe('foreign');
  });
});

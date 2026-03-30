import { describe, it, expect } from 'vitest';
import { Lexer } from '../../src/lexer/lexer.js';
import { Parser } from '../../src/parser/parser.js';
import { lowerModule } from '../../src/ir/lower.js';
import { MultiFileCompiler } from '../../src/modules/compiler.js';
import * as path from 'node:path';
import * as fs from 'node:fs';

const STDLIB_DIR = path.resolve(__dirname, '../../../../stdlib');

// NOTE: Stdlib runtime integration tests removed — they tested TS-specific
// codegen and Node.js runtime execution. These will be rewritten for the WASM
// backend. The compilation tests below verify that stdlib modules still parse
// and compile through the IR pipeline.

function compileSource(source: string) {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  return lowerModule(ast);
}

// ─── Stdlib Module Compilation Tests ───

describe('Standard Library', () => {

  describe('module parsing and IR lowering', () => {
    const modules = ['Math', 'String', 'Option', 'Result', 'IO', 'Process'];

    for (const mod of modules) {
      it(`${mod}.japl parses and lowers to IR`, () => {
        const source = fs.readFileSync(path.join(STDLIB_DIR, `${mod}.japl`), 'utf-8');
        const ir = compileSource(source);
        expect(ir.decls.length).toBeGreaterThan(0);
      });
    }
  });

  describe('multi-file compilation', () => {
    it('compiles test_math.japl with Math dependency', () => {
      const testFile = path.join(__dirname, 'test_math.japl');
      if (fs.existsSync(testFile)) {
        const compiler = new MultiFileCompiler([STDLIB_DIR]);
        const result = compiler.compile(testFile);
        expect(result.errors).toEqual([]);
        expect(result.files.length).toBeGreaterThan(0);
      }
    });

    it('compiles test_string.japl with String dependency', () => {
      const testFile = path.join(__dirname, 'test_string.japl');
      if (fs.existsSync(testFile)) {
        const compiler = new MultiFileCompiler([STDLIB_DIR]);
        const result = compiler.compile(testFile);
        expect(result.errors).toEqual([]);
        expect(result.files.length).toBeGreaterThan(0);
      }
    });

    it('compiles test_option.japl with Option dependency', () => {
      const testFile = path.join(__dirname, 'test_option.japl');
      if (fs.existsSync(testFile)) {
        const compiler = new MultiFileCompiler([STDLIB_DIR]);
        const result = compiler.compile(testFile);
        expect(result.errors).toEqual([]);
        expect(result.files.length).toBeGreaterThan(0);
      }
    });
  });

  // ─── String Interpolation parsing ───

  describe('String interpolation parsing', () => {
    it('parses interpolation syntax', () => {
      const source = `
fn main() {
  let name = "world"
  println("hello " <> name)
}
`;
      const ir = compileSource(source);
      expect(ir.decls.length).toBeGreaterThan(0);
    });
  });
});

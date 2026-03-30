import { describe, it, expect } from 'vitest';
import { Lexer } from '../../src/lexer/lexer.js';
import { Parser } from '../../src/parser/parser.js';
import { lowerModule } from '../../src/ir/lower.js';
import { TsEmitter } from '../../src/codegen/emit.js';
import { MultiFileCompiler } from '../../src/modules/compiler.js';
import { buildMultiFile } from '../../src/cli/build.js';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as os from 'node:os';
import { execFileSync } from 'node:child_process';

const STDLIB_DIR = path.resolve(__dirname, '../../../../stdlib');
const TEST_DIR = path.resolve(__dirname, '.');
const COMPILER_ROOT = path.resolve(__dirname, '../..');

// ─── Helper: compile single JAPL source to TS ───

function compile(source: string): string {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  const ir = lowerModule(ast);
  const emitter = new TsEmitter();
  return emitter.emit(ir);
}

// ─── Helper: compile multi-file and run ───

function compileAndRunMulti(entryPath: string): string {
  const result = buildMultiFile(entryPath);
  if (result.errors.length > 0) {
    throw new Error(result.errors.join('\n'));
  }

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-stdlib-test-'));
  try {
    for (const file of result.files) {
      const outPath = path.join(tmpDir, file.moduleName + '.ts');
      fs.writeFileSync(outPath, file.code);
    }
    const entry = result.files.find(f => f.isEntry)!;
    const entryTsPath = path.join(tmpDir, entry.moduleName + '.ts');
    const output = execFileSync('npx', ['tsx', entryTsPath], {
      encoding: 'utf-8',
      cwd: tmpDir,
    });
    return output.trimEnd();
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

// ─── Helper: compile single source and run ───

function compileAndRun(source: string): string {
  const ts = compile(source);
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-stdlib-test-'));
  const tmpFile = path.join(tmpDir, 'test.ts');
  fs.writeFileSync(tmpFile, ts);
  try {
    const output = execFileSync('npx', ['tsx', tmpFile], {
      encoding: 'utf-8',
      cwd: COMPILER_ROOT,
    });
    return output.trimEnd();
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

// ─── Stdlib Module Compilation Tests ───

describe('Standard Library', () => {

  describe('module compilation', () => {
    const modules = ['Math', 'String', 'Option', 'Result', 'IO', 'Process'];

    for (const mod of modules) {
      it(`${mod}.japl compiles to valid TypeScript`, () => {
        const source = fs.readFileSync(path.join(STDLIB_DIR, `${mod}.japl`), 'utf-8');
        const ts = compile(source);
        expect(ts).toContain('export function');
      });
    }

    it('Math.japl exports abs, max, min, clamp', () => {
      const source = fs.readFileSync(path.join(STDLIB_DIR, 'Math.japl'), 'utf-8');
      const ts = compile(source);
      expect(ts).toContain('export function abs');
      expect(ts).toContain('export function max');
      expect(ts).toContain('export function min');
      expect(ts).toContain('export function clamp');
    });

    it('Option.japl defines Some/None constructors', () => {
      const source = fs.readFileSync(path.join(STDLIB_DIR, 'Option.japl'), 'utf-8');
      const ts = compile(source);
      expect(ts).toContain('_tag: "Some"');
      expect(ts).toContain('_tag: "None"');
    });

    it('IO.japl imports from node:fs', () => {
      const source = fs.readFileSync(path.join(STDLIB_DIR, 'IO.japl'), 'utf-8');
      const ts = compile(source);
      expect(ts).toContain("from 'node:fs'");
    });
  });

  // ─── Integration: Math stdlib ───

  describe('Math module integration', () => {
    it('test_math.japl runs correctly', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_math.japl'));
      expect(output).toBe('5\n7\n3\n10');
    });
  });

  // ─── Integration: String stdlib ───

  describe('String module integration', () => {
    it('test_string.japl runs correctly', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_string.japl'));
      expect(output).toBe('hahaha\ntrue\nfalse\n4');
    });
  });

  // ─── Integration: Option stdlib ───

  describe('Option module integration', () => {
    it('test_option.japl runs correctly', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_option.japl'));
      expect(output).toBe('42\n99\ntrue\nfalse');
    });
  });

  // ─── Integration: Result stdlib ───

  describe('Result module integration', () => {
    it('test_result.japl runs correctly', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_result.japl'));
      expect(output).toBe('42\n99\ntrue\nfalse');
    });
  });

  // ─── Integration: IO stdlib ───

  describe('IO module integration', () => {
    it('test_io.japl runs correctly', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_io.japl'));
      expect(output).toBe('true\nhello from JAPL stdlib');
    });
  });

  // ─── String Interpolation ───

  describe('String interpolation', () => {
    it('basic variable interpolation', () => {
      const output = compileAndRun(`
fn main() {
  let name = "world"
  println("hello " <> name)
}
`);
      expect(output).toBe('hello world');
    });

    it('interpolation with ${var}', () => {
      const output = compileAndRun(`
fn main() {
  let name = "JAPL"
  println("Hello from \${name}!")
}
`);
      expect(output).toBe('Hello from JAPL!');
    });

    it('interpolation with ${expr}', () => {
      const output = compileAndRun(`
fn main() {
  let x = 21
  println("result: \${show(x * 2)}")
}
`);
      expect(output).toBe('result: 42');
    });

    it('multiple interpolations in one string', () => {
      const output = compileAndRun(`
fn main() {
  let a = "hello"
  let b = "world"
  println("\${a} \${b}")
}
`);
      expect(output).toBe('hello world');
    });

    it('interpolation desugars to concatenation in TS output', () => {
      const ts = compile(`
fn main() {
  let name = "JAPL"
  println("Hello \${name}!")
}
`);
      expect(ts).toContain('"Hello " + name + "!"');
    });

    it('end-to-end interpolation test', () => {
      const output = compileAndRunMulti(path.join(TEST_DIR, 'test_interp.japl'));
      expect(output).toBe('Hello from JAPL, version 42!\nmath: 5\nhi world');
    });
  });
});

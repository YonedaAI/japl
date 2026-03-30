import { describe, it, expect } from 'vitest';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { WatEmitter } from './emit_wat.js';
import { execFileSync } from 'node:child_process';
import { writeFileSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

function compileToWat(source: string): string {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  const ir = lowerModule(ast);
  const emitter = new WatEmitter();
  return emitter.emit(ir);
}

/** Compile WAT to WASM and run with wasmtime, return stdout */
function runWat(wat: string): string | null {
  let hasWat2wasm = false;
  let hasWasmtime = false;
  try {
    execFileSync('which', ['wat2wasm'], { stdio: 'pipe' });
    hasWat2wasm = true;
  } catch { /* not available */ }
  try {
    execFileSync('which', ['wasmtime'], { stdio: 'pipe' });
    hasWasmtime = true;
  } catch { /* not available */ }

  if (!hasWat2wasm || !hasWasmtime) return null;

  const watPath = join(tmpdir(), `japl-test-${Date.now()}.wat`);
  const wasmPath = watPath.replace('.wat', '.wasm');
  try {
    writeFileSync(watPath, wat);
    execFileSync('wat2wasm', [watPath, '-o', wasmPath], { stdio: 'pipe' });
    const result = execFileSync('wasmtime', [wasmPath], {
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    return result;
  } finally {
    try { unlinkSync(watPath); } catch { /* ignore */ }
    try { unlinkSync(wasmPath); } catch { /* ignore */ }
  }
}

describe('WatEmitter', () => {
  describe('WAT structure', () => {
    it('emits a valid module wrapper', () => {
      const wat = compileToWat('fn main() { println("hi") }');
      expect(wat).toContain('(module');
      expect(wat.trim()).toMatch(/\)$/);
    });

    it('includes WASI imports', () => {
      const wat = compileToWat('fn main() { println("hi") }');
      expect(wat).toContain('(import "wasi_snapshot_preview1" "fd_write"');
      expect(wat).toContain('(import "wasi_snapshot_preview1" "proc_exit"');
    });

    it('exports memory', () => {
      const wat = compileToWat('fn main() { println("hi") }');
      expect(wat).toContain('(memory (export "memory") 1)');
    });

    it('exports _start entry point', () => {
      const wat = compileToWat('fn main() { println("hi") }');
      expect(wat).toContain('(export "_start")');
      expect(wat).toContain('call $main');
    });

    it('emits data segments for strings', () => {
      const wat = compileToWat('fn main() { println("Hello") }');
      expect(wat).toContain('(data (i32.const');
      expect(wat).toContain('Hello');
    });
  });

  describe('expression emission', () => {
    it('emits integer constants as i64', () => {
      const wat = compileToWat('fn add(x: Int, y: Int) -> Int { x + y }\nfn main() { println(show(add(1, 2))) }');
      expect(wat).toContain('i64.const 1');
      expect(wat).toContain('i64.const 2');
    });

    it('emits i64.add for + operator', () => {
      const wat = compileToWat('fn add(x: Int, y: Int) -> Int { x + y }\nfn main() { println(show(add(1, 2))) }');
      expect(wat).toContain('i64.add');
    });

    it('emits i64.sub for - operator', () => {
      const wat = compileToWat('fn sub(x: Int, y: Int) -> Int { x - y }\nfn main() { println(show(sub(5, 3))) }');
      expect(wat).toContain('i64.sub');
    });

    it('emits comparison operators', () => {
      const wat = compileToWat('fn f(n: Int) -> Int { if n <= 1 { n } else { 0 } }\nfn main() { println(show(f(1))) }');
      expect(wat).toContain('i64.le_s');
    });

    it('emits function params and calls', () => {
      const wat = compileToWat('fn double(x: Int) -> Int { x + x }\nfn main() { println(show(double(5))) }');
      expect(wat).toContain('(param $x i64)');
      expect(wat).toContain('call $double');
    });

    it('emits if/else with result type', () => {
      const wat = compileToWat('fn f(x: Int) -> Int { if x < 0 { 0 } else { x } }\nfn main() { println(show(f(1))) }');
      expect(wat).toContain('(if (result i64)');
      expect(wat).toContain('(then');
      expect(wat).toContain('(else');
    });

    it('emits recursive calls', () => {
      const wat = compileToWat(`fn fib(n: Int) -> Int {
        if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
      }
      fn main() { println(show(fib(5))) }`);
      expect(wat).toContain('call $fib');
    });
  });

  describe('built-in functions', () => {
    it('emits $println helper', () => {
      const wat = compileToWat('fn main() { println("test") }');
      expect(wat).toContain('(func $println');
      expect(wat).toContain('call $fd_write');
    });

    it('emits $show_i64 helper', () => {
      const wat = compileToWat('fn id(x: Int) -> Int { x }\nfn main() { println(show(id(1))) }');
      expect(wat).toContain('(func $show_i64');
      expect(wat).toContain('(result i32)');
    });

    it('chains show into println', () => {
      const wat = compileToWat('fn id(x: Int) -> Int { x }\nfn main() { println(show(id(42))) }');
      expect(wat).toContain('call $show_i64');
      expect(wat).toContain('call $println');
    });
  });

  describe('end-to-end via wasmtime', () => {
    it('hello world', () => {
      const wat = compileToWat('fn main() { println("Hello from JAPL!") }');
      const output = runWat(wat);
      if (output === null) return; // tools not available
      expect(output).toBe('Hello from JAPL!\n');
    });

    it('arithmetic: add(1, 2) = 3', () => {
      const wat = compileToWat(`fn add(x: Int, y: Int) -> Int { x + y }
fn main() { println(show(add(1, 2))) }`);
      const output = runWat(wat);
      if (output === null) return;
      expect(output).toBe('3\n');
    });

    it('fibonacci: fib(10) = 55', () => {
      const wat = compileToWat(`fn fib(n: Int) -> Int {
  if n <= 1 { n }
  else { fib(n - 1) + fib(n - 2) }
}
fn main() { println(show(fib(10))) }`);
      const output = runWat(wat);
      if (output === null) return;
      expect(output).toBe('55\n');
    });

    it('if/else with abs', () => {
      const wat = compileToWat(`fn abs(x: Int) -> Int {
  if x < 0 { 0 - x } else { x }
}
fn main() {
  println(show(abs(0 - 42)))
  println(show(abs(7)))
}`);
      const output = runWat(wat);
      if (output === null) return;
      expect(output).toBe('42\n7\n');
    });

    it('multiplication', () => {
      const wat = compileToWat(`fn mul(a: Int, b: Int) -> Int { a * b }
fn main() { println(show(mul(6, 7))) }`);
      const output = runWat(wat);
      if (output === null) return;
      expect(output).toBe('42\n');
    });

    it('nested function calls', () => {
      const wat = compileToWat(`fn double(x: Int) -> Int { x + x }
fn quadruple(x: Int) -> Int { double(double(x)) }
fn main() { println(show(quadruple(3))) }`);
      const output = runWat(wat);
      if (output === null) return;
      expect(output).toBe('12\n');
    });
  });
});

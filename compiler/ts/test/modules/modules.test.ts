import { describe, it, expect } from 'vitest';
import { MultiFileCompiler } from '../../src/modules/compiler.js';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as os from 'node:os';
import { execFileSync } from 'node:child_process';

const MODULES_DIR = path.resolve(__dirname, '.');

function compilePath(filename: string) {
  const compiler = new MultiFileCompiler();
  return compiler.compile(path.join(MODULES_DIR, filename));
}

function runGenerated(result: ReturnType<typeof compilePath>): string {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-test-modules-'));
  try {
    // Write all generated files
    for (const file of result.files) {
      const outPath = path.join(tmpDir, file.moduleName + '.ts');
      fs.writeFileSync(outPath, file.code);
    }

    // Find the entry file
    const entry = result.files.find(f => f.isEntry)!;
    const entryPath = path.join(tmpDir, entry.moduleName + '.ts');

    // Execute with tsx
    const output = execFileSync('npx', ['tsx', entryPath], {
      encoding: 'utf-8',
      cwd: tmpDir,
    });
    return output.trimEnd();
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

describe('Module System', () => {

  // ─── Basic import with selective imports ───
  describe('basic imports', () => {
    it('compiles main.japl with Math.japl dependency', () => {
      const result = compilePath('main.japl');
      expect(result.errors).toEqual([]);
      expect(result.files.length).toBe(2);
    });

    it('generates two .ts files (main + Math)', () => {
      const result = compilePath('main.japl');
      const names = result.files.map(f => f.moduleName).sort();
      expect(names).toEqual(['Math', 'main']);
    });

    it('Math.ts has export function add', () => {
      const result = compilePath('main.japl');
      const mathFile = result.files.find(f => f.moduleName === 'Math')!;
      expect(mathFile.code).toContain('export function add');
    });

    it('Math.ts has export function sub', () => {
      const result = compilePath('main.japl');
      const mathFile = result.files.find(f => f.moduleName === 'Math')!;
      expect(mathFile.code).toContain('export function sub');
    });

    it('Math.ts does NOT export internal()', () => {
      const result = compilePath('main.japl');
      const mathFile = result.files.find(f => f.moduleName === 'Math')!;
      expect(mathFile.code).not.toContain('export function internal');
      expect(mathFile.code).toContain('function internal');
    });

    it('main.ts has import { add, sub } from Math', () => {
      const result = compilePath('main.japl');
      const mainFile = result.files.find(f => f.moduleName === 'main')!;
      expect(mainFile.code).toContain('import { add, sub } from "./Math.js"');
    });

    it('runs and produces correct output: 30 and 42', () => {
      const result = compilePath('main.japl');
      const output = runGenerated(result);
      expect(output).toBe('30\n42');
    });
  });

  // ─── Private import rejection ───
  describe('private import rejection', () => {
    it('fails when importing a private function', () => {
      const result = compilePath('private.japl');
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('internal');
      expect(result.errors[0]).toContain('not public');
    });
  });

  // ─── Transitive dependencies (chain) ───
  describe('transitive dependencies', () => {
    it('compiles chain.japl with transitive A -> B dependency', () => {
      const result = compilePath('chain.japl');
      expect(result.errors).toEqual([]);
      // chain imports B, B imports A: should have 3 files
      expect(result.files.length).toBe(3);
    });

    it('generates A.ts, B.ts, and chain.ts', () => {
      const result = compilePath('chain.japl');
      const names = result.files.map(f => f.moduleName).sort();
      expect(names).toEqual(['A', 'B', 'chain']);
    });

    it('B.ts imports from A.js', () => {
      const result = compilePath('chain.japl');
      const bFile = result.files.find(f => f.moduleName === 'B')!;
      expect(bFile.code).toContain('import { a } from "./A.js"');
    });

    it('chain.ts imports from B.js', () => {
      const result = compilePath('chain.japl');
      const chainFile = result.files.find(f => f.moduleName === 'chain')!;
      expect(chainFile.code).toContain('import { b } from "./B.js"');
    });

    it('runs and produces correct output: 2', () => {
      const result = compilePath('chain.japl');
      const output = runGenerated(result);
      expect(output).toBe('2');
    });
  });

  // ─── Circular dependency detection ───
  describe('circular dependency detection', () => {
    it('detects circular dependency and errors', () => {
      const result = compilePath('circular.japl');
      expect(result.errors.length).toBeGreaterThan(0);
      expect(result.errors[0]).toContain('Circular dependency');
    });
  });

  // ─── Non-existent module ───
  describe('missing module', () => {
    it('errors when importing a non-existent module', () => {
      // Create a temp file that imports a non-existent module
      const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-test-missing-'));
      const tmpFile = path.join(tmpDir, 'bad.japl');
      fs.writeFileSync(tmpFile, 'import Nonexistent.{foo}\n\nfn main() { foo() }\n');
      try {
        const compiler = new MultiFileCompiler();
        const result = compiler.compile(tmpFile);
        expect(result.errors.length).toBeGreaterThan(0);
        expect(result.errors[0]).toContain('Cannot find module');
      } finally {
        fs.rmSync(tmpDir, { recursive: true, force: true });
      }
    });
  });

  // ─── Non-existent symbol in module ───
  describe('non-existent symbol', () => {
    it('errors when importing a symbol that does not exist', () => {
      const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-test-nosym-'));
      const modFile = path.join(tmpDir, 'Mod.japl');
      const mainFile = path.join(tmpDir, 'entry.japl');
      fs.writeFileSync(modFile, 'pub fn real() -> Int { 1 }\n');
      fs.writeFileSync(mainFile, 'import Mod.{nonexistent}\n\nfn main() { nonexistent() }\n');
      try {
        const compiler = new MultiFileCompiler();
        const result = compiler.compile(mainFile);
        expect(result.errors.length).toBeGreaterThan(0);
        expect(result.errors[0]).toContain('nonexistent');
        expect(result.errors[0]).toContain('does not exist');
      } finally {
        fs.rmSync(tmpDir, { recursive: true, force: true });
      }
    });
  });
});

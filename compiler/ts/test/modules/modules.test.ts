import { describe, it, expect } from 'vitest';
import { MultiFileCompiler } from '../../src/modules/compiler.js';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as os from 'node:os';

const MODULES_DIR = path.resolve(__dirname, '.');

function compilePath(filename: string) {
  const compiler = new MultiFileCompiler();
  return compiler.compile(path.join(MODULES_DIR, filename));
}

describe('Module System', () => {

  // ─── Basic import with selective imports ───
  describe('basic imports', () => {
    it('compiles main.japl with Math.japl dependency', () => {
      const result = compilePath('main.japl');
      expect(result.errors).toEqual([]);
      expect(result.files.length).toBe(2);
    });

    it('generates two output files (main + Math)', () => {
      const result = compilePath('main.japl');
      const names = result.files.map(f => f.moduleName).sort();
      expect(names).toEqual(['Math', 'main']);
    });

    it('each file produces WAT output', () => {
      const result = compilePath('main.japl');
      for (const file of result.files) {
        expect(file.code).toContain('(module');
      }
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

    it('generates A, B, and chain output files', () => {
      const result = compilePath('chain.japl');
      const names = result.files.map(f => f.moduleName).sort();
      expect(names).toEqual(['A', 'B', 'chain']);
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

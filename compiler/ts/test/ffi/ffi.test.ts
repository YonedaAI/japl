import { describe, it, expect } from 'vitest';
import { Lexer } from '../../src/lexer/lexer.js';
import { Parser } from '../../src/parser/parser.js';
import { lowerModule } from '../../src/ir/lower.js';
import { TsEmitter } from '../../src/codegen/emit.js';
import { execFileSync } from 'node:child_process';
import * as path from 'node:path';
import * as fs from 'node:fs';
import * as os from 'node:os';

// ─── Helper: full pipeline from JAPL source to TypeScript output ───

function compile(source: string): string {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(", ")}`);
  }
  const ir = lowerModule(ast);
  const emitter = new TsEmitter();
  return emitter.emit(ir);
}

const compilerRoot = path.resolve(__dirname, '../..');

/** Compile JAPL to TS, write to temp file, run with tsx, return stdout */
function compileAndRun(source: string): string {
  const ts = compile(source);
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-ffi-test-'));
  const tmpFile = path.join(tmpDir, 'test.ts');
  fs.writeFileSync(tmpFile, ts);
  try {
    const output = execFileSync(
      'npx', ['tsx', tmpFile],
      { cwd: compilerRoot, encoding: 'utf-8' }
    );
    return output.trim();
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

describe("FFI: end-to-end", () => {

  // ─── 1. readFileSync via foreign "node:fs" ───
  it("compiles and runs readFileSync from node:fs", () => {
    const source = `
foreign "node:fs" fn readFileSync(path: String, encoding: String) -> String

fn main() {
  let content = readFileSync("test/ffi/hello.txt", "utf-8")
  println(content)
}
`;
    const ts = compile(source);

    // Verify import statement is present
    expect(ts).toContain(`import { readFileSync } from 'node:fs';`);

    // Verify the function is used, not redefined
    expect(ts).not.toContain("function readFileSync");

    // Actually run it
    const output = compileAndRun(source);
    expect(output).toBe("hello from FFI");
  });

  // ─── 2. Multiple modules, grouped imports ───
  it("groups imports by module", () => {
    const source = `
foreign "node:fs" fn existsSync(path: String) -> Bool
foreign "node:os" fn hostname() -> String

fn main() {
  let exists = existsSync("test/ffi/hello.txt")
  println(show(exists))
  let host = hostname()
  println(host)
}
`;
    const ts = compile(source);

    // Each module gets one import statement
    expect(ts).toContain(`import { existsSync } from 'node:fs';`);
    expect(ts).toContain(`import { hostname } from 'node:os';`);

    // Actually run it
    const output = compileAndRun(source);
    const lines = output.split('\n');
    expect(lines[0]).toBe("true");
    // hostname() returns something non-empty
    expect(lines[1].length).toBeGreaterThan(0);
  });

  // ─── 3. Multiple fns from same module → one import ───
  it("merges multiple imports from the same module", () => {
    const source = `
foreign "node:fs" fn readFileSync(path: String, encoding: String) -> String
foreign "node:fs" fn existsSync(path: String) -> Bool

fn main() {
  let content = readFileSync("test/ffi/hello.txt", "utf-8")
  println(content)
  let exists = existsSync("test/ffi/hello.txt")
  println(show(exists))
}
`;
    const ts = compile(source);

    // Should have ONE import from node:fs with both functions
    expect(ts).toContain(`import { readFileSync, existsSync } from 'node:fs';`);

    // Should NOT have two separate import lines from node:fs
    const importLines = ts.split('\n').filter(l => l.includes("from 'node:fs'"));
    expect(importLines.length).toBe(1);

    // Run it
    const output = compileAndRun(source);
    const lines = output.split('\n');
    expect(lines[0]).toBe("hello from FFI");
    expect(lines[1]).toBe("true");
  });

  // ─── 4. Builtins still work after FFI changes ───
  it("builtins still work", () => {
    const source = `
fn main() {
  println("builtins still work")
  println(show(42))
  println(show(3.14))
}
`;
    const ts = compile(source);

    // Should have builtin helpers
    expect(ts).toContain("const println");
    expect(ts).toContain("const show");

    // Run it
    const output = compileAndRun(source);
    const lines = output.split('\n');
    expect(lines[0]).toBe("builtins still work");
    expect(lines[1]).toBe("42");
    expect(lines[2]).toBe("3.14");
  });

  // ─── 5. Foreign without module → builtin (no import) ───
  it("foreign without module does not emit import", () => {
    const source = `
foreign fn customBuiltin(x: String) -> String

fn main() {
  println("hello")
}
`;
    const ts = compile(source);

    // Should NOT have any import { customBuiltin } line
    expect(ts).not.toContain("import { customBuiltin");
    // Should still compile (customBuiltin is treated as builtin)
    expect(ts).toContain("function main()");
  });

  // ─── 6. The full io.japl goal (readFileSync + join) ───
  it("runs the full io.japl example: readFileSync + path.join", () => {
    const source = `
foreign "node:fs" fn readFileSync(path: String, encoding: String) -> String
foreign "node:path" fn join(parts: String, more: String) -> String

fn main() {
  let content = readFileSync("test/ffi/hello.txt", "utf-8")
  println(content)
  let p = join("src", "main.japl")
  println(p)
}
`;
    const ts = compile(source);

    expect(ts).toContain(`import { readFileSync } from 'node:fs';`);
    expect(ts).toContain(`import { join } from 'node:path';`);

    const output = compileAndRun(source);
    const lines = output.split('\n');
    expect(lines[0]).toBe("hello from FFI");
    expect(lines[1]).toBe("src/main.japl");
  });

  // ─── 7. npm-style package module ───
  it("handles npm-style package imports", () => {
    const source = `
foreign "@some/pkg" fn doSomething() -> String

fn main() {
  println("ok")
}
`;
    const ts = compile(source);
    // Should emit the import even for scoped packages
    expect(ts).toContain(`import { doSomething } from '@some/pkg';`);
  });

  // ─── 8. jsName alias (as syntax) ───
  it("handles foreign fn with as alias", () => {
    const source = `
foreign "node:fs" fn read_file as "readFileSync"(path: String, encoding: String) -> String

fn main() {
  let content = read_file("test/ffi/hello.txt", "utf-8")
  println(content)
}
`;
    const ts = compile(source);

    // Should emit: import { readFileSync as read_file } from 'node:fs';
    expect(ts).toContain(`import { readFileSync as read_file } from 'node:fs';`);

    // Run it - read_file should be available as the alias
    const output = compileAndRun(source);
    expect(output).toBe("hello from FFI");
  });
});

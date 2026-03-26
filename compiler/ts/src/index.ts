#!/usr/bin/env node

import { buildFile, buildToString } from './cli/build.js';
import { parseConfig } from './cli/config.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import { execFileSync } from 'node:child_process';
import { Lexer } from './lexer/index.js';
import { Parser } from './parser/index.js';
import { TypeChecker } from './checker/index.js';

const VERSION = '0.1.0';

const args = process.argv.slice(2);
const command = args[0];

function printHelp(): void {
  console.log(`JAPL ${VERSION} — Just Another Programming Language

Usage:
  japl build <file.japl> [--target ts|c] [--out <dir>]
  japl run <file.japl>              Build to TS and execute with node
  japl check <file.japl>            Type check only
  japl fmt <file.japl>              Format (stub)
  japl test [dir]                   Find and run test blocks
  japl new <name>                   Scaffold a project
  japl version                      Print version
  japl help                         Show this help`);
}

function parseCliArgs(args: string[]): { flags: Record<string, string>; positional: string[] } {
  const flags: Record<string, string> = {};
  const positional: string[] = [];
  let i = 0;
  while (i < args.length) {
    if (args[i].startsWith('--')) {
      const key = args[i].slice(2);
      if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
        flags[key] = args[i + 1];
        i += 2;
      } else {
        flags[key] = 'true';
        i++;
      }
    } else {
      positional.push(args[i]);
      i++;
    }
  }
  return { flags, positional };
}

function cmdBuild(args: string[]): void {
  const { flags, positional } = parseCliArgs(args);
  const inputFile = positional[0];
  if (!inputFile) {
    console.error('Error: missing input file');
    console.error('Usage: japl build <file.japl> [--target ts|c] [--out <dir>]');
    process.exit(1);
  }

  const target = (flags['target'] ?? 'ts') as 'ts' | 'c';
  if (target !== 'ts' && target !== 'c') {
    console.error(`Error: target "${target}" not yet supported`);
    process.exit(1);
  }

  const ext = target === 'c' ? '.c' : '.ts';
  let outputPath: string;
  if (flags['out']) {
    const outDir = flags['out'];
    const baseName = path.basename(inputFile, '.japl') + ext;
    outputPath = path.join(outDir, baseName);
  } else {
    outputPath = inputFile.replace(/\.japl$/, ext);
  }

  buildFile(inputFile, outputPath, target);
}

function cmdRun(args: string[]): void {
  const inputFile = args[0];
  if (!inputFile) {
    console.error('Error: missing input file');
    console.error('Usage: japl run <file.japl>');
    process.exit(1);
  }

  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-'));
  const baseName = path.basename(inputFile, '.japl') + '.ts';
  const tmpFile = path.join(tmpDir, baseName);

  try {
    buildFile(inputFile, tmpFile);

    // Try tsx first, then ts-node, then tsc+node
    try {
      execFileSync('npx', ['tsx', tmpFile], { stdio: 'inherit' });
    } catch {
      try {
        execFileSync('npx', ['ts-node', '--esm', tmpFile], { stdio: 'inherit' });
      } catch {
        // Fallback: compile with tsc, then run with node
        const jsFile = tmpFile.replace(/\.ts$/, '.js');
        execFileSync('npx', [
          'tsc', '--outDir', tmpDir, '--module', 'Node16',
          '--moduleResolution', 'Node16', '--target', 'ES2022',
          '--esModuleInterop', 'true', tmpFile
        ], { stdio: 'inherit' });
        execFileSync('node', [jsFile], { stdio: 'inherit' });
      }
    }
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

function cmdCheck(args: string[]): void {
  const inputFile = args[0];
  if (!inputFile) {
    console.error('Error: missing input file');
    console.error('Usage: japl check <file.japl>');
    process.exit(1);
  }

  const source = fs.readFileSync(inputFile, 'utf-8');
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const parseErrors = parser.getErrors();

  if (parseErrors.length > 0) {
    console.error(`Parse errors in ${inputFile}:`);
    for (const err of parseErrors) {
      console.error(`  ${err.message}`);
    }
    process.exit(1);
  }

  const checker = new TypeChecker();
  const result = checker.checkModule(ast);

  if (result.errors.length > 0) {
    console.error(`Type errors in ${inputFile}:`);
    for (const err of result.errors) {
      console.error(`  ${err.message}`);
    }
    process.exit(1);
  }

  console.log(`${inputFile}: OK`);
}

function cmdFmt(args: string[]): void {
  const inputFile = args[0];
  if (!inputFile) {
    console.error('Error: missing input file');
    console.error('Usage: japl fmt <file.japl>');
    process.exit(1);
  }
  console.log(`fmt: not yet implemented (${inputFile})`);
}

function cmdTest(args: string[]): void {
  const dir = args[0] ?? '.';
  const files = findJaplFiles(dir);
  let totalTests = 0;
  let passed = 0;
  let failed = 0;

  for (const file of files) {
    const source = fs.readFileSync(file, 'utf-8');

    // Quick check for test declarations
    if (!source.includes('test ')) continue;

    try {
      const tsCode = buildToString(source);
      if (!tsCode.includes('function test_')) continue;

      const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-test-'));
      const tmpFile = path.join(tmpDir, 'test.ts');

      // Wrap test functions with runner
      const testRunner = tsCode + '\n\n' + extractTestRunner(tsCode);
      fs.writeFileSync(tmpFile, testRunner);

      try {
        execFileSync('npx', ['tsx', tmpFile], { stdio: 'pipe', encoding: 'utf-8' });
        const testCount = (tsCode.match(/function test_/g) || []).length;
        totalTests += testCount;
        passed += testCount;
        console.log(`  PASS ${file} (${testCount} tests)`);
      } catch (e: unknown) {
        const testCount = (tsCode.match(/function test_/g) || []).length;
        totalTests += testCount;
        failed += testCount;
        const msg = e instanceof Error ? e.message : String(e);
        console.log(`  FAIL ${file}: ${msg}`);
      } finally {
        fs.rmSync(tmpDir, { recursive: true, force: true });
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error(`  ERROR ${file}: ${msg}`);
    }
  }

  console.log(`\nTests: ${passed} passed, ${failed} failed, ${totalTests} total`);
  if (failed > 0) process.exit(1);
}

function extractTestRunner(tsCode: string): string {
  const testFns = tsCode.match(/function (test_\w+)/g) || [];
  const calls = testFns.map(m => {
    const name = m.replace('function ', '');
    return `try { ${name}(); console.log("  pass: ${name}"); } catch(e) { console.error("  fail: ${name}:", e.message); process.exitCode = 1; }`;
  });
  return calls.join('\n');
}

function findJaplFiles(dir: string): string[] {
  const results: string[] = [];
  if (!fs.existsSync(dir)) return results;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules') {
      results.push(...findJaplFiles(fullPath));
    } else if (entry.isFile() && entry.name.endsWith('.japl')) {
      results.push(fullPath);
    }
  }
  return results;
}

function cmdNew(args: string[]): void {
  const name = args[0];
  if (!name) {
    console.error('Error: missing project name');
    console.error('Usage: japl new <name>');
    process.exit(1);
  }

  const projectDir = path.resolve(name);
  if (fs.existsSync(projectDir)) {
    console.error(`Error: directory "${name}" already exists`);
    process.exit(1);
  }

  fs.mkdirSync(path.join(projectDir, 'src'), { recursive: true });

  fs.writeFileSync(path.join(projectDir, 'japl.toml'), `[package]
name = "${name}"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]

[dev-dependencies]
`);

  fs.writeFileSync(path.join(projectDir, 'src', 'main.japl'), `fn main() {
  println("Hello from ${name}!")
}
`);

  fs.writeFileSync(path.join(projectDir, '.gitignore'), `dist/
node_modules/
*.ts
!src/**/*.japl
`);

  console.log(`Created project "${name}" at ${projectDir}`);
  console.log(`  ${name}/japl.toml`);
  console.log(`  ${name}/src/main.japl`);
  console.log(`  ${name}/.gitignore`);
}

// ─── Dispatch ───

switch (command) {
  case 'build':
    cmdBuild(args.slice(1));
    break;
  case 'run':
    cmdRun(args.slice(1));
    break;
  case 'check':
    cmdCheck(args.slice(1));
    break;
  case 'fmt':
    cmdFmt(args.slice(1));
    break;
  case 'test':
    cmdTest(args.slice(1));
    break;
  case 'new':
    cmdNew(args.slice(1));
    break;
  case 'version':
  case '--version':
  case '-v':
    console.log(`japl ${VERSION}`);
    break;
  case 'help':
  case '--help':
  case '-h':
  case undefined:
    printHelp();
    break;
  default:
    console.error(`Unknown command: ${command}`);
    printHelp();
    process.exit(1);
}

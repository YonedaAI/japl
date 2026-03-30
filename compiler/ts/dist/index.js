#!/usr/bin/env node
import { buildToWat, buildToWasm, runWasm, checkTools, findJaplRuntime, programUsesProcesses } from './cli/build.js';
import { execFileSync } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import { Lexer } from './lexer/index.js';
import { Parser } from './parser/index.js';
import { TypeChecker } from './checker/index.js';
const VERSION = '0.2.0';
const args = process.argv.slice(2);
const command = args[0];
function printHelp() {
    console.log(`JAPL ${VERSION} — Just Another Programming Language

Usage:
  japl build <file.japl>                Compile to WASM
  japl run <file.japl>                  Compile and execute
  japl run --node <name> <file.japl>    Run in distributed mode
  japl check <file.japl>                Type check only
  japl fmt <file.japl>                  Format (stub)
  japl new <name>                       Scaffold a project
  japl version                          Print version
  japl help                             Show this help

Run options:
  --node <name>          Node name for distributed mode
  --listen <:port>       Listen for connections
  --connect <host:port>  Connect to peer node

Build options:
  --emit-wat             Output WAT text (debug)
  --out <dir>            Output directory (default: build/)

Requirements:
  wat2wasm               brew install wabt
  wasmtime               brew install wasmtime (optional, for simple programs)
  japl-runtime           cd japl-runtime && cargo build (for processes)`);
}
function parseCliArgs(args) {
    const flags = {};
    const positional = [];
    let i = 0;
    while (i < args.length) {
        if (args[i].startsWith('--')) {
            const key = args[i].slice(2);
            if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
                flags[key] = args[i + 1];
                i += 2;
            }
            else {
                flags[key] = 'true';
                i++;
            }
        }
        else {
            positional.push(args[i]);
            i++;
        }
    }
    return { flags, positional };
}
function cmdBuild(args) {
    const { flags, positional } = parseCliArgs(args);
    const inputFile = positional[0];
    if (!inputFile) {
        console.error('Usage: japl build <file.japl> [--emit-wat] [--out <dir>]');
        process.exit(1);
    }
    const emitWat = 'emit-wat' in flags;
    const outDir = flags['out'] ?? 'build';
    // Ensure output directory exists
    if (!fs.existsSync(outDir)) {
        fs.mkdirSync(outDir, { recursive: true });
    }
    if (emitWat) {
        // Just emit WAT text
        const source = fs.readFileSync(inputFile, 'utf-8');
        const wat = buildToWat(source);
        const watPath = path.join(outDir, path.basename(inputFile, '.japl') + '.wat');
        fs.writeFileSync(watPath, wat);
        console.log(`Compiled ${inputFile} → ${watPath}`);
    }
    else {
        // Full pipeline: .japl → .wat → .wasm
        checkTools(['wat2wasm']);
        const wasmPath = buildToWasm(inputFile, outDir);
        console.log(`Compiled ${inputFile} → ${wasmPath}`);
    }
}
function cmdCheck(args) {
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
function cmdFmt(args) {
    const inputFile = args[0];
    if (!inputFile) {
        console.error('Error: missing input file');
        console.error('Usage: japl fmt <file.japl>');
        process.exit(1);
    }
    console.log(`fmt: not yet implemented (${inputFile})`);
}
function cmdNew(args) {
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
    fs.writeFileSync(path.join(projectDir, '.gitignore'), `build/
.japl-build/
*.wasm
*.wat
node_modules/
`);
    console.log(`Created project "${name}" at ${projectDir}`);
    console.log(`  ${name}/japl.toml`);
    console.log(`  ${name}/src/main.japl`);
    console.log(`  ${name}/.gitignore`);
}
function cmdRun(args) {
    const { flags, positional } = parseCliArgs(args);
    const inputFile = positional[0];
    if (!inputFile) {
        console.error('Usage: japl run <file.japl> [--node <name>] [--listen <:port>] [--connect <host:port>]');
        process.exit(1);
    }
    checkTools(['wat2wasm']);
    // Build to temp directory
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-'));
    try {
        const wasmPath = buildToWasm(inputFile, tmpDir);
        // Determine if we need the full japl-runtime (process support)
        const needsRuntime = flags['node'] !== undefined ||
            flags['listen'] !== undefined ||
            flags['connect'] !== undefined ||
            programUsesProcesses(wasmPath);
        if (needsRuntime) {
            // Use japl-runtime for process support
            const runtimeBin = findJaplRuntime();
            const runtimeArgs = ['run', wasmPath];
            if (flags['node'] !== undefined)
                runtimeArgs.push('--node', flags['node']);
            if (flags['listen'] !== undefined)
                runtimeArgs.push('--listen', flags['listen']);
            if (flags['connect'] !== undefined)
                runtimeArgs.push('--connect', flags['connect']);
            execFileSync(runtimeBin, runtimeArgs, { stdio: 'inherit' });
        }
        else {
            // Use wasmtime directly (faster for simple programs)
            runWasm(wasmPath);
        }
    }
    catch (err) {
        console.error(err.message);
        process.exit(1);
    }
    finally {
        fs.rmSync(tmpDir, { recursive: true, force: true });
    }
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
//# sourceMappingURL=index.js.map
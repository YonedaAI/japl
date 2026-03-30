import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { TsEmitter } from '../codegen/emit.js';
import { CEmitter } from '../codegen/emit_c.js';
import { MultiFileCompiler } from '../modules/compiler.js';
import { TypeChecker } from '../checker/infer.js';
import { LinearityChecker } from '../checker/linearity.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { execFileSync } from 'node:child_process';
export function buildToString(source, targetOrOpts = 'ts') {
    const opts = typeof targetOrOpts === 'string'
        ? { target: targetOrOpts }
        : targetOrOpts;
    const target = opts.target ?? 'ts';
    const strict = opts.strict ?? false;
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    const errors = parser.getErrors();
    if (errors.length > 0) {
        throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
    }
    // Type check + effect/linearity enforcement in strict mode
    if (strict) {
        const checker = new TypeChecker();
        const typed = checker.checkModule(ast);
        // Effect violations
        const effectErrors = typed.errors.filter(e => e.message.includes('effect') || e.message.includes('pure') || e.message.includes('Effect') || e.message.includes('Pure'));
        if (effectErrors.length > 0) {
            throw new Error(`Effect violations:\n${effectErrors.map(e => `  ${e.toString()}`).join('\n')}`);
        }
        // Linearity violations (from dedicated linearity checker)
        const linearityChecker = new LinearityChecker();
        const linearityErrors = linearityChecker.checkModule(ast);
        if (linearityErrors.length > 0) {
            throw new Error(`Linearity violations:\n${linearityErrors.map(e => `  ${e.toString()}`).join('\n')}`);
        }
        // Exhaustiveness violations
        const exhaustErrors = typed.errors.filter(e => e.message.includes('Non-exhaustive') || e.message.includes('exhaustive'));
        if (exhaustErrors.length > 0) {
            throw new Error(`Exhaustiveness violations:\n${exhaustErrors.map(e => `  ${e.toString()}`).join('\n')}`);
        }
    }
    const ir = lowerModule(ast);
    if (target === 'c') {
        const emitter = new CEmitter();
        return emitter.emit(ir);
    }
    const emitter = new TsEmitter();
    return emitter.emit(ir);
}
/**
 * Multi-file build: resolves imports, compiles all dependencies.
 * Returns the CompileResult with all generated files.
 */
export function buildMultiFile(inputPath, extraSearchPaths = []) {
    // Auto-discover stdlib path: look for ../../stdlib relative to compiler or input
    const thisFile = new URL(import.meta.url).pathname;
    const thisDir = path.dirname(thisFile);
    const stdlibCandidates = [
        path.resolve(path.dirname(inputPath), '../stdlib'),
        path.resolve(path.dirname(inputPath), '../../stdlib'),
        path.resolve(thisDir, '../../../stdlib'),
        path.resolve(thisDir, '../../../../stdlib'),
    ];
    const searchPaths = [...extraSearchPaths];
    for (const candidate of stdlibCandidates) {
        if (fs.existsSync(candidate)) {
            searchPaths.push(candidate);
            break;
        }
    }
    const compiler = new MultiFileCompiler(searchPaths);
    return compiler.compile(inputPath);
}
/**
 * Multi-file build that writes all output files to a directory.
 * Returns the path to the entry file's generated .ts file.
 */
export function buildMultiFileTo(inputPath, outDir, extraSearchPaths = []) {
    const result = buildMultiFile(inputPath, extraSearchPaths);
    if (result.errors.length > 0) {
        throw new Error(result.errors.join('\n'));
    }
    if (!fs.existsSync(outDir)) {
        fs.mkdirSync(outDir, { recursive: true });
    }
    let entryOutPath = '';
    for (const file of result.files) {
        const baseName = file.moduleName + '.ts';
        const outPath = path.join(outDir, baseName);
        fs.writeFileSync(outPath, file.code);
        if (file.isEntry) {
            entryOutPath = outPath;
        }
    }
    return entryOutPath;
}
/**
 * Check whether a file has any import declarations (needs multi-file build).
 */
function fileHasImports(inputPath) {
    const source = fs.readFileSync(inputPath, 'utf-8');
    // Quick heuristic: check for import keyword at start of line
    return /^\s*import\s+[A-Z]/m.test(source);
}
export function buildFile(inputPath, outputPath, target = 'ts') {
    // For TypeScript target, check if the file has imports and use multi-file build
    if (target === 'ts' && fileHasImports(inputPath)) {
        const outDir = outputPath ? path.dirname(outputPath) : path.dirname(inputPath);
        const result = buildMultiFile(inputPath);
        if (result.errors.length > 0) {
            throw new Error(result.errors.join('\n'));
        }
        for (const file of result.files) {
            const baseName = file.moduleName + '.ts';
            const outPath = path.join(outDir, baseName);
            const dir = path.dirname(outPath);
            if (!fs.existsSync(dir)) {
                fs.mkdirSync(dir, { recursive: true });
            }
            fs.writeFileSync(outPath, file.code);
            console.log(`Compiled ${file.sourcePath} → ${outPath}`);
        }
        return;
    }
    const source = fs.readFileSync(inputPath, 'utf-8');
    const code = buildToString(source, target);
    const ext = target === 'c' ? '.c' : '.ts';
    const outPath = outputPath ?? inputPath.replace('.japl', ext);
    const dir = path.dirname(outPath);
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
    }
    fs.writeFileSync(outPath, code);
    console.log(`Compiled ${inputPath} → ${outPath}`);
    // Optionally compile C to binary
    if (target === 'c') {
        const binPath = outPath.replace('.c', '');
        const runtimeDir = path.resolve(path.dirname(outPath), '../../c');
        const includeDir = path.join(runtimeDir, 'include');
        const libPath = path.join(runtimeDir, 'libjapl_runtime.a');
        if (fs.existsSync(libPath)) {
            try {
                execFileSync('gcc', [
                    '-std=c11', '-Wall',
                    `-I${includeDir}`,
                    '-o', binPath,
                    outPath,
                    libPath,
                    '-lpthread',
                ], { stdio: 'inherit' });
                console.log(`Linked ${outPath} → ${binPath}`);
            }
            catch {
                console.error('Failed to compile C output with gcc');
            }
        }
    }
}
//# sourceMappingURL=build.js.map
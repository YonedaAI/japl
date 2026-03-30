// ─── Multi-File Compiler ───
// Resolves imports, builds dependency graph, compiles in order.
// Target: WASM only (WAT text format as intermediate)
import * as fs from 'node:fs';
import * as path from 'node:path';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { WatEmitter } from '../codegen/emit_wat.js';
import { ModuleResolver, ModuleError } from './resolver.js';
export class MultiFileCompiler {
    resolver;
    modules = new Map();
    errors = [];
    constructor(searchPaths = []) {
        this.resolver = new ModuleResolver(searchPaths);
    }
    compile(entryFile) {
        this.modules.clear();
        this.errors = [];
        const absEntry = path.resolve(entryFile);
        // 1. Parse entry file and discover all dependencies
        try {
            this.discoverModules(absEntry, true);
        }
        catch (e) {
            if (e instanceof ModuleError) {
                return { files: [], errors: [e.message] };
            }
            throw e;
        }
        if (this.errors.length > 0) {
            return { files: [], errors: this.errors };
        }
        // 2. Topological sort (detect circular deps)
        let sorted;
        try {
            sorted = this.topologicalSort();
        }
        catch (e) {
            if (e instanceof ModuleError) {
                return { files: [], errors: [e.message] };
            }
            throw e;
        }
        // 3. Compile each module in dependency order
        const files = [];
        for (const mod of sorted) {
            const ir = lowerModule(mod.ast);
            const emitter = new WatEmitter();
            const code = emitter.emitModule(ir, {
                isEntry: mod.isEntry,
            });
            files.push({
                sourcePath: mod.filePath,
                moduleName: mod.moduleName,
                code,
                isEntry: mod.isEntry,
            });
        }
        return { files, errors: [] };
    }
    discoverModules(filePath, isEntry) {
        if (this.modules.has(filePath))
            return;
        // Parse the file
        const source = fs.readFileSync(filePath, 'utf-8');
        const lexer = new Lexer(source);
        const tokens = lexer.tokenize();
        const parser = new Parser(tokens);
        const ast = parser.parse();
        const parseErrors = parser.getErrors();
        if (parseErrors.length > 0) {
            throw new ModuleError(`Parse errors in ${filePath}:\n` +
                parseErrors.map(e => `  ${e.message}`).join('\n'));
        }
        const moduleName = path.basename(filePath, '.japl');
        const imports = [];
        // Create placeholder to prevent infinite recursion on circular deps
        const node = { filePath, moduleName, ast, imports, isEntry };
        this.modules.set(filePath, node);
        // Find all import declarations
        for (const decl of ast.decls) {
            if (decl.kind === 'import') {
                // path is e.g., ["Math"] for `import Math.{add}`
                const importModuleName = decl.path[0];
                let resolved;
                try {
                    resolved = this.resolver.resolve(importModuleName, filePath);
                }
                catch (e) {
                    if (e instanceof ModuleError) {
                        throw e;
                    }
                    throw e;
                }
                // Validate that all requested items are public
                for (const item of decl.items) {
                    if (!resolved.exports.has(item)) {
                        if (resolved.allDecls.has(item)) {
                            throw new ModuleError(`Cannot import "${item}" from module "${importModuleName}": ` +
                                `"${item}" is not public. Add "pub" to export it.`);
                        }
                        else {
                            throw new ModuleError(`Cannot import "${item}" from module "${importModuleName}": ` +
                                `"${item}" does not exist in that module.`);
                        }
                    }
                }
                imports.push({
                    moduleName: importModuleName,
                    items: decl.items,
                    resolvedModule: resolved,
                });
                // Recursively discover the imported module's dependencies
                this.discoverModules(resolved.path, false);
            }
        }
    }
    topologicalSort() {
        const result = [];
        const visited = new Set();
        const visiting = new Set(); // for cycle detection
        const visit = (filePath, chain) => {
            if (visited.has(filePath))
                return;
            if (visiting.has(filePath)) {
                const cycle = [...chain, filePath].map(p => path.basename(p, '.japl'));
                throw new ModuleError(`Circular dependency detected: ${cycle.join(' -> ')}`);
            }
            visiting.add(filePath);
            const node = this.modules.get(filePath);
            for (const imp of node.imports) {
                visit(imp.resolvedModule.path, [...chain, filePath]);
            }
            visiting.delete(filePath);
            visited.add(filePath);
            result.push(node);
        };
        for (const [filePath] of this.modules) {
            visit(filePath, []);
        }
        return result;
    }
}
//# sourceMappingURL=compiler.js.map
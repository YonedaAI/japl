// ─── Module Resolver ───
// Given an import like `import Math.{add, multiply}` in file src/main.japl:
// 1. Convert module name to file path: Math -> Math.japl or math.japl
// 2. Check if file exists
// 3. Parse it
// 4. Return its AST and public exports
import * as fs from 'node:fs';
import * as path from 'node:path';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
export class ModuleResolver {
    cache = new Map();
    searchPaths;
    constructor(searchPaths) {
        this.searchPaths = searchPaths;
    }
    resolve(moduleName, fromFile) {
        // Check cache first
        const cacheKey = moduleName;
        const cached = this.cache.get(cacheKey);
        if (cached)
            return cached;
        // Build candidate file paths
        const fromDir = path.dirname(path.resolve(fromFile));
        const candidates = [];
        // Try same directory as importing file first
        candidates.push(path.join(fromDir, moduleName + '.japl'));
        candidates.push(path.join(fromDir, moduleName.toLowerCase() + '.japl'));
        // Then try search paths
        for (const searchPath of this.searchPaths) {
            candidates.push(path.join(searchPath, moduleName + '.japl'));
            candidates.push(path.join(searchPath, moduleName.toLowerCase() + '.japl'));
        }
        // Find first existing file
        let resolvedPath = null;
        for (const candidate of candidates) {
            if (fs.existsSync(candidate)) {
                resolvedPath = candidate;
                break;
            }
        }
        if (!resolvedPath) {
            throw new ModuleError(`Cannot find module "${moduleName}". Searched:\n` +
                candidates.map(c => `  - ${c}`).join('\n'));
        }
        // Parse the module file
        const source = fs.readFileSync(resolvedPath, 'utf-8');
        const lexer = new Lexer(source);
        const tokens = lexer.tokenize();
        const parser = new Parser(tokens);
        const ast = parser.parse();
        const parseErrors = parser.getErrors();
        if (parseErrors.length > 0) {
            throw new ModuleError(`Parse errors in module "${moduleName}" (${resolvedPath}):\n` +
                parseErrors.map(e => `  ${e.message}`).join('\n'));
        }
        // Collect exports (pub) and all declarations
        const exports = new Map();
        const allDecls = new Map();
        for (const decl of ast.decls) {
            if (decl.kind === 'fn') {
                allDecls.set(decl.name, decl);
                if (decl.pub) {
                    exports.set(decl.name, decl);
                }
            }
            else if (decl.kind === 'type') {
                allDecls.set(decl.name, decl);
                exports.set(decl.name, decl); // types are always public
                // Also export variant constructor names
                for (const variant of decl.variants) {
                    allDecls.set(variant.name, decl);
                    exports.set(variant.name, decl);
                }
            }
            else if (decl.kind === 'record_type') {
                allDecls.set(decl.name, decl);
                exports.set(decl.name, decl); // record types are always public
            }
        }
        const resolved = {
            name: moduleName,
            path: resolvedPath,
            ast,
            exports,
            allDecls,
        };
        this.cache.set(cacheKey, resolved);
        return resolved;
    }
}
export class ModuleError extends Error {
    constructor(message) {
        super(message);
        this.name = 'ModuleError';
    }
}
//# sourceMappingURL=resolver.js.map
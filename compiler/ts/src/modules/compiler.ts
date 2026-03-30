// ─── Multi-File Compiler ───
// Resolves imports, builds dependency graph, compiles in order.

import * as fs from 'node:fs';
import * as path from 'node:path';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { TsEmitter } from '../codegen/emit.js';
import * as AST from '../parser/ast.js';
import * as IR from '../ir/ir.js';
import { ModuleResolver, ModuleError, ResolvedModule } from './resolver.js';

export interface CompiledFile {
  /** Original .japl source path */
  sourcePath: string;
  /** Module name (e.g., "Math") */
  moduleName: string;
  /** Generated TypeScript code */
  code: string;
  /** Whether this is the entry file */
  isEntry: boolean;
}

export interface CompileResult {
  files: CompiledFile[];
  errors: string[];
}

interface ModuleNode {
  filePath: string;
  moduleName: string;
  ast: AST.Module;
  imports: ImportInfo[];
  isEntry: boolean;
}

interface ImportInfo {
  moduleName: string;
  items: string[];
  resolvedModule: ResolvedModule;
}

export class MultiFileCompiler {
  private resolver: ModuleResolver;
  private modules: Map<string, ModuleNode> = new Map();
  private errors: string[] = [];

  constructor(searchPaths: string[] = []) {
    this.resolver = new ModuleResolver(searchPaths);
  }

  compile(entryFile: string): CompileResult {
    this.modules.clear();
    this.errors = [];

    const absEntry = path.resolve(entryFile);

    // 1. Parse entry file and discover all dependencies
    try {
      this.discoverModules(absEntry, true);
    } catch (e) {
      if (e instanceof ModuleError) {
        return { files: [], errors: [e.message] };
      }
      throw e;
    }

    if (this.errors.length > 0) {
      return { files: [], errors: this.errors };
    }

    // 2. Topological sort (detect circular deps)
    let sorted: ModuleNode[];
    try {
      sorted = this.topologicalSort();
    } catch (e) {
      if (e instanceof ModuleError) {
        return { files: [], errors: [e.message] };
      }
      throw e;
    }

    // 3. Compile each module in dependency order
    const files: CompiledFile[] = [];
    for (const mod of sorted) {
      const ir = lowerModule(mod.ast);

      // For non-entry modules, mark all pub fns as exported
      // The IR already has exported: true from AST pub: true via lowerDecl

      // Determine import rewrite info: for each import in this module,
      // compute relative path from this file to the imported module's output
      const importRewrites = new Map<string, string>();
      for (const imp of mod.imports) {
        const importedNode = this.modules.get(imp.resolvedModule.path);
        if (importedNode) {
          // All output files are placed in the same flat directory,
          // so import paths are always ./ModuleName.js
          importRewrites.set(imp.moduleName, `./${importedNode.moduleName}.js`);
        }
      }

      const emitter = new TsEmitter();
      const code = emitter.emitModule(ir, {
        isEntry: mod.isEntry,
        importRewrites,
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

  private discoverModules(filePath: string, isEntry: boolean): void {
    if (this.modules.has(filePath)) return;

    // Parse the file
    const source = fs.readFileSync(filePath, 'utf-8');
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    const parseErrors = parser.getErrors();

    if (parseErrors.length > 0) {
      throw new ModuleError(
        `Parse errors in ${filePath}:\n` +
        parseErrors.map(e => `  ${e.message}`).join('\n')
      );
    }

    const moduleName = path.basename(filePath, '.japl');
    const imports: ImportInfo[] = [];

    // Create placeholder to prevent infinite recursion on circular deps
    const node: ModuleNode = { filePath, moduleName, ast, imports, isEntry };
    this.modules.set(filePath, node);

    // Find all import declarations
    for (const decl of ast.decls) {
      if (decl.kind === 'import') {
        // path is e.g., ["Math"] for `import Math.{add}`
        const importModuleName = decl.path[0];
        let resolved: ResolvedModule;
        try {
          resolved = this.resolver.resolve(importModuleName, filePath);
        } catch (e) {
          if (e instanceof ModuleError) {
            throw e;
          }
          throw e;
        }

        // Validate that all requested items are public
        for (const item of decl.items) {
          if (!resolved.exports.has(item)) {
            if (resolved.allDecls.has(item)) {
              throw new ModuleError(
                `Cannot import "${item}" from module "${importModuleName}": ` +
                `"${item}" is not public. Add "pub" to export it.`
              );
            } else {
              throw new ModuleError(
                `Cannot import "${item}" from module "${importModuleName}": ` +
                `"${item}" does not exist in that module.`
              );
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

  private topologicalSort(): ModuleNode[] {
    const result: ModuleNode[] = [];
    const visited = new Set<string>();
    const visiting = new Set<string>(); // for cycle detection

    const visit = (filePath: string, chain: string[]) => {
      if (visited.has(filePath)) return;
      if (visiting.has(filePath)) {
        const cycle = [...chain, filePath].map(p => path.basename(p, '.japl'));
        throw new ModuleError(
          `Circular dependency detected: ${cycle.join(' -> ')}`
        );
      }

      visiting.add(filePath);
      const node = this.modules.get(filePath)!;

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

import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { TsEmitter } from '../codegen/emit.js';
import { CEmitter } from '../codegen/emit_c.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { execFileSync } from 'node:child_process';

export type Target = 'ts' | 'c';

export function buildToString(source: string, target: Target = 'ts'): string {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  const ir = lowerModule(ast);

  if (target === 'c') {
    const emitter = new CEmitter();
    return emitter.emit(ir);
  }

  const emitter = new TsEmitter();
  return emitter.emit(ir);
}

export function buildFile(inputPath: string, outputPath?: string, target: Target = 'ts'): void {
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
      } catch {
        console.error('Failed to compile C output with gcc');
      }
    }
  }
}

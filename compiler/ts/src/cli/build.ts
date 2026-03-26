import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { TsEmitter } from '../codegen/emit.js';
import * as fs from 'node:fs';
import * as path from 'node:path';

export function buildToString(source: string): string {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  const ir = lowerModule(ast);
  const emitter = new TsEmitter();
  return emitter.emit(ir);
}

export function buildFile(inputPath: string, outputPath?: string): void {
  const source = fs.readFileSync(inputPath, 'utf-8');
  const tsCode = buildToString(source);

  const outPath = outputPath ?? inputPath.replace('.japl', '.ts');
  const dir = path.dirname(outPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  fs.writeFileSync(outPath, tsCode);
  console.log(`Compiled ${inputPath} → ${outPath}`);
}

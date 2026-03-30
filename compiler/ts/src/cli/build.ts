import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { lowerModule } from '../ir/lower.js';
import { WatEmitter } from '../codegen/emit_wat.js';
import { TypeChecker } from '../checker/infer.js';
import { LinearityChecker } from '../checker/linearity.js';
import * as fs from 'node:fs';
import * as path from 'node:path';

export interface BuildOptions {
  strict?: boolean;
  emitWat?: boolean;
}

export function buildToWat(source: string, opts: BuildOptions = {}): string {
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
    const effectErrors = typed.errors.filter(e =>
      e.message.includes('effect') || e.message.includes('pure') || e.message.includes('Effect') || e.message.includes('Pure')
    );
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
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive') || e.message.includes('exhaustive')
    );
    if (exhaustErrors.length > 0) {
      throw new Error(`Exhaustiveness violations:\n${exhaustErrors.map(e => `  ${e.toString()}`).join('\n')}`);
    }
  }

  // Lower to IR
  const ir = lowerModule(ast);

  // Emit WAT (stub for now)
  const emitter = new WatEmitter();
  return emitter.emit(ir);
}

export function buildFile(inputPath: string, outputPath?: string): void {
  const source = fs.readFileSync(inputPath, 'utf-8');
  const wat = buildToWat(source);

  const outPath = outputPath ?? inputPath.replace(/\.japl$/, '.wat');
  const dir = path.dirname(outPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  fs.writeFileSync(outPath, wat);
  console.log(`Compiled ${inputPath} → ${outPath}`);
}

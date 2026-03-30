// compiler/ts/src/codegen/emit_wat.ts
// JAPL IR → WAT (WebAssembly Text Format) code generator
//
// Emits WAT that runs under WASI (wasmtime, wasmer, etc.)
// Supports: integers (i64), booleans (i32), strings (linear memory),
//           functions, if/else, recursion, println, show

import * as IR from '../ir/ir.js';

export class WatEmitter {
  private output: string[] = [];
  private indent: number = 0;
  private localCounter: number = 0;
  private stringData: { offset: number; bytes: Uint8Array; text: string }[] = [];
  private memoryOffset: number = 0;
  private currentFnLocals: Map<string, string> = new Map(); // name -> wasm type
  private declaredLocals: string[] = [];
  private userFunctions: Set<string> = new Set();

  emit(module: IR.IrModule): string {
    this.output = [];
    this.stringData = [];
    this.memoryOffset = 1024; // reserve 0-1023 for iov/scratch
    this.userFunctions.clear();

    // Collect user function names
    for (const decl of module.decls) {
      if (decl.kind === 'fn') {
        this.userFunctions.add(decl.name);
      }
    }

    this.line('(module');
    this.indent++;

    // WASI imports
    this.emitWasiImports();

    // Memory
    this.line('(memory (export "memory") 1)');
    this.line('');

    // Collect strings and emit data segments
    this.collectStrings(module);
    this.emitDataSegments();

    // Built-in helper functions
    this.emitPrintln();
    this.line('');
    this.emitShowI64();
    this.line('');

    // User functions
    for (const decl of module.decls) {
      if (decl.kind === 'fn' && decl.name !== 'main') {
        this.emitFunction(decl);
        this.line('');
      }
    }

    // main function
    for (const decl of module.decls) {
      if (decl.kind === 'fn' && decl.name === 'main') {
        this.emitFunction(decl);
        this.line('');
      }
    }

    // _start entry point
    this.emitStart();

    this.indent--;
    this.line(')');

    return this.output.join('\n') + '\n';
  }

  emitModule(module: IR.IrModule, _options?: { isEntry?: boolean; importRewrites?: Map<string, string> }): string {
    return this.emit(module);
  }

  // ─── WASI Imports ───

  private emitWasiImports(): void {
    this.line('(import "wasi_snapshot_preview1" "fd_write"');
    this.line('  (func $fd_write (param i32 i32 i32 i32) (result i32)))');
    this.line('(import "wasi_snapshot_preview1" "proc_exit"');
    this.line('  (func $proc_exit (param i32)))');
    this.line('');
  }

  // ─── String Collection ───

  private collectStrings(module: IR.IrModule): void {
    for (const decl of module.decls) {
      if (decl.kind === 'fn' || decl.kind === 'test') {
        this.collectStringsFromExpr(decl.body);
      }
    }
  }

  private collectStringsFromExpr(expr: IR.IrExpr): void {
    switch (expr.kind) {
      case 'string':
        this.addString(this.stripQuotes(expr.value));
        break;
      case 'app':
        this.collectStringsFromExpr(expr.fn);
        for (const a of expr.args) this.collectStringsFromExpr(a);
        break;
      case 'let':
        this.collectStringsFromExpr(expr.value);
        this.collectStringsFromExpr(expr.body);
        break;
      case 'if':
        this.collectStringsFromExpr(expr.cond);
        this.collectStringsFromExpr(expr.then);
        this.collectStringsFromExpr(expr.else);
        break;
      case 'binop':
        this.collectStringsFromExpr(expr.left);
        this.collectStringsFromExpr(expr.right);
        break;
      case 'unaryop':
        this.collectStringsFromExpr(expr.operand);
        break;
      case 'block':
        for (const e of expr.exprs) this.collectStringsFromExpr(e);
        break;
      case 'match':
        this.collectStringsFromExpr(expr.scrutinee);
        for (const arm of expr.arms) this.collectStringsFromExpr(arm.body);
        break;
      case 'lambda':
        this.collectStringsFromExpr(expr.body);
        break;
      default:
        break;
    }
  }

  private stripQuotes(s: string): string {
    if (s.startsWith('"') && s.endsWith('"')) {
      return s.slice(1, -1);
    }
    return s;
  }

  private addString(text: string): { offset: number; bytes: Uint8Array; text: string } {
    // Check if already added
    const existing = this.stringData.find(s => s.text === text);
    if (existing) return existing;

    const encoder = new TextEncoder();
    const bytes = encoder.encode(text);
    const offset = this.memoryOffset;

    // Layout: [4 bytes length (i32 LE)] [UTF-8 bytes]
    this.memoryOffset += 4 + bytes.length;
    // Align to 4 bytes
    this.memoryOffset = (this.memoryOffset + 3) & ~3;

    const entry = { offset, bytes, text };
    this.stringData.push(entry);
    return entry;
  }

  private findString(text: string): { offset: number; bytes: Uint8Array; text: string } | undefined {
    return this.stringData.find(s => s.text === text);
  }

  private emitDataSegments(): void {
    for (const entry of this.stringData) {
      // Emit length as 4 LE bytes, then the UTF-8 string bytes
      const lenBytes = new Uint8Array(4);
      new DataView(lenBytes.buffer).setUint32(0, entry.bytes.length, true);
      const allBytes = new Uint8Array(4 + entry.bytes.length);
      allBytes.set(lenBytes, 0);
      allBytes.set(entry.bytes, 4);

      const escaped = this.escapeDataString(allBytes);
      this.line(`(data (i32.const ${entry.offset}) "${escaped}")`);
    }
    if (this.stringData.length > 0) {
      this.line('');
    }
  }

  private escapeDataString(bytes: Uint8Array): string {
    let result = '';
    for (let i = 0; i < bytes.length; i++) {
      const b = bytes[i];
      if (b >= 0x20 && b < 0x7f && b !== 0x22 && b !== 0x5c) {
        result += String.fromCharCode(b);
      } else {
        result += '\\' + b.toString(16).padStart(2, '0');
      }
    }
    return result;
  }

  // ─── Built-in Functions ───

  private emitPrintln(): void {
    // $println: takes i32 string pointer, prints string + newline via WASI fd_write
    this.line('(func $println (param $str_ptr i32)');
    this.indent++;
    // Set up iov at memory[0]: { buf_ptr, buf_len }
    // buf_ptr = str_ptr + 4 (skip length prefix)
    this.line('(i32.store (i32.const 0) (i32.add (local.get $str_ptr) (i32.const 4)))');
    // buf_len = load length from str_ptr
    this.line('(i32.store (i32.const 4) (i32.load (local.get $str_ptr)))');
    // fd_write(fd=1, iovs=0, iovs_len=1, nwritten=8)
    this.line('(drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))');
    // Write newline
    this.line('(i32.store8 (i32.const 12) (i32.const 10))');
    this.line('(i32.store (i32.const 0) (i32.const 12))');
    this.line('(i32.store (i32.const 4) (i32.const 1))');
    this.line('(drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))');
    this.indent--;
    this.line(')');
  }

  private emitShowI64(): void {
    // $show_i64: converts i64 to decimal string in memory, returns i32 pointer
    // Uses scratch space at memoryOffset for the result
    // Strategy: extract digits in reverse, then reverse into final string
    this.line('(func $show_i64 (param $val i64) (result i32)');
    this.indent++;
    this.line('(local $buf_start i32)');
    this.line('(local $pos i32)');
    this.line('(local $is_neg i32)');
    this.line('(local $digit i64)');
    this.line('(local $str_ptr i32)');
    this.line('(local $len i32)');
    this.line('(local $tmp i64)');

    // Use a scratch buffer at 256 for digit extraction (well before string data at 1024)
    this.line('(local.set $buf_start (i32.const 256))');
    this.line('(local.set $pos (i32.const 0))');
    this.line('(local.set $is_neg (i32.const 0))');

    // Handle zero
    this.line('(if (i64.eqz (local.get $val))');
    this.line('  (then');
    this.indent++;
    // Write "0" string: length=1, char='0'
    this.line(`(i32.store (i32.const 280) (i32.const 1))`);
    this.line(`(i32.store8 (i32.const 284) (i32.const 48))`);
    this.line('(return (i32.const 280))');
    this.indent--;
    this.line('  )');
    this.line(')');

    // Handle negative
    this.line('(if (i64.lt_s (local.get $val) (i64.const 0))');
    this.line('  (then');
    this.indent++;
    this.line('(local.set $is_neg (i32.const 1))');
    this.line('(local.set $val (i64.sub (i64.const 0) (local.get $val)))');
    this.indent--;
    this.line('  )');
    this.line(')');

    // Extract digits (stored in reverse order at buf_start)
    this.line('(local.set $tmp (local.get $val))');
    this.line('(block $done');
    this.line('  (loop $extract');
    this.line('    (br_if $done (i64.eqz (local.get $tmp)))');
    this.line('    (local.set $digit (i64.rem_u (local.get $tmp) (i64.const 10)))');
    this.line('    (i32.store8');
    this.line('      (i32.add (local.get $buf_start) (local.get $pos))');
    this.line('      (i32.add (i32.const 48) (i32.wrap_i64 (local.get $digit)))');
    this.line('    )');
    this.line('    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))');
    this.line('    (local.set $tmp (i64.div_u (local.get $tmp) (i64.const 10)))');
    this.line('    (br $extract)');
    this.line('  )');
    this.line(')');

    // Calculate total length including sign
    this.line('(local.set $len (i32.add (local.get $pos) (local.get $is_neg)))');

    // Build string at 288 (after the zero string)
    this.line('(local.set $str_ptr (i32.const 288))');
    // Write length
    this.line('(i32.store (local.get $str_ptr) (local.get $len))');

    // Write '-' if negative
    this.line('(if (local.get $is_neg)');
    this.line('  (then');
    this.line('    (i32.store8 (i32.add (local.get $str_ptr) (i32.const 4)) (i32.const 45))');
    this.line('  )');
    this.line(')');

    // Copy digits in reverse order
    // We need a loop: for i = 0..pos-1, copy buf_start[pos-1-i] to str_ptr+4+is_neg+i
    this.line('(local.set $tmp (i64.const 0))'); // reuse as loop counter
    this.line('(block $copy_done');
    this.line('  (loop $copy');
    this.line('    (br_if $copy_done (i64.ge_u (local.get $tmp) (i64.extend_i32_u (local.get $pos))))');
    this.line('    (i32.store8');
    this.line('      (i32.add (i32.add (local.get $str_ptr) (i32.const 4)) (i32.add (local.get $is_neg) (i32.wrap_i64 (local.get $tmp))))');
    this.line('      (i32.load8_u (i32.add (local.get $buf_start) (i32.sub (i32.sub (local.get $pos) (i32.const 1)) (i32.wrap_i64 (local.get $tmp)))))');
    this.line('    )');
    this.line('    (local.set $tmp (i64.add (local.get $tmp) (i64.const 1)))');
    this.line('    (br $copy)');
    this.line('  )');
    this.line(')');

    this.line('(local.get $str_ptr)');
    this.indent--;
    this.line(')');
  }

  // ─── Function Emission ───

  private emitFunction(decl: IR.IrDecl & { kind: 'fn' }): void {
    this.currentFnLocals = new Map();
    this.declaredLocals = [];
    this.localCounter = 0;

    // Register params
    for (const p of decl.params) {
      this.currentFnLocals.set(p, 'i64'); // assume all params are i64 for now
    }

    // Pre-scan body for let bindings to declare locals
    this.collectLocals(decl.body);

    // Determine result type
    const resultType = this.inferResultType(decl.body);

    // Build param string
    const params = decl.params.map(p => `(param $${p} i64)`).join(' ');
    const resultStr = resultType === 'void' ? '' : ` (result ${resultType})`;

    if (decl.name === 'main') {
      this.line(`(func $main${params ? ' ' + params : ''}${resultStr}`);
    } else {
      this.line(`(func $${decl.name}${params ? ' ' + params : ''}${resultStr}`);
    }
    this.indent++;

    // Declare locals
    for (const localName of this.declaredLocals) {
      const type = this.currentFnLocals.get(localName)!;
      this.line(`(local $${localName} ${type})`);
    }

    // Emit body
    const bodyLines = this.emitExpr(decl.body, resultType === 'void');
    for (const l of bodyLines) {
      this.line(l);
    }

    this.indent--;
    this.line(')');
  }

  private collectLocals(expr: IR.IrExpr): void {
    switch (expr.kind) {
      case 'let':
        if (!this.currentFnLocals.has(expr.name)) {
          const type = this.inferExprType(expr.value);
          this.currentFnLocals.set(expr.name, type);
          this.declaredLocals.push(expr.name);
        }
        this.collectLocals(expr.value);
        this.collectLocals(expr.body);
        break;
      case 'if':
        this.collectLocals(expr.cond);
        this.collectLocals(expr.then);
        this.collectLocals(expr.else);
        break;
      case 'binop':
        this.collectLocals(expr.left);
        this.collectLocals(expr.right);
        break;
      case 'unaryop':
        this.collectLocals(expr.operand);
        break;
      case 'app':
        for (const a of expr.args) this.collectLocals(a);
        break;
      case 'block':
        for (const e of expr.exprs) this.collectLocals(e);
        break;
      case 'match':
        this.collectLocals(expr.scrutinee);
        for (const arm of expr.arms) this.collectLocals(arm.body);
        break;
      default:
        break;
    }
  }

  // ─── Type Inference (simple) ───

  private inferExprType(expr: IR.IrExpr): string {
    switch (expr.kind) {
      case 'int': return 'i64';
      case 'float': return 'f64';
      case 'bool': return 'i32';
      case 'string': return 'i32'; // pointer
      case 'unit': return 'void';
      case 'var': {
        const t = this.currentFnLocals.get(expr.name);
        return t ?? 'i64';
      }
      case 'binop': {
        if (['==', '!=', '<', '>', '<=', '>='].includes(expr.op)) return 'i32';
        if (['&&', '||'].includes(expr.op)) return 'i32';
        return this.inferExprType(expr.left);
      }
      case 'unaryop': {
        if (expr.op === '!') return 'i32';
        return this.inferExprType(expr.operand);
      }
      case 'if': return this.inferExprType(expr.then);
      case 'let': return this.inferExprType(expr.body);
      case 'app': return this.inferAppType(expr);
      case 'block': {
        if (expr.exprs.length === 0) return 'void';
        return this.inferExprType(expr.exprs[expr.exprs.length - 1]);
      }
      default: return 'i64';
    }
  }

  private inferAppType(expr: IR.IrExpr & { kind: 'app' }): string {
    if (expr.fn.kind === 'var') {
      switch (expr.fn.name) {
        case 'println': return 'void';
        case 'show': return 'i32'; // returns string pointer
        default: return 'i64'; // user functions return i64 by default
      }
    }
    return 'i64';
  }

  private inferResultType(body: IR.IrExpr): string {
    return this.inferExprType(body);
  }

  // ─── Expression Emission ───

  private emitExpr(expr: IR.IrExpr, isVoid: boolean = false): string[] {
    switch (expr.kind) {
      case 'int':
        return [`i64.const ${expr.value}`];

      case 'float':
        return [`f64.const ${expr.value}`];

      case 'bool':
        return [`i32.const ${expr.value ? 1 : 0}`];

      case 'string': {
        const text = this.stripQuotes(expr.value);
        const entry = this.findString(text);
        if (!entry) {
          throw new Error(`String not found in data: ${text}`);
        }
        return [`i32.const ${entry.offset}`];
      }

      case 'unit':
        return [];

      case 'var':
        return [`local.get $${expr.name}`];

      case 'let': {
        const lines: string[] = [];
        lines.push(...this.emitExpr(expr.value));
        lines.push(`local.set $${expr.name}`);
        lines.push(...this.emitExpr(expr.body, isVoid));
        return lines;
      }

      case 'binop':
        return this.emitBinop(expr);

      case 'unaryop':
        return this.emitUnaryop(expr);

      case 'if':
        return this.emitIf(expr, isVoid);

      case 'app':
        return this.emitApp(expr, isVoid);

      case 'block':
        return this.emitBlock(expr, isVoid);

      default:
        return [`;; TODO: ${expr.kind}`];
    }
  }

  private emitBinop(expr: IR.IrExpr & { kind: 'binop' }): string[] {
    const lines: string[] = [];
    const leftType = this.inferExprType(expr.left);

    lines.push(...this.emitExpr(expr.left));
    lines.push(...this.emitExpr(expr.right));

    if (leftType === 'f64') {
      lines.push(this.floatBinop(expr.op));
    } else {
      lines.push(this.intBinop(expr.op));
    }
    return lines;
  }

  private intBinop(op: string): string {
    switch (op) {
      case '+': return 'i64.add';
      case '-': return 'i64.sub';
      case '*': return 'i64.mul';
      case '/': return 'i64.div_s';
      case '%': return 'i64.rem_s';
      case '==': return 'i64.eq';
      case '!=': return 'i64.ne';
      case '<': return 'i64.lt_s';
      case '>': return 'i64.gt_s';
      case '<=': return 'i64.le_s';
      case '>=': return 'i64.ge_s';
      case '&&': return 'i32.and';
      case '||': return 'i32.or';
      default: return `;; unknown op: ${op}`;
    }
  }

  private floatBinop(op: string): string {
    switch (op) {
      case '+': return 'f64.add';
      case '-': return 'f64.sub';
      case '*': return 'f64.mul';
      case '/': return 'f64.div';
      case '==': return 'f64.eq';
      case '!=': return 'f64.ne';
      case '<': return 'f64.lt';
      case '>': return 'f64.gt';
      case '<=': return 'f64.le';
      case '>=': return 'f64.ge';
      default: return `;; unknown float op: ${op}`;
    }
  }

  private emitUnaryop(expr: IR.IrExpr & { kind: 'unaryop' }): string[] {
    const lines: string[] = [];
    lines.push(...this.emitExpr(expr.operand));
    switch (expr.op) {
      case '-':
        lines.push('i64.const 0');
        // swap: need to negate, so push 0 first then sub
        // Actually we need 0 - operand, so rewrite:
        return [`i64.const 0`, ...this.emitExpr(expr.operand), 'i64.sub'];
      case '!':
        lines.push('i32.eqz');
        return lines;
      default:
        lines.push(`;; unknown unary op: ${expr.op}`);
        return lines;
    }
  }

  private emitIf(expr: IR.IrExpr & { kind: 'if' }, isVoid: boolean): string[] {
    const lines: string[] = [];
    const resultType = isVoid ? 'void' : this.inferExprType(expr.then);

    // Emit condition
    lines.push(...this.emitCondition(expr.cond));

    if (resultType === 'void') {
      lines.push('(if');
    } else {
      lines.push(`(if (result ${resultType})`);
    }
    lines.push('  (then');
    const thenLines = this.emitExpr(expr.then, isVoid);
    for (const l of thenLines) {
      lines.push('    ' + l);
    }
    lines.push('  )');
    lines.push('  (else');
    const elseLines = this.emitExpr(expr.else, isVoid);
    for (const l of elseLines) {
      lines.push('    ' + l);
    }
    lines.push('  )');
    lines.push(')');
    return lines;
  }

  private emitCondition(expr: IR.IrExpr): string[] {
    // If the condition is a comparison binop, it already returns i32
    if (expr.kind === 'binop' && ['==', '!=', '<', '>', '<=', '>='].includes(expr.op)) {
      return this.emitExpr(expr);
    }
    // If it's a bool, it's already i32
    if (expr.kind === 'bool') {
      return this.emitExpr(expr);
    }
    // Otherwise emit and ensure i32
    const lines = this.emitExpr(expr);
    // If it's an i64, wrap to i32 (truthy = nonzero)
    const type = this.inferExprType(expr);
    if (type === 'i64') {
      lines.push('i64.const 0');
      lines.push('i64.ne');
    }
    return lines;
  }

  private emitApp(expr: IR.IrExpr & { kind: 'app' }, isVoid: boolean): string[] {
    const lines: string[] = [];

    if (expr.fn.kind === 'var') {
      const fnName = expr.fn.name;

      // Built-in: println
      if (fnName === 'println') {
        // println takes a string pointer (i32)
        if (expr.args.length === 1) {
          lines.push(...this.emitExpr(expr.args[0]));
          lines.push('call $println');
        }
        return lines;
      }

      // Built-in: show (int to string)
      if (fnName === 'show') {
        if (expr.args.length === 1) {
          lines.push(...this.emitExpr(expr.args[0]));
          lines.push('call $show_i64');
        }
        return lines;
      }

      // User function call
      for (const arg of expr.args) {
        lines.push(...this.emitExpr(arg));
      }
      lines.push(`call $${fnName}`);

      // If call result unused in void context, drop it
      if (isVoid) {
        const retType = this.inferAppType(expr);
        if (retType !== 'void') {
          lines.push('drop');
        }
      }
      return lines;
    }

    // Fallback: indirect call not supported yet
    lines.push(`;; TODO: indirect call`);
    return lines;
  }

  private emitBlock(expr: IR.IrExpr & { kind: 'block' }, isVoid: boolean): string[] {
    const lines: string[] = [];
    for (let i = 0; i < expr.exprs.length; i++) {
      const isLast = i === expr.exprs.length - 1;
      const subVoid = isLast ? isVoid : true;
      const subLines = this.emitExpr(expr.exprs[i], subVoid);
      lines.push(...subLines);

      // If not the last expression and it leaves a value on the stack, drop it
      if (!isLast) {
        const exprType = this.inferExprType(expr.exprs[i]);
        if (exprType !== 'void') {
          // Check if the expression is a void call (println etc)
          // inferExprType handles this already
        }
      }
    }
    return lines;
  }

  // ─── _start ───

  private emitStart(): void {
    this.line('(func (export "_start")');
    this.indent++;
    this.line('call $main');
    this.indent--;
    this.line(')');
  }

  // ─── Helpers ───

  private line(text: string): void {
    const prefix = '  '.repeat(this.indent);
    this.output.push(prefix + text);
  }
}

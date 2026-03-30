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

  // Tagged union registry: variant name -> { typeName, tagId, fieldCount }
  private variantRegistry: Map<string, { typeName: string; tagId: number; fieldCount: number }> = new Map();
  // Record type registry: type name -> sorted field names
  private recordTypeRegistry: Map<string, string[]> = new Map();
  // Lambda storage: generated functions to emit
  private lambdaFunctions: { name: string; params: string[]; body: IR.IrExpr }[] = [];
  private lambdaCounter: number = 0;
  // Track lambda names mapped to variable names
  private lambdaNameMap: Map<string, string> = new Map();
  // Heap start (after string data segments)
  private heapStart: number = 0;

  emit(module: IR.IrModule): string {
    this.output = [];
    this.stringData = [];
    this.memoryOffset = 1024; // reserve 0-1023 for iov/scratch
    this.userFunctions.clear();
    this.variantRegistry.clear();
    this.recordTypeRegistry.clear();
    this.lambdaFunctions = [];
    this.lambdaCounter = 0;
    this.lambdaNameMap.clear();

    // Register type declarations (variants and record types)
    for (const decl of module.decls) {
      if (decl.kind === 'type') {
        for (let i = 0; i < decl.variants.length; i++) {
          const v = decl.variants[i];
          this.variantRegistry.set(v.name, {
            typeName: decl.name,
            tagId: i,
            fieldCount: v.fields,
          });
        }
      }
      if (decl.kind === 'record_type') {
        const sortedFields = decl.fields.map(f => f[0]).sort();
        this.recordTypeRegistry.set(decl.name, sortedFields);
      }
    }

    // Collect user function names (including variant constructors)
    for (const decl of module.decls) {
      if (decl.kind === 'fn') {
        this.userFunctions.add(decl.name);
      }
    }
    for (const [name] of this.variantRegistry) {
      this.userFunctions.add(name);
    }

    // Pre-scan for lambdas to generate named functions
    for (const decl of module.decls) {
      if (decl.kind === 'fn' || decl.kind === 'test') {
        this.collectLambdas(decl.body);
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

    // Heap pointer global (placed after all string data)
    this.heapStart = (this.memoryOffset + 7) & ~7; // align to 8 bytes
    this.line(`(global $heap_ptr (mut i32) (i32.const ${this.heapStart}))`);
    this.line('');

    // Bump allocator
    this.emitAlloc();
    this.line('');

    // Built-in helper functions
    this.emitPrintln();
    this.line('');
    this.emitShowI64();
    this.line('');

    // String concat helper
    this.emitStringConcat();
    this.line('');

    // List helpers (cons and nil)
    this.emitListHelpers();
    this.line('');

    // Variant constructor functions
    for (const [name, info] of this.variantRegistry) {
      this.emitVariantConstructor(name, info.tagId, info.fieldCount);
      this.line('');
    }

    // Lambda functions
    for (const lam of this.lambdaFunctions) {
      this.emitLambdaFunction(lam);
      this.line('');
    }

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
        for (const arm of expr.arms) {
          this.collectStringsFromExpr(arm.body);
          if (arm.guard) this.collectStringsFromExpr(arm.guard);
        }
        break;
      case 'lambda':
        this.collectStringsFromExpr(expr.body);
        break;
      case 'construct':
        for (const a of expr.args) this.collectStringsFromExpr(a);
        break;
      case 'concat':
        this.collectStringsFromExpr(expr.left);
        this.collectStringsFromExpr(expr.right);
        break;
      case 'record':
        for (const [, val] of expr.fields) this.collectStringsFromExpr(val);
        break;
      case 'field_access':
        this.collectStringsFromExpr(expr.expr);
        break;
      case 'record_update':
        this.collectStringsFromExpr(expr.record);
        for (const [, val] of expr.updates) this.collectStringsFromExpr(val);
        break;
      case 'list':
        for (const e of expr.elements) this.collectStringsFromExpr(e);
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

    // Pre-scan body for let bindings and temp locals to declare
    this.collectLocals(decl.body);

    // Reset counter so emitExpr generates matching names
    this.localCounter = 0;

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
        this.collectLocals(expr.fn);
        for (const a of expr.args) this.collectLocals(a);
        break;
      case 'block':
        for (const e of expr.exprs) this.collectLocals(e);
        break;
      case 'match': {
        this.collectLocals(expr.scrutinee);
        // Pre-allocate the match scrutinee local (must match emitMatch counter)
        const matchLocalName = `__match_${this.localCounter++}`;
        if (!this.currentFnLocals.has(matchLocalName)) {
          this.currentFnLocals.set(matchLocalName, 'i32');
          this.declaredLocals.push(matchLocalName);
        }
        for (const arm of expr.arms) {
          this.collectMatchPatternLocals(arm.pattern);
          this.collectLocals(arm.body);
          if (arm.guard) this.collectLocals(arm.guard);
        }
        break;
      }
      case 'construct':
        for (const a of expr.args) this.collectLocals(a);
        break;
      case 'concat':
        this.collectLocals(expr.left);
        this.collectLocals(expr.right);
        break;
      case 'record': {
        // Pre-allocate record pointer local + field temp locals
        const sorted = [...expr.fields].sort((a, b) => a[0].localeCompare(b[0]));
        const recPtrName = `__rec_${this.localCounter++}`;
        if (!this.currentFnLocals.has(recPtrName)) {
          this.currentFnLocals.set(recPtrName, 'i32');
          this.declaredLocals.push(recPtrName);
        }
        for (let i = 0; i < sorted.length; i++) {
          const tmpName = `__rec_tmp_${i}`;
          if (!this.currentFnLocals.has(tmpName)) {
            this.currentFnLocals.set(tmpName, 'i64');
            this.declaredLocals.push(tmpName);
          }
        }
        for (const [, val] of expr.fields) this.collectLocals(val);
        break;
      }
      case 'field_access':
        this.collectLocals(expr.expr);
        break;
      case 'record_update': {
        this.collectLocals(expr.record);
        // Pre-allocate temp locals for record update (must match emitRecordUpdate counter)
        const srcName = `__rupd_src_${this.localCounter++}`;
        const dstName = `__rupd_dst_${this.localCounter++}`;
        const cntName = `__rupd_cnt_${this.localCounter++}`;
        const szName = `__rupd_sz_${this.localCounter++}`;
        const idxName = `__rupd_i_${this.localCounter++}`;
        for (const [name, type] of [[srcName, 'i32'], [dstName, 'i32'], [cntName, 'i32'], [szName, 'i32'], [idxName, 'i32']] as [string, string][]) {
          if (!this.currentFnLocals.has(name)) {
            this.currentFnLocals.set(name, type);
            this.declaredLocals.push(name);
          }
        }
        for (const [, val] of expr.updates) {
          const tmpName = `__rupd_val_${this.localCounter++}`;
          if (!this.currentFnLocals.has(tmpName)) {
            this.currentFnLocals.set(tmpName, 'i64');
            this.declaredLocals.push(tmpName);
          }
          this.collectLocals(val);
        }
        break;
      }
      case 'list': {
        // Pre-allocate list tail temp locals
        for (let i = expr.elements.length - 1; i >= 0; i--) {
          const tailName = `__list_tail_${this.localCounter++}`;
          if (!this.currentFnLocals.has(tailName)) {
            this.currentFnLocals.set(tailName, 'i64');
            this.declaredLocals.push(tailName);
          }
        }
        for (const e of expr.elements) this.collectLocals(e);
        break;
      }
      case 'lambda':
        this.collectLocals(expr.body);
        break;
      default:
        break;
    }
  }

  private collectMatchPatternLocals(pat: IR.IrPattern): void {
    switch (pat.kind) {
      case 'pvar':
        if (!this.currentFnLocals.has(pat.name)) {
          this.currentFnLocals.set(pat.name, 'i64');
          this.declaredLocals.push(pat.name);
        }
        break;
      case 'pconstructor':
        for (const arg of pat.args) this.collectMatchPatternLocals(arg);
        break;
      case 'pwildcard':
      case 'pliteral':
        break;
      case 'plist':
        for (const el of pat.elements) this.collectMatchPatternLocals(el);
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
      case 'construct': return 'i64'; // pointer as i64
      case 'match': {
        if (expr.arms.length > 0) return this.inferExprType(expr.arms[0].body);
        return 'i64';
      }
      case 'record': return 'i64'; // pointer as i64
      case 'field_access': return 'i64';
      case 'record_update': return 'i64';
      case 'list': return 'i64';
      case 'concat': return 'i64'; // returns string pointer as i64
      case 'lambda': return 'i64'; // function reference as i64
      default: return 'i64';
    }
  }

  private inferAppType(expr: IR.IrExpr & { kind: 'app' }): string {
    if (expr.fn.kind === 'var') {
      switch (expr.fn.name) {
        case 'println': return 'void';
        case 'show': return 'i32'; // returns string pointer
        default: {
          // Check if this is a variant constructor
          if (this.variantRegistry.has(expr.fn.name)) return 'i64';
          // Check if this is a lambda reference
          if (this.lambdaNameMap.has(expr.fn.name)) return 'i64';
          return 'i64'; // user functions return i64 by default
        }
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

      case 'construct':
        return this.emitConstruct(expr);

      case 'match':
        return this.emitMatch(expr);

      case 'record':
        return this.emitRecord(expr);

      case 'field_access':
        return this.emitFieldAccess(expr);

      case 'record_update':
        return this.emitRecordUpdate(expr);

      case 'list':
        return this.emitList(expr);

      case 'concat':
        return this.emitConcat(expr);

      case 'lambda':
        return this.emitLambdaRef(expr);

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
          // If the argument is i64 (e.g., from concat), convert to i32
          const argType = this.inferExprType(expr.args[0]);
          if (argType === 'i64') {
            lines.push('i32.wrap_i64');
          }
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

      // Check if this is a lambda reference
      const lambdaName = this.lambdaNameMap.get(fnName);
      const callTarget = lambdaName ?? fnName;

      // User function call (or lambda call)
      for (const arg of expr.args) {
        lines.push(...this.emitExpr(arg));
      }
      lines.push(`call $${callTarget}`);

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

  // ─── Construct (Tagged Union) ───

  private emitConstruct(expr: IR.IrExpr & { kind: 'construct' }): string[] {
    const lines: string[] = [];
    const info = this.variantRegistry.get(expr.tag);
    if (info) {
      // Call the generated constructor function
      for (const arg of expr.args) {
        lines.push(...this.emitExpr(arg));
      }
      lines.push(`call $${expr.tag}`);
    } else {
      lines.push(`;; unknown constructor: ${expr.tag}`);
    }
    return lines;
  }

  // ─── Match Expression ───

  private emitMatch(expr: IR.IrExpr & { kind: 'match' }): string[] {
    const lines: string[] = [];
    const matchLocal = `__match_${this.localCounter++}`;

    // Declare match scrutinee local
    if (!this.currentFnLocals.has(matchLocal)) {
      this.currentFnLocals.set(matchLocal, 'i32');
      this.declaredLocals.push(matchLocal);
    }

    // Evaluate scrutinee and store as i32 pointer
    lines.push(...this.emitExpr(expr.scrutinee));
    lines.push('i32.wrap_i64');
    lines.push(`local.set $${matchLocal}`);

    // Determine result type from first arm body
    const resultType = expr.arms.length > 0 ? this.inferExprType(expr.arms[0].body) : 'i64';

    // Generate nested if/else chain
    const matchLines = this.emitMatchArms(expr.arms, matchLocal, resultType, 0);
    lines.push(...matchLines);

    return lines;
  }

  private emitMatchArms(arms: IR.IrMatchArm[], matchLocal: string, resultType: string, index: number): string[] {
    if (index >= arms.length) {
      // Unreachable fallback
      return ['unreachable'];
    }

    const arm = arms[index];
    const pat = arm.pattern;

    // If this is a wildcard or variable pattern (catch-all), just emit the body
    if (pat.kind === 'pwildcard') {
      return this.emitExpr(arm.body);
    }
    if (pat.kind === 'pvar') {
      const lines: string[] = [];
      // Bind the whole scrutinee value (as i64) to the variable
      lines.push(`local.get $${matchLocal}`);
      lines.push('i64.extend_i32_u');
      lines.push(`local.set $${pat.name}`);
      lines.push(...this.emitExpr(arm.body));
      return lines;
    }
    if (pat.kind === 'pliteral') {
      const lines: string[] = [];
      // Compare scrutinee (as i64 from pointer) to literal
      // For literal patterns on ints, the scrutinee is already the value
      lines.push(`local.get $${matchLocal}`);
      lines.push('i64.extend_i32_u');
      lines.push(...this.emitExpr(pat.value));
      lines.push('i64.eq');
      lines.push(`(if (result ${resultType})`);
      lines.push('  (then');
      const bodyLines = this.emitExpr(arm.body);
      for (const l of bodyLines) lines.push('    ' + l);
      lines.push('  )');
      lines.push('  (else');
      const restLines = this.emitMatchArms(arms, matchLocal, resultType, index + 1);
      for (const l of restLines) lines.push('    ' + l);
      lines.push('  )');
      lines.push(')');
      return lines;
    }

    if (pat.kind === 'pconstructor') {
      const info = this.variantRegistry.get(pat.tag);
      if (!info) {
        return [`;; unknown constructor in pattern: ${pat.tag}`];
      }

      const lines: string[] = [];
      // Check tag: i32.load from pointer == tagId
      lines.push(`local.get $${matchLocal}`);
      lines.push('i32.load');
      lines.push(`i32.const ${info.tagId}`);
      lines.push('i32.eq');

      lines.push(`(if (result ${resultType})`);
      lines.push('  (then');

      // Extract fields and bind pattern variables
      const bindLines = this.emitPatternBindings(pat, matchLocal);
      for (const l of bindLines) lines.push('    ' + l);

      const bodyLines = this.emitExpr(arm.body);
      for (const l of bodyLines) lines.push('    ' + l);

      lines.push('  )');
      lines.push('  (else');

      // Rest of arms
      if (index + 1 < arms.length) {
        const restLines = this.emitMatchArms(arms, matchLocal, resultType, index + 1);
        for (const l of restLines) lines.push('    ' + l);
      } else {
        lines.push('    unreachable');
      }

      lines.push('  )');
      lines.push(')');
      return lines;
    }

    // Fallback: emit body directly (e.g., for unsupported patterns)
    return this.emitExpr(arm.body);
  }

  private emitPatternBindings(pat: IR.IrPattern, matchLocal: string): string[] {
    const lines: string[] = [];
    if (pat.kind === 'pconstructor') {
      for (let i = 0; i < pat.args.length; i++) {
        const arg = pat.args[i];
        if (arg.kind === 'pvar') {
          // Field i is at offset 8 + i*8 (after 4-byte tag + 4-byte count)
          const offset = 8 + i * 8;
          lines.push(`(local.set $${arg.name} (i64.load offset=${offset} (local.get $${matchLocal})))`);
        } else if (arg.kind === 'pwildcard') {
          // skip
        }
        // Nested constructors could be handled recursively but kept simple for now
      }
    }
    return lines;
  }

  // ─── Record Expression ───

  private emitRecord(expr: IR.IrExpr & { kind: 'record' }): string[] {
    const lines: string[] = [];
    // Sort fields alphabetically
    const sorted = [...expr.fields].sort((a, b) => a[0].localeCompare(b[0]));
    const fieldCount = sorted.length;
    const size = 4 + fieldCount * 8; // 4 bytes for count + 8 bytes per field

    const ptrLocal = `__rec_${this.localCounter++}`;
    if (!this.currentFnLocals.has(ptrLocal)) {
      this.currentFnLocals.set(ptrLocal, 'i32');
      this.declaredLocals.push(ptrLocal);
    }

    // Allocate
    lines.push(`(local.set $${ptrLocal} (call $alloc (i32.const ${size})))`);
    // Store field count
    lines.push(`(i32.store (local.get $${ptrLocal}) (i32.const ${fieldCount}))`);

    // Store each field value
    for (let i = 0; i < sorted.length; i++) {
      const offset = 4 + i * 8;
      lines.push(...this.emitExpr(sorted[i][1]));
      // Widen to i64 if the value is i32 (e.g., string pointer, bool)
      const valType = this.inferExprType(sorted[i][1]);
      if (valType === 'i32') {
        lines.push('i64.extend_i32_u');
      }
      lines.push(`local.set $__rec_tmp_${i}`);

      // Declare temp local if needed
      const tmpName = `__rec_tmp_${i}`;
      if (!this.currentFnLocals.has(tmpName)) {
        this.currentFnLocals.set(tmpName, 'i64');
        this.declaredLocals.push(tmpName);
      }

      lines.push(`(i64.store offset=${offset} (local.get $${ptrLocal}) (local.get $${tmpName}))`);
    }

    // Return pointer as i64
    lines.push(`(i64.extend_i32_u (local.get $${ptrLocal}))`);
    return lines;
  }

  private getRecordFieldIndex(fields: [string, IR.IrExpr][], fieldName: string): number {
    const sorted = [...fields].map(f => f[0]).sort();
    return sorted.indexOf(fieldName);
  }

  // ─── Field Access ───

  private emitFieldAccess(expr: IR.IrExpr & { kind: 'field_access' }): string[] {
    const lines: string[] = [];
    // We need to figure out the field index. Since we sort alphabetically,
    // we need context about which record type this is. For now, we handle
    // field access by looking up the record expression's field layout.
    // If expr.expr is a var, check record type registry; otherwise use
    // a runtime approach based on known field positions.

    // Try to determine field order from record type registry or from
    // a record literal. For general case, we need the field index.
    const fieldIndex = this.resolveFieldIndex(expr.expr, expr.field);
    const offset = 4 + fieldIndex * 8;

    lines.push(...this.emitExpr(expr.expr));
    lines.push('i32.wrap_i64');
    lines.push(`i64.load offset=${offset}`);
    return lines;
  }

  private resolveFieldIndex(expr: IR.IrExpr, fieldName: string): number {
    // If the expression is a record literal, we can determine the order
    if (expr.kind === 'record') {
      const sorted = [...expr.fields].map(f => f[0]).sort();
      const idx = sorted.indexOf(fieldName);
      if (idx >= 0) return idx;
    }
    // If it's a var, try to look up in record type registry
    // This is a best-effort approach; the field name itself determines position
    // For known record types, look through all registered types
    for (const [, fields] of this.recordTypeRegistry) {
      const idx = fields.indexOf(fieldName);
      if (idx >= 0) return idx;
    }
    // Fallback: hash to an index (not ideal, but for simple cases field name lookup works)
    return 0;
  }

  // ─── Record Update ───

  private emitRecordUpdate(expr: IR.IrExpr & { kind: 'record_update' }): string[] {
    const lines: string[] = [];

    // We need to copy the old record and override specific fields.
    // First, evaluate the source record.
    const srcLocal = `__rupd_src_${this.localCounter++}`;
    const dstLocal = `__rupd_dst_${this.localCounter++}`;
    if (!this.currentFnLocals.has(srcLocal)) {
      this.currentFnLocals.set(srcLocal, 'i32');
      this.declaredLocals.push(srcLocal);
    }
    if (!this.currentFnLocals.has(dstLocal)) {
      this.currentFnLocals.set(dstLocal, 'i32');
      this.declaredLocals.push(dstLocal);
    }

    lines.push(...this.emitExpr(expr.record));
    lines.push('i32.wrap_i64');
    lines.push(`local.set $${srcLocal}`);

    // Read field count from source
    // For simplicity, determine field count from record type registry or updates
    // We'll read it from the source record at runtime
    const countLocal = `__rupd_cnt_${this.localCounter++}`;
    if (!this.currentFnLocals.has(countLocal)) {
      this.currentFnLocals.set(countLocal, 'i32');
      this.declaredLocals.push(countLocal);
    }
    lines.push(`(local.set $${countLocal} (i32.load (local.get $${srcLocal})))`);

    // Allocate new record: 4 + count * 8
    // Use a calculated size
    const sizeLocal = `__rupd_sz_${this.localCounter++}`;
    if (!this.currentFnLocals.has(sizeLocal)) {
      this.currentFnLocals.set(sizeLocal, 'i32');
      this.declaredLocals.push(sizeLocal);
    }
    lines.push(`(local.set $${sizeLocal} (i32.add (i32.const 4) (i32.mul (local.get $${countLocal}) (i32.const 8))))`);
    lines.push(`(local.set $${dstLocal} (call $alloc (local.get $${sizeLocal})))`);

    // Copy field count
    lines.push(`(i32.store (local.get $${dstLocal}) (local.get $${countLocal}))`);

    // Memory copy all fields from source to dest
    // Loop: for i=0..count-1, copy field
    const idxLocal = `__rupd_i_${this.localCounter++}`;
    if (!this.currentFnLocals.has(idxLocal)) {
      this.currentFnLocals.set(idxLocal, 'i32');
      this.declaredLocals.push(idxLocal);
    }
    lines.push(`(local.set $${idxLocal} (i32.const 0))`);
    lines.push(`(block $rupd_done`);
    lines.push(`  (loop $rupd_loop`);
    lines.push(`    (br_if $rupd_done (i32.ge_u (local.get $${idxLocal}) (local.get $${countLocal})))`);
    lines.push(`    (i64.store`);
    lines.push(`      (i32.add (local.get $${dstLocal}) (i32.add (i32.const 4) (i32.mul (local.get $${idxLocal}) (i32.const 8))))`);
    lines.push(`      (i64.load (i32.add (local.get $${srcLocal}) (i32.add (i32.const 4) (i32.mul (local.get $${idxLocal}) (i32.const 8)))))`);
    lines.push(`    )`);
    lines.push(`    (local.set $${idxLocal} (i32.add (local.get $${idxLocal}) (i32.const 1)))`);
    lines.push(`    (br $rupd_loop)`);
    lines.push(`  )`);
    lines.push(`)`);

    // Now override updated fields
    for (const [name, val] of expr.updates) {
      const fieldIdx = this.resolveFieldIndex(expr.record, name);
      const offset = 4 + fieldIdx * 8;
      const tmpLocal = `__rupd_val_${this.localCounter++}`;
      if (!this.currentFnLocals.has(tmpLocal)) {
        this.currentFnLocals.set(tmpLocal, 'i64');
        this.declaredLocals.push(tmpLocal);
      }
      lines.push(...this.emitExpr(val));
      lines.push(`local.set $${tmpLocal}`);
      lines.push(`(i64.store offset=${offset} (local.get $${dstLocal}) (local.get $${tmpLocal}))`);
    }

    // Return pointer as i64
    lines.push(`(i64.extend_i32_u (local.get $${dstLocal}))`);
    return lines;
  }

  // ─── List Expression ───

  private emitList(expr: IR.IrExpr & { kind: 'list' }): string[] {
    const lines: string[] = [];
    // Build list from right to left: [1,2,3] => cons(1, cons(2, cons(3, nil)))
    // Start with nil
    lines.push('call $nil');

    // Then cons each element from right to left
    for (let i = expr.elements.length - 1; i >= 0; i--) {
      // Stack has tail; push head, then swap order for cons(head, tail)
      const tailLocal = `__list_tail_${this.localCounter++}`;
      if (!this.currentFnLocals.has(tailLocal)) {
        this.currentFnLocals.set(tailLocal, 'i64');
        this.declaredLocals.push(tailLocal);
      }
      lines.push(`local.set $${tailLocal}`);
      lines.push(...this.emitExpr(expr.elements[i]));
      lines.push(`local.get $${tailLocal}`);
      lines.push('call $cons');
    }

    return lines;
  }

  // ─── String Concat ───

  private emitConcat(expr: IR.IrExpr & { kind: 'concat' }): string[] {
    const lines: string[] = [];
    // Both operands should be i32 (string pointers) or i64 depending on context.
    // Our string pointers from data segments are i32, but from records they're i64.
    // string_concat expects i64 params (pointers widened to i64)
    lines.push(...this.emitExpr(expr.left));
    // Ensure i64
    if (this.inferExprType(expr.left) === 'i32') {
      lines.push('i64.extend_i32_u');
    }
    lines.push(...this.emitExpr(expr.right));
    if (this.inferExprType(expr.right) === 'i32') {
      lines.push('i64.extend_i32_u');
    }
    lines.push('call $string_concat');
    return lines;
  }

  // ─── Lambda Reference ───

  private emitLambdaRef(expr: IR.IrExpr & { kind: 'lambda' }): string[] {
    // The lambda was already registered during collectLambdas.
    // Find it by matching params + body identity (use counter-based name).
    // For now, return a reference to the lambda function as i64.
    // Since we don't have real closure support (no captured vars),
    // we just return a sentinel that maps to the function index.
    // The caller will use call $__lambda_N directly.
    const name = `__lambda_${this.lambdaCounter++}`;
    // Actually, lambdas are pre-registered with lower counters.
    // Let's look up the last registered lambda matching these params.
    // Simple approach: we store the function name, and emit i64.const 0 as placeholder.
    // Real calls go through emitApp which resolves lambda var names.
    return [`i64.const 0 ;; lambda ref placeholder`];
  }

  // ─── Bump Allocator ───

  private emitAlloc(): void {
    this.line('(func $alloc (param $size i32) (result i32)');
    this.indent++;
    this.line('(local $ptr i32)');
    this.line('global.get $heap_ptr');
    this.line('local.set $ptr');
    this.line('global.get $heap_ptr');
    this.line('local.get $size');
    this.line('i32.add');
    this.line('global.set $heap_ptr');
    this.line('local.get $ptr');
    this.indent--;
    this.line(')');
  }

  // ─── Variant Constructor Functions ───

  private emitVariantConstructor(name: string, tagId: number, fieldCount: number): void {
    const size = 4 + 4 + fieldCount * 8; // tag(4) + count(4) + fields(8 each)
    const params = [];
    for (let i = 0; i < fieldCount; i++) {
      params.push(`(param $v${i} i64)`);
    }

    this.line(`(func $${name} ${params.join(' ')} (result i64)`);
    this.indent++;
    this.line('(local $ptr i32)');
    this.line(`(local.set $ptr (call $alloc (i32.const ${size})))`);
    this.line(`(i32.store (local.get $ptr) (i32.const ${tagId}))`);
    this.line(`(i32.store offset=4 (local.get $ptr) (i32.const ${fieldCount}))`);
    for (let i = 0; i < fieldCount; i++) {
      const offset = 8 + i * 8;
      this.line(`(i64.store offset=${offset} (local.get $ptr) (local.get $v${i}))`);
    }
    this.line('(i64.extend_i32_u (local.get $ptr))');
    this.indent--;
    this.line(')');
  }

  // ─── String Concat Helper ───

  private emitStringConcat(): void {
    // $string_concat(a: i64, b: i64) -> i64
    // a and b are i64 pointers to strings: [4 bytes len][UTF-8 bytes]
    // Returns i64 pointer to new concatenated string
    this.line('(func $string_concat (param $a i64) (param $b i64) (result i64)');
    this.indent++;
    this.line('(local $a_ptr i32) (local $b_ptr i32)');
    this.line('(local $a_len i32) (local $b_len i32)');
    this.line('(local $new_len i32) (local $new_ptr i32)');

    // Get i32 pointers
    this.line('(local.set $a_ptr (i32.wrap_i64 (local.get $a)))');
    this.line('(local.set $b_ptr (i32.wrap_i64 (local.get $b)))');

    // Get lengths
    this.line('(local.set $a_len (i32.load (local.get $a_ptr)))');
    this.line('(local.set $b_len (i32.load (local.get $b_ptr)))');

    // Total length
    this.line('(local.set $new_len (i32.add (local.get $a_len) (local.get $b_len)))');

    // Allocate: 4 + new_len, aligned to 4
    this.line('(local.set $new_ptr (call $alloc (i32.add (i32.const 4) (local.get $new_len))))');

    // Write length
    this.line('(i32.store (local.get $new_ptr) (local.get $new_len))');

    // Copy a's bytes: memory.copy(dst=new_ptr+4, src=a_ptr+4, len=a_len)
    this.line('(memory.copy');
    this.line('  (i32.add (local.get $new_ptr) (i32.const 4))');
    this.line('  (i32.add (local.get $a_ptr) (i32.const 4))');
    this.line('  (local.get $a_len)');
    this.line(')');

    // Copy b's bytes: memory.copy(dst=new_ptr+4+a_len, src=b_ptr+4, len=b_len)
    this.line('(memory.copy');
    this.line('  (i32.add (i32.add (local.get $new_ptr) (i32.const 4)) (local.get $a_len))');
    this.line('  (i32.add (local.get $b_ptr) (i32.const 4))');
    this.line('  (local.get $b_len)');
    this.line(')');

    // Return as i64
    this.line('(i64.extend_i32_u (local.get $new_ptr))');
    this.indent--;
    this.line(')');
  }

  // ─── List Helpers ───

  private emitListHelpers(): void {
    // $nil() -> i64: allocate [tag=0 (4 bytes)]
    this.line('(func $nil (result i64)');
    this.indent++;
    this.line('(local $ptr i32)');
    this.line('(local.set $ptr (call $alloc (i32.const 4)))');
    this.line('(i32.store (local.get $ptr) (i32.const 0))');
    this.line('(i64.extend_i32_u (local.get $ptr))');
    this.indent--;
    this.line(')');

    this.line('');

    // $cons(head: i64, tail: i64) -> i64: allocate [tag=1][head][tail]
    this.line('(func $cons (param $head i64) (param $tail i64) (result i64)');
    this.indent++;
    this.line('(local $ptr i32)');
    this.line('(local.set $ptr (call $alloc (i32.const 20)))');
    this.line('(i32.store (local.get $ptr) (i32.const 1))');
    this.line('(i64.store offset=4 (local.get $ptr) (local.get $head))');
    this.line('(i64.store offset=12 (local.get $ptr) (local.get $tail))');
    this.line('(i64.extend_i32_u (local.get $ptr))');
    this.indent--;
    this.line(')');
  }

  // ─── Lambda Collection and Emission ───

  private collectLambdas(expr: IR.IrExpr): void {
    switch (expr.kind) {
      case 'let':
        if (expr.value.kind === 'lambda') {
          // Register this lambda with a name based on the let binding
          const lamName = `__lambda_${this.lambdaFunctions.length}`;
          this.lambdaFunctions.push({
            name: lamName,
            params: expr.value.params,
            body: expr.value.body,
          });
          this.lambdaNameMap.set(expr.name, lamName);
          this.userFunctions.add(lamName);
          this.collectLambdas(expr.value.body);
        } else {
          this.collectLambdas(expr.value);
        }
        this.collectLambdas(expr.body);
        break;
      case 'lambda': {
        // Anonymous lambda (not bound to let)
        const lamName = `__lambda_${this.lambdaFunctions.length}`;
        this.lambdaFunctions.push({
          name: lamName,
          params: expr.params,
          body: expr.body,
        });
        this.userFunctions.add(lamName);
        this.collectLambdas(expr.body);
        break;
      }
      case 'app':
        this.collectLambdas(expr.fn);
        for (const a of expr.args) this.collectLambdas(a);
        break;
      case 'if':
        this.collectLambdas(expr.cond);
        this.collectLambdas(expr.then);
        this.collectLambdas(expr.else);
        break;
      case 'binop':
        this.collectLambdas(expr.left);
        this.collectLambdas(expr.right);
        break;
      case 'unaryop':
        this.collectLambdas(expr.operand);
        break;
      case 'block':
        for (const e of expr.exprs) this.collectLambdas(e);
        break;
      case 'match':
        this.collectLambdas(expr.scrutinee);
        for (const arm of expr.arms) this.collectLambdas(arm.body);
        break;
      case 'construct':
        for (const a of expr.args) this.collectLambdas(a);
        break;
      case 'concat':
        this.collectLambdas(expr.left);
        this.collectLambdas(expr.right);
        break;
      case 'record':
        for (const [, val] of expr.fields) this.collectLambdas(val);
        break;
      case 'field_access':
        this.collectLambdas(expr.expr);
        break;
      case 'record_update':
        this.collectLambdas(expr.record);
        for (const [, val] of expr.updates) this.collectLambdas(val);
        break;
      case 'list':
        for (const e of expr.elements) this.collectLambdas(e);
        break;
      default:
        break;
    }
  }

  private emitLambdaFunction(lam: { name: string; params: string[]; body: IR.IrExpr }): void {
    // Emit as a regular function
    const savedLocals = this.currentFnLocals;
    const savedDeclared = this.declaredLocals;
    const savedCounter = this.localCounter;

    this.currentFnLocals = new Map();
    this.declaredLocals = [];
    this.localCounter = 0;

    // Register params
    for (const p of lam.params) {
      this.currentFnLocals.set(p, 'i64');
    }

    // Pre-scan body for locals
    this.collectLocals(lam.body);

    const resultType = this.inferExprType(lam.body);
    const params = lam.params.map(p => `(param $${p} i64)`).join(' ');
    const resultStr = resultType === 'void' ? '' : ` (result ${resultType})`;

    this.line(`(func $${lam.name} ${params}${resultStr}`);
    this.indent++;

    for (const localName of this.declaredLocals) {
      const type = this.currentFnLocals.get(localName)!;
      this.line(`(local $${localName} ${type})`);
    }

    const bodyLines = this.emitExpr(lam.body, resultType === 'void');
    for (const l of bodyLines) {
      this.line(l);
    }

    this.indent--;
    this.line(')');

    // Restore
    this.currentFnLocals = savedLocals;
    this.declaredLocals = savedDeclared;
    this.localCounter = savedCounter;
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

import { describe, it, expect } from 'vitest';
import { Lexer } from '../lexer/lexer.js';
import { Parser } from '../parser/parser.js';
import { lowerModule } from '../ir/lower.js';
import { CEmitter } from './emit_c.js';
// ─── Helper: full pipeline from JAPL source to C output ───
function compile(source) {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    const errors = parser.getErrors();
    if (errors.length > 0) {
        throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
    }
    const ir = lowerModule(ast);
    const emitter = new CEmitter();
    return emitter.emit(ir);
}
// Helper to compile directly from IR
function emitIR(module) {
    const emitter = new CEmitter();
    return emitter.emit(module);
}
describe('C Codegen: end-to-end', () => {
    // 1. Hello world with main
    it('compiles hello world with int main() wrapper', () => {
        const out = compile(`fn main() { println("Hello") }`);
        expect(out).toContain('#include "japl_runtime.h"');
        expect(out).toContain('JaplValue japl_fn_main(JaplValue* args, int argc, JaplValue* env, int envc)');
        expect(out).toContain('int main(void)');
        expect(out).toContain('japl_runtime_init()');
        expect(out).toContain('japl_runtime_shutdown()');
        expect(out).toContain('japl_builtin_println');
    });
    // 2. Function with params uses uniform signature
    it('compiles function with params to JaplValue signature', () => {
        const out = compile(`fn add(x, y) { x + y }`);
        expect(out).toContain('JaplValue japl_fn_add(JaplValue* args, int argc, JaplValue* env, int envc)');
        expect(out).toContain('JaplValue x = args[0]');
        expect(out).toContain('JaplValue y = args[1]');
        expect(out).toContain('japl_add(x, y)');
    });
    // 3. Let binding
    it('compiles let binding to JaplValue local', () => {
        const out = compile(`fn foo() { let x = 42; x }`);
        expect(out).toContain('JaplValue x = japl_int(42)');
        expect(out).toContain('return x;');
    });
    // 4. Integer literal
    it('compiles int literal to japl_int', () => {
        const out = compile(`fn num() { 42 }`);
        expect(out).toContain('japl_int(42)');
    });
    // 5. Float literal
    it('compiles float literal to japl_float', () => {
        const out = compile(`fn pi() { 3.14 }`);
        expect(out).toContain('japl_float(3.14)');
    });
    // 6. String literal
    it('compiles string literal to japl_string', () => {
        const out = compile(`fn greet() { "hello" }`);
        expect(out).toContain('japl_string("hello")');
    });
    // 7. Boolean literals
    it('compiles boolean literals to japl_bool', () => {
        const out = compile(`fn yes() { true }`);
        expect(out).toContain('japl_bool(1)');
        const out2 = compile(`fn no() { false }`);
        expect(out2).toContain('japl_bool(0)');
    });
    // 8. Unit value
    it('compiles unit to japl_unit', () => {
        const out = compile(`fn noop() { () }`);
        expect(out).toContain('japl_unit()');
    });
    // 9. Binary operators
    it('compiles binary ops to runtime calls', () => {
        const out = compile(`fn calc(a, b) { a + b }`);
        expect(out).toContain('japl_add(a, b)');
        const out2 = compile(`fn calc(a, b) { a * b }`);
        expect(out2).toContain('japl_mul(a, b)');
        const out3 = compile(`fn calc(a, b) { a == b }`);
        expect(out3).toContain('japl_eq(a, b)');
        const out4 = compile(`fn calc(a, b) { a != b }`);
        expect(out4).toContain('japl_neq(a, b)');
        const out5 = compile(`fn calc(a, b) { a < b }`);
        expect(out5).toContain('japl_lt(a, b)');
        const out6 = compile(`fn calc(a, b) { a > b }`);
        expect(out6).toContain('japl_gt(a, b)');
    });
    // 10. String concat
    it('compiles string concat to japl_string_concat', () => {
        const out = compile(`fn greet() { "hello" <> " world" }`);
        expect(out).toContain('japl_string_concat(');
        expect(out).toContain('japl_string("hello")');
        expect(out).toContain('japl_string(" world")');
    });
    // 11. Constructor (tagged union)
    it('compiles constructor to japl_tagged', () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn wrap(x) { Some(42) }
    `);
        expect(out).toContain('japl_tagged("Some", 1, japl_int(42))');
    });
    // 12. Zero-arg constructor
    it('compiles zero-arg constructor to japl_tagged with 0 fields', () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn empty() { None }
    `);
        expect(out).toContain('japl_tagged("None", 0)');
    });
    // 13. Pattern match with if/strcmp chain
    it('compiles pattern match to if/strcmp chain', () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn unwrap(opt) {
        match opt {
          Some(x) => x,
          None => 0
        }
      }
    `);
        expect(out).toContain('strcmp(japl_get_tag(');
        expect(out).toContain('"Some"');
        expect(out).toContain('japl_get_field(');
        expect(out).toContain('"None"');
    });
    // 14. Record literal
    it('compiles record to japl_record', () => {
        const out = compile(`fn mkUser() { { name: "alice", age: 30 } }`);
        expect(out).toContain('japl_record(2');
        expect(out).toContain('"name"');
        expect(out).toContain('japl_string("alice")');
        expect(out).toContain('"age"');
        expect(out).toContain('japl_int(30)');
    });
    // 15. Field access
    it('compiles field access to japl_field', () => {
        const out = compile(`fn getName(user) { user.name }`);
        expect(out).toContain('japl_field(user, "name")');
    });
    // 16. Record update
    it('compiles record update to japl_record_update', () => {
        const out = compile(`fn birthday(user) { { user | age: 31 } }`);
        expect(out).toContain('japl_record_update(user, "age", japl_int(31))');
    });
    // 17. List literal
    it('compiles list to nested japl_cons', () => {
        const out = compile(`fn nums() { [1, 2, 3] }`);
        expect(out).toContain('japl_cons(japl_int(1),');
        expect(out).toContain('japl_cons(japl_int(2),');
        expect(out).toContain('japl_cons(japl_int(3),');
        expect(out).toContain('japl_nil()');
    });
    // 18. Lambda (lifted static function + closure)
    it('compiles lambda to lifted function and japl_closure', () => {
        const out = compile(`fn mkInc() { fn(x) { x + 1 } }`);
        expect(out).toContain('_lambda_');
        expect(out).toContain('japl_closure(');
        // The lifted function should have the uniform signature
        expect(out).toContain('JaplValue* args, int argc, JaplValue* env, int envc');
        expect(out).toContain('japl_add(x, japl_int(1))');
    });
    // 19. If/else as ternary
    it('compiles if/else using japl_to_bool', () => {
        const out = compile(`fn abs(x) { if x > 0 { x } else { 0 } }`);
        expect(out).toContain('japl_to_bool(');
        expect(out).toContain('japl_gt(x, japl_int(0))');
    });
    // 20. Multiple functions produce forward declarations
    it('emits forward declarations for multiple functions', () => {
        const out = compile(`
      fn add(x, y) { x + y }
      fn sub(x, y) { x - y }
    `);
        expect(out).toContain('/* Forward declarations */');
        expect(out).toContain('JaplValue japl_fn_add(JaplValue* args, int argc, JaplValue* env, int envc);');
        expect(out).toContain('JaplValue japl_fn_sub(JaplValue* args, int argc, JaplValue* env, int envc);');
    });
    // 21. Main function generates int main() wrapper
    it('only generates int main() when JAPL has main fn', () => {
        const out = compile(`fn helper() { 42 }`);
        expect(out).not.toContain('int main(void)');
        expect(out).not.toContain('japl_runtime_init');
    });
    // 22. Unary negation
    it('compiles unary negation to japl_negate', () => {
        const out = compile(`fn neg(x) { -x }`);
        expect(out).toContain('japl_negate(x)');
    });
    // 23. Spawn from IR
    it('compiles spawn to japl_spawn', () => {
        const ir = {
            decls: [{
                    kind: 'fn',
                    name: 'start',
                    params: [],
                    body: { kind: 'spawn', fn: { kind: 'var', name: 'worker' } },
                    exported: false,
                }, {
                    kind: 'fn',
                    name: 'worker',
                    params: [],
                    body: { kind: 'unit' },
                    exported: false,
                }],
        };
        const out = emitIR(ir);
        expect(out).toContain('japl_spawn(&japl_fn_worker, japl_unit())');
    });
    // 24. Send from IR
    it('compiles send to japl_send', () => {
        const ir = {
            decls: [{
                    kind: 'fn',
                    name: 'notify',
                    params: ['pid'],
                    body: {
                        kind: 'send',
                        pid: { kind: 'var', name: 'pid' },
                        msg: { kind: 'string', value: '"hello"' },
                    },
                    exported: false,
                }],
        };
        const out = emitIR(ir);
        expect(out).toContain('japl_send(');
        expect(out).toContain('japl_string("hello")');
    });
    // 25. Includes are always present
    it('always includes japl_runtime.h and standard headers', () => {
        const out = compile(`fn id(x) { x }`);
        expect(out).toContain('#include "japl_runtime.h"');
        expect(out).toContain('#include <stdio.h>');
        expect(out).toContain('#include <stdlib.h>');
        expect(out).toContain('#include <string.h>');
    });
    // 26. Multiple let bindings
    it('compiles multiple let bindings in sequence', () => {
        const out = compile(`fn calc() { let x = 1; let y = 2; x + y }`);
        expect(out).toContain('JaplValue x = japl_int(1)');
        expect(out).toContain('JaplValue y = japl_int(2)');
        expect(out).toContain('japl_add(x, y)');
    });
    // 27. Pipe operator (desugared to application in IR)
    it('compiles pipe operator (desugared to app)', () => {
        const out = compile(`fn pipeline(x) { x |> f }`);
        // x |> f desugars to f(x) in IR lowering
        // f is not a known function, so it uses japl_apply
        expect(out).toContain('japl_apply(f, 1, x)');
    });
    // 28. Known function call uses direct call
    it('calls known functions directly', () => {
        const out = compile(`
      fn double(x) { x + x }
      fn caller() { double(5) }
    `);
        expect(out).toContain('japl_fn_double(');
        expect(out).toContain('japl_int(5)');
    });
    // 29. Lambda with closure captures env vars
    it('compiles closure-capturing lambda with env', () => {
        const ir = {
            decls: [{
                    kind: 'fn',
                    name: 'make_adder',
                    params: ['y'],
                    body: {
                        kind: 'lambda',
                        params: ['x'],
                        body: { kind: 'binop', op: '+', left: { kind: 'var', name: 'x' }, right: { kind: 'var', name: 'y' } },
                    },
                    exported: false,
                }],
        };
        const out = emitIR(ir);
        // The lambda should capture y from env
        expect(out).toContain('env[0]');
        expect(out).toContain('japl_closure(');
        // The closure call should pass y as captured env
        expect(out).toMatch(/japl_closure\(&_lambda_\d+, 1, 1, y\)/);
    });
    // 30. Type declaration is a comment (no C struct needed)
    it('emits type declaration as comment', () => {
        const out = compile(`type Color = | Red | Green | Blue`);
        expect(out).toContain('/* type Color');
    });
    // 31. Subtraction and division ops
    it('compiles subtraction and division', () => {
        const out1 = compile(`fn sub(a, b) { a - b }`);
        expect(out1).toContain('japl_sub(a, b)');
        const out2 = compile(`fn div(a, b) { a / b }`);
        expect(out2).toContain('japl_div(a, b)');
        const out3 = compile(`fn modulo(a, b) { a % b }`);
        expect(out3).toContain('japl_mod(a, b)');
    });
});
//# sourceMappingURL=codegen_c.test.js.map
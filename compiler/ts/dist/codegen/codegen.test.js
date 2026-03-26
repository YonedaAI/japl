import { describe, it, expect } from 'vitest';
import { Lexer } from '../lexer/lexer.js';
import { Parser } from '../parser/parser.js';
import { lowerModule } from '../ir/lower.js';
import { TsEmitter } from './emit.js';
// ─── Helper: full pipeline from JAPL source to TypeScript output ───
function compile(source) {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const ast = parser.parse();
    const errors = parser.getErrors();
    if (errors.length > 0) {
        throw new Error(`Parse errors: ${errors.map(e => e.message).join(", ")}`);
    }
    const ir = lowerModule(ast);
    const emitter = new TsEmitter();
    return emitter.emit(ir);
}
// Normalize whitespace for comparison
function norm(s) {
    return s.trim().replace(/\r\n/g, "\n");
}
describe("Codegen: end-to-end", () => {
    // 1. Hello world
    it("compiles hello world", () => {
        const out = compile(`fn main() { println("Hello") }`);
        expect(out).toContain("function main()");
        expect(out).toContain('println("Hello")');
        // Strings come through with quotes from the lexer
    });
    // 2. Function with params
    it("compiles function with params", () => {
        const out = compile(`fn add(x, y) { x + y }`);
        expect(out).toContain("function add(x, y)");
        expect(out).toContain("x + y");
    });
    // 3. Let binding
    it("compiles let binding in function", () => {
        const out = compile(`fn foo() { let x = 42; x }`);
        expect(out).toContain("const x = 42");
        expect(out).toContain("return x");
    });
    // 4. Sum type declaration
    it("compiles sum type", () => {
        const out = compile(`type Shape = | Circle(Float) | Rectangle(Float, Float)`);
        expect(out).toContain('_tag: "Circle"');
        expect(out).toContain('_tag: "Rectangle"');
        expect(out).toContain("type Shape =");
        // Should have constructor functions
        expect(out).toContain("const Circle =");
        expect(out).toContain("const Rectangle =");
    });
    // 5. Pattern match on constructor
    it("compiles pattern match", () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn unwrap(opt) {
        match opt {
          Some(x) => x,
          None => 0
        }
      }
    `);
        expect(out).toContain("switch (opt._tag)");
        expect(out).toContain('case "Some"');
        expect(out).toContain('case "None"');
        expect(out).toContain("const x = opt._0");
    });
    // 6. If/else (simple => ternary)
    it("compiles simple if/else as ternary", () => {
        const out = compile(`fn abs(x) { if x > 0 { x } else { 0 - x } }`);
        expect(out).toContain("x > 0");
        // Should produce a ternary or if statement
        expect(out).toMatch(/x > 0/);
    });
    // 7. Record literal
    it("compiles record literal", () => {
        const out = compile(`fn mkUser() { { name: "alice", age: 30 } }`);
        // Lexer preserves quotes in string values
        expect(out).toContain("name: \"alice\"");
        expect(out).toContain("age: 30");
    });
    // 8. Record update
    it("compiles record update", () => {
        const out = compile(`fn birthday(user) { { user | age: 31 } }`);
        expect(out).toContain("...user");
        expect(out).toContain("age: 31");
    });
    // 9. List literal
    it("compiles list literal", () => {
        const out = compile(`fn nums() { [1, 2, 3] }`);
        expect(out).toContain("[1, 2, 3]");
    });
    // 10. Lambda
    it("compiles lambda expression", () => {
        const out = compile(`fn mkInc() { fn(x) { x + 1 } }`);
        expect(out).toContain("(x) => x + 1");
    });
    // 11. Pipe operator
    it("compiles pipe operator", () => {
        const out = compile(`fn pipeline(x) { x |> f |> g }`);
        // x |> f → f(x), then |> g → g(f(x))
        expect(out).toContain("g(f(x))");
    });
    // 12. Constructor expression
    it("compiles constructor expression", () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn wrap(x) { Some(42) }
    `);
        expect(out).toContain('_tag: "Some"');
        expect(out).toContain("_0: 42");
    });
    // 13. Spawn (tested via IR directly since parser doesn't yet support spawn as expression)
    it("compiles spawn from IR", () => {
        const emitter = new TsEmitter();
        const ir = {
            decls: [{
                    kind: "fn",
                    name: "start",
                    params: [],
                    body: { kind: "spawn", fn: { kind: "var", name: "worker" } },
                    exported: false,
                }],
        };
        const out = emitter.emit(ir);
        expect(out).toContain("spawn(() => worker())");
        expect(out).toContain('import { spawn } from "@japl/runtime"');
    });
    // 14. String concat
    it("compiles string concat", () => {
        const out = compile(`fn greet() { "hello" <> " world" }`);
        expect(out).toContain('"hello" + " world"');
    });
    // 15. Binary operators
    it("compiles binary operators", () => {
        const out = compile(`fn calc(x, y, z) { x + y * z }`);
        expect(out).toContain("x + y * z");
    });
    // 16. Nested match
    it("compiles nested match", () => {
        const out = compile(`
      type Option = | Some(a) | None
      fn deep(a, b) {
        match a {
          Some(x) => match b {
            Some(y) => x + y,
            None => x
          },
          None => 0
        }
      }
    `);
        expect(out).toContain("switch (a._tag)");
        expect(out).toContain("switch (b._tag)");
    });
    // 17. Block with multiple let bindings
    it("compiles block with let bindings", () => {
        const out = compile(`fn calc() { let x = 1; let y = 2; x + y }`);
        expect(out).toContain("const x = 1");
        expect(out).toContain("const y = 2");
        expect(out).toContain("return x + y");
    });
    // 18. Multiple declarations
    it("compiles multiple declarations", () => {
        const out = compile(`
      fn add(x, y) { x + y }
      fn sub(x, y) { x - y }
    `);
        expect(out).toContain("function add(x, y)");
        expect(out).toContain("function sub(x, y)");
    });
    // 19. Exported function
    it("compiles exported function", () => {
        const out = compile(`pub fn hello() { "hello" }`);
        expect(out).toContain("export function hello()");
    });
    // 20. Test declaration (assert isn't an expression keyword in parser yet, use plain fn call)
    it("compiles test declaration", () => {
        // The parser expects test body to be a block of valid expressions.
        // 'assert' is a keyword but not a recognized expression starter, so use a function call.
        const out = compile(`test "addition works" { 1 + 1 == 2 }`);
        expect(out).toContain("test: addition works");
        expect(out).toContain("function test_");
        expect(out).toContain("1 + 1 === 2");
    });
    // 21. Unit value
    it("compiles unit value", () => {
        const out = compile(`fn noop() { () }`);
        expect(out).toContain("undefined");
    });
    // 22. Boolean literal
    it("compiles boolean literals", () => {
        const out = compile(`fn yes() { true }`);
        expect(out).toContain("true");
    });
    // 23. Float literal
    it("compiles float literal", () => {
        const out = compile(`fn pi() { 3.14 }`);
        expect(out).toContain("3.14");
    });
    // 24. Field access
    it("compiles field access", () => {
        const out = compile(`fn getName(user) { user.name }`);
        expect(out).toContain("user.name");
    });
    // 25. Unary operator
    it("compiles unary operator", () => {
        const out = compile(`fn neg(x) { -x }`);
        expect(out).toContain("-x");
    });
    // 26. Zero-field variant (enum-like)
    it("compiles zero-field variant as constant", () => {
        const out = compile(`type Color = | Red | Green | Blue`);
        expect(out).toContain('const Red: Color = { _tag: "Red" }');
        expect(out).toContain('const Green: Color = { _tag: "Green" }');
        expect(out).toContain('const Blue: Color = { _tag: "Blue" }');
    });
    // 27. Send expression (tested via IR directly since parser doesn't yet support send as expression)
    it("compiles send expression from IR", () => {
        const emitter = new TsEmitter();
        const ir = {
            decls: [{
                    kind: "fn",
                    name: "notify",
                    params: ["pid"],
                    body: {
                        kind: "send",
                        pid: { kind: "var", name: "pid" },
                        msg: { kind: "string", value: '"hello"' },
                    },
                    exported: false,
                }],
        };
        const out = emitter.emit(ir);
        expect(out).toContain('send(pid, "hello")');
        expect(out).toContain('import { send } from "@japl/runtime"');
    });
    // 28. Pipe with partial application
    it("compiles pipe with partial application", () => {
        const out = compile(`fn example(xs) { xs |> map(double) }`);
        // xs |> map(double) → map(xs, double)
        expect(out).toContain("map(xs, double)");
    });
    // 29. No runtime import when not needed
    it("omits runtime imports when not needed", () => {
        const out = compile(`fn add(x, y) { x + y }`);
        expect(out).not.toContain("@japl/runtime");
    });
    // 30. Equality uses strict equality
    it("uses strict equality", () => {
        const out = compile(`fn eq(a, b) { a == b }`);
        expect(out).toContain("a === b");
    });
    // 31. Inequality uses strict inequality
    it("uses strict inequality", () => {
        const out = compile(`fn neq(a, b) { a != b }`);
        expect(out).toContain("a !== b");
    });
});
//# sourceMappingURL=codegen.test.js.map
import { describe, it, expect, beforeEach } from 'vitest';
import { TokenKind } from '../lexer/token.js';
import { Lexer } from '../lexer/lexer.js';
import { Parser } from '../parser/parser.js';
import { TypeChecker } from './infer.js';
import { resetVarCounter, typeToString, INT, FLOAT, BYTE, STRING, BOOL, UNIT } from './types.js';
import { UnificationEngine } from './unify.js';
import { TypeError } from './errors.js';
import { EffectChecker } from './effects.js';
// ─── Helpers ───
const span = { start: 0, end: 0, line: 1, col: 1 };
function tok(kind, value = "") {
    return { kind, value, span };
}
function ident(name) { return tok(TokenKind.Ident, name); }
function upper(name) { return tok(TokenKind.UpperIdent, name); }
function int(value) { return tok(TokenKind.Int, String(value)); }
function float(value) { return tok(TokenKind.Float, String(value)); }
function str(value) { return tok(TokenKind.String, value); }
function eof() { return tok(TokenKind.EOF); }
const FN = tok(TokenKind.Fn, "fn");
const LET = tok(TokenKind.Let, "let");
const TYPE = tok(TokenKind.Type, "type");
const MATCH = tok(TokenKind.Match, "match");
const IF = tok(TokenKind.If, "if");
const ELSE = tok(TokenKind.Else, "else");
const SPAWN = tok(TokenKind.Spawn, "spawn");
const SEND = tok(TokenKind.Send, "send");
const RECEIVE = tok(TokenKind.Receive, "receive");
const TRUE = tok(TokenKind.True, "true");
const FALSE = tok(TokenKind.False, "false");
const RETURN = tok(TokenKind.Return, "return");
const LPAREN = tok(TokenKind.LParen, "(");
const RPAREN = tok(TokenKind.RParen, ")");
const LBRACE = tok(TokenKind.LBrace, "{");
const RBRACE = tok(TokenKind.RBrace, "}");
const LBRACKET = tok(TokenKind.LBracket, "[");
const RBRACKET = tok(TokenKind.RBracket, "]");
const COMMA = tok(TokenKind.Comma, ",");
const COLON = tok(TokenKind.Colon, ":");
const DOT = tok(TokenKind.Dot, ".");
const ASSIGN = tok(TokenKind.Assign, "=");
const ARROW = tok(TokenKind.Arrow, "->");
const FAT_ARROW = tok(TokenKind.FatArrow, "=>");
const BAR = tok(TokenKind.Bar, "|");
const PIPE = tok(TokenKind.Pipe, "|>");
const PLUS = tok(TokenKind.Plus, "+");
const MINUS = tok(TokenKind.Minus, "-");
const STAR = tok(TokenKind.Star, "*");
const SLASH = tok(TokenKind.Slash, "/");
const EQ = tok(TokenKind.Eq, "==");
const LT = tok(TokenKind.Lt, "<");
const GT = tok(TokenKind.Gt, ">");
const AND = tok(TokenKind.And, "&&");
const OR = tok(TokenKind.Or, "||");
const NOT = tok(TokenKind.Not, "!");
const QUESTION = tok(TokenKind.Question, "?");
function parse(tokens) {
    const parser = new Parser([...tokens, eof()]);
    return parser.parse();
}
function parseExpr(tokens) {
    const parser = new Parser([...tokens, eof()]);
    return parser.parseExpr();
}
function checkModule(tokens) {
    const mod = parse(tokens);
    const checker = new TypeChecker();
    const result = checker.checkModule(mod);
    return { checker, result };
}
function inferExprType(tokens) {
    const expr = parseExpr(tokens);
    const checker = new TypeChecker();
    const [type] = checker.inferExpr(expr);
    const resolved = checker.getUnifier().deepResolve(type);
    return { type: resolved, checker, errors: checker.getErrors() };
}
/** Check a module and expect no errors. */
function checkOk(tokens) {
    const r = checkModule(tokens);
    if (r.result.errors.length > 0) {
        throw new Error(`Expected no errors but got:\n${r.result.errors.map(e => e.message).join("\n")}`);
    }
    return r;
}
/** Check a module and expect at least one error. */
function checkErr(tokens) {
    const r = checkModule(tokens);
    expect(r.result.errors.length).toBeGreaterThan(0);
    return r;
}
function checkSource(source) {
    const lexer = new Lexer(source);
    const tokens = lexer.tokenize();
    const parser = new Parser(tokens);
    const mod = parser.parse();
    const checker = new TypeChecker();
    const result = checker.checkModule(mod);
    return { checker, result };
}
// ─── Tests ───
describe("TypeChecker", () => {
    beforeEach(() => {
        resetVarCounter();
    });
    // 1. Literal inference: Int
    it("infers Int literal", () => {
        const { type } = inferExprType([int(42)]);
        expect(type.kind).toBe("int");
    });
    // 2. Literal inference: Float
    it("infers Float literal", () => {
        const { type } = inferExprType([float(3.14)]);
        expect(type.kind).toBe("float");
    });
    // 3. Literal inference: String
    it("infers String literal", () => {
        const { type } = inferExprType([str("hello")]);
        expect(type.kind).toBe("string");
    });
    // 4. Literal inference: Bool (true)
    it("infers Bool literal true", () => {
        const { type } = inferExprType([TRUE]);
        expect(type.kind).toBe("bool");
    });
    // 5. Literal inference: Bool (false)
    it("infers Bool literal false", () => {
        const { type } = inferExprType([FALSE]);
        expect(type.kind).toBe("bool");
    });
    // 6. Variable binding: let x = 42 in x
    it("infers variable binding type", () => {
        // let x = 42 in x  =>  let .. in ..
        // We build: LET ident("x") ASSIGN int(42) IN ident("x")
        // Actually, the parser for let expects: let name = value; body
        // Let's use a fn that has a let binding
        // fn f() -> Int { let x = 42; x }
        const tokens = [
            FN, ident("f"), LPAREN, RPAREN, ARROW, upper("Int"),
            LBRACE, LET, ident("x"), ASSIGN, int(42), tok(TokenKind.Semicolon, ";"), ident("x"), RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 7. Function type: fn add(x: Int, y: Int) -> Int { x + y }
    it("infers function with annotated types", () => {
        const tokens = [
            FN, ident("add"), LPAREN,
            ident("x"), COLON, upper("Int"), COMMA,
            ident("y"), COLON, upper("Int"),
            RPAREN, ARROW, upper("Int"),
            LBRACE, ident("x"), PLUS, ident("y"), RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 8. Type error: return type mismatch
    it("reports error when body type mismatches return annotation", () => {
        // fn f(x: Int) -> Int { "hello" }
        const tokens = [
            FN, ident("f"), LPAREN,
            ident("x"), COLON, upper("Int"),
            RPAREN, ARROW, upper("Int"),
            LBRACE, str("hello"), RBRACE,
        ];
        const { result } = checkErr(tokens);
        expect(result.errors[0].message).toContain("Int");
        expect(result.errors[0].message).toContain("String");
    });
    // 9. Application: add(1, 2) where add is defined
    it("infers application result type", () => {
        // fn add(x: Int, y: Int) -> Int { x + y }
        // fn main() -> Int { add(1, 2) }
        const tokens = [
            FN, ident("add"), LPAREN,
            ident("x"), COLON, upper("Int"), COMMA,
            ident("y"), COLON, upper("Int"),
            RPAREN, ARROW, upper("Int"),
            LBRACE, ident("x"), PLUS, ident("y"), RBRACE,
            FN, ident("main"), LPAREN, RPAREN, ARROW, upper("Int"),
            LBRACE, ident("add"), LPAREN, int(1), COMMA, int(2), RPAREN, RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 10. Wrong arity: add(1)
    it("reports error on wrong argument count", () => {
        const tokens = [
            FN, ident("add"), LPAREN,
            ident("x"), COLON, upper("Int"), COMMA,
            ident("y"), COLON, upper("Int"),
            RPAREN, ARROW, upper("Int"),
            LBRACE, ident("x"), PLUS, ident("y"), RBRACE,
            FN, ident("main"), LPAREN, RPAREN, ARROW, upper("Int"),
            LBRACE, ident("add"), LPAREN, int(1), RPAREN, RBRACE,
        ];
        const { result } = checkErr(tokens);
        expect(result.errors.some(e => e.message.includes("2") && e.message.includes("1"))).toBe(true);
    });
    // 11. Constructor: Some(42)
    it("infers constructor type Some(42)", () => {
        const { type } = inferExprType([upper("Some"), LPAREN, int(42), RPAREN]);
        const resolved = type;
        expect(resolved.kind).toBe("option");
        if (resolved.kind === "option") {
            expect(resolved.some.kind).toBe("int");
        }
    });
    // 12. Constructor: None
    it("infers constructor type None", () => {
        const { type } = inferExprType([upper("None")]);
        expect(type.kind).toBe("option");
    });
    // 13. Constructor wrong arity: Some expects 1 argument
    it("reports error on constructor wrong arity", () => {
        const { errors } = inferExprType([upper("Some"), LPAREN, int(1), COMMA, int(2), RPAREN]);
        expect(errors.some(e => e.message.includes("1") && e.message.includes("2"))).toBe(true);
    });
    // 14. List: [1, 2, 3]
    it("infers list type [1, 2, 3]", () => {
        const { type } = inferExprType([
            LBRACKET, int(1), COMMA, int(2), COMMA, int(3), RBRACKET,
        ]);
        expect(type.kind).toBe("list");
        if (type.kind === "list") {
            expect(type.element.kind).toBe("int");
        }
    });
    // 15. List mixed types: [1, "hello"]
    it("reports error on mixed type list", () => {
        const { errors } = inferExprType([
            LBRACKET, int(1), COMMA, str("hello"), RBRACKET,
        ]);
        expect(errors.length).toBeGreaterThan(0);
    });
    // 16. If/else: if true { 1 } else { 2 }
    it("infers if/else expression type", () => {
        const { type } = inferExprType([
            IF, TRUE, LBRACE, int(1), RBRACE,
            ELSE, LBRACE, int(2), RBRACE,
        ]);
        expect(type.kind).toBe("int");
    });
    // 17. If branches mismatch
    it("reports error on if branch type mismatch", () => {
        const { errors } = inferExprType([
            IF, TRUE, LBRACE, int(1), RBRACE,
            ELSE, LBRACE, str("no"), RBRACE,
        ]);
        expect(errors.length).toBeGreaterThan(0);
    });
    // 18. Record: { name: "alice", age: 30 }
    it("infers record type", () => {
        const { type } = inferExprType([
            LBRACE, ident("name"), COLON, str("alice"), COMMA,
            ident("age"), COLON, int(30), RBRACE,
        ]);
        expect(type.kind).toBe("record");
        if (type.kind === "record") {
            expect(type.fields.get("name")?.kind).toBe("string");
            expect(type.fields.get("age")?.kind).toBe("int");
        }
    });
    // 19. Field access: user.name
    it("infers field access type", () => {
        // fn get_name() -> String { let u = { name: "alice" }; u.name }
        const tokens = [
            FN, ident("get_name"), LPAREN, RPAREN, ARROW, upper("String"),
            LBRACE,
            LET, ident("u"), ASSIGN, LBRACE, ident("name"), COLON, str("alice"), RBRACE,
            tok(TokenKind.Semicolon, ";"),
            ident("u"), DOT, ident("name"),
            RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 20. Binary operations: arithmetic
    it("infers arithmetic binary op types", () => {
        const { type } = inferExprType([int(1), PLUS, int(2)]);
        expect(type.kind).toBe("int");
    });
    // 21. Comparison returns Bool
    it("infers comparison returns Bool", () => {
        const { type } = inferExprType([int(1), LT, int(2)]);
        expect(type.kind).toBe("bool");
    });
    // 22. Logical operators require Bool
    it("infers logical operators", () => {
        const { type } = inferExprType([TRUE, AND, FALSE]);
        expect(type.kind).toBe("bool");
    });
    // 23. Unary negation
    it("infers unary minus on Int", () => {
        const { type } = inferExprType([MINUS, int(5)]);
        expect(type.kind).toBe("int");
    });
    // 24. Unary not
    it("infers unary not on Bool", () => {
        const { type } = inferExprType([NOT, TRUE]);
        expect(type.kind).toBe("bool");
    });
    // 25. Empty list
    it("infers empty list with fresh element type", () => {
        const { type } = inferExprType([LBRACKET, RBRACKET]);
        expect(type.kind).toBe("list");
    });
    // 26. Pattern matching on Option
    it("checks pattern matching on constructors", () => {
        // fn unwrap(opt: Option[Int]) -> Int { match opt { Some(x) => x, None => 0 } }
        // Need to represent Option[Int] in the token stream
        const tokens = [
            FN, ident("unwrap"), LPAREN,
            ident("opt"), COLON, upper("Option"), LBRACKET, upper("Int"), RBRACKET,
            RPAREN, ARROW, upper("Int"),
            LBRACE,
            MATCH, ident("opt"), LBRACE,
            upper("Some"), LPAREN, ident("x"), RPAREN, FAT_ARROW, ident("x"), COMMA,
            upper("None"), FAT_ARROW, int(0),
            RBRACE,
            RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 27. Recursive function: fn fib(n: Int) -> Int { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
    it("handles recursive functions", () => {
        const tokens = [
            FN, ident("fib"), LPAREN, ident("n"), COLON, upper("Int"), RPAREN, ARROW, upper("Int"),
            LBRACE,
            IF, ident("n"), LT, int(2),
            LBRACE, ident("n"), RBRACE,
            ELSE, LBRACE,
            ident("fib"), LPAREN, ident("n"), MINUS, int(1), RPAREN,
            PLUS,
            ident("fib"), LPAREN, ident("n"), MINUS, int(2), RPAREN,
            RBRACE,
            RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 28. ADT definition: type Shape = | Circle(Float) | Rect(Float, Float)
    it("registers ADT constructors", () => {
        const tokens = [
            TYPE, upper("Shape"), ASSIGN,
            BAR, upper("Circle"), LPAREN, upper("Float"), RPAREN,
            BAR, upper("Rect"), LPAREN, upper("Float"), COMMA, upper("Float"), RPAREN,
        ];
        const { checker, result } = checkOk(tokens);
        const env = checker.getEnv();
        expect(env.lookupConstructor("Circle")).toBeDefined();
        expect(env.lookupConstructor("Rect")).toBeDefined();
    });
    // 29. ADT constructor use
    it("type checks ADT constructor application", () => {
        const tokens = [
            TYPE, upper("Shape"), ASSIGN,
            BAR, upper("Circle"), LPAREN, upper("Float"), RPAREN,
            BAR, upper("Rect"), LPAREN, upper("Float"), COMMA, upper("Float"), RPAREN,
            FN, ident("make_circle"), LPAREN, ident("r"), COLON, upper("Float"), RPAREN,
            ARROW, upper("Shape"),
            LBRACE, upper("Circle"), LPAREN, ident("r"), RPAREN, RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 30. Lambda: fn(x) { x + 1 }
    it("infers lambda type", () => {
        const { type } = inferExprType([
            FN, LPAREN, ident("x"), RPAREN, LBRACE, ident("x"), PLUS, int(1), RBRACE,
        ]);
        expect(type.kind).toBe("fn");
        if (type.kind === "fn") {
            expect(type.params.length).toBe(1);
        }
    });
    // 31. Pipe: 42 |> f where f: fn(Int) -> Int
    it("infers pipe expression type", () => {
        // fn double(x: Int) -> Int { x + x }
        // fn main() -> Int { 42 |> double }
        const tokens = [
            FN, ident("double"), LPAREN, ident("x"), COLON, upper("Int"), RPAREN,
            ARROW, upper("Int"),
            LBRACE, ident("x"), PLUS, ident("x"), RBRACE,
            FN, ident("main"), LPAREN, RPAREN, ARROW, upper("Int"),
            LBRACE, int(42), PIPE, ident("double"), RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 32. Ok/Err constructors
    it("infers Ok and Err constructors", () => {
        const { type: okType } = inferExprType([upper("Ok"), LPAREN, int(42), RPAREN]);
        expect(okType.kind).toBe("result");
        if (okType.kind === "result") {
            expect(okType.ok.kind).toBe("int");
        }
        const { type: errType } = inferExprType([upper("Err"), LPAREN, str("fail"), RPAREN]);
        expect(errType.kind).toBe("result");
        if (errType.kind === "result") {
            expect(errType.err.kind).toBe("string");
        }
    });
    // 33. Try expression: expr?
    it("infers try expression unwraps Result", () => {
        // Ok(42)? should unwrap to Int
        const { type, errors } = inferExprType([
            upper("Ok"), LPAREN, int(42), RPAREN, QUESTION,
        ]);
        expect(errors).toHaveLength(0);
        expect(type.kind).toBe("int");
    });
    // 34. Multiple uses of a polymorphic function (generalization)
    it("supports polymorphic generalization", () => {
        // fn id(x: a) -> a { x }   -- but we write it without annotation for inference
        // Actually, let's define with annotation using a type var
        // fn first(x: Int) -> Int { id(x) }
        // fn second(x: String) -> String { id(x) }
        // This is simpler: define id with fresh type param, use at Int and String
        const tokens = [
            FN, ident("id"), LPAREN, ident("x"), RPAREN,
            LBRACE, ident("x"), RBRACE,
            FN, ident("test_poly"), LPAREN, RPAREN, ARROW, upper("Int"),
            LBRACE,
            LET, ident("a"), ASSIGN, ident("id"), LPAREN, int(42), RPAREN,
            tok(TokenKind.Semicolon, ";"),
            LET, ident("b"), ASSIGN, ident("id"), LPAREN, str("hello"), RPAREN,
            tok(TokenKind.Semicolon, ";"),
            ident("a"),
            RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
    // 35. Undefined variable
    it("reports error on undefined variable", () => {
        const { errors } = inferExprType([ident("nonexistent")]);
        expect(errors.some(e => e.message.includes("Undefined variable"))).toBe(true);
    });
    // 36. Float arithmetic
    it("infers Float arithmetic", () => {
        const { type } = inferExprType([float(1.5), PLUS, float(2.5)]);
        expect(type.kind).toBe("float");
    });
    // 37. Unit literal
    it("infers Unit literal", () => {
        const { type } = inferExprType([LPAREN, RPAREN]);
        expect(type.kind).toBe("unit");
    });
    // 38. Block expression returns last value
    it("infers block expression type as last expr", () => {
        const { type } = inferExprType([
            LBRACE, int(1), tok(TokenKind.Semicolon, ";"), int(2), tok(TokenKind.Semicolon, ";"), str("result"), RBRACE,
        ]);
        expect(type.kind).toBe("string");
    });
    // 39. Equality comparison
    it("infers equality comparison returns Bool", () => {
        const { type } = inferExprType([int(1), EQ, int(2)]);
        expect(type.kind).toBe("bool");
    });
    // 40. Multiple function definitions: second can call first
    it("allows later functions to call earlier functions", () => {
        const tokens = [
            FN, ident("square"), LPAREN, ident("x"), COLON, upper("Int"), RPAREN, ARROW, upper("Int"),
            LBRACE, ident("x"), STAR, ident("x"), RBRACE,
            FN, ident("cube"), LPAREN, ident("x"), COLON, upper("Int"), RPAREN, ARROW, upper("Int"),
            LBRACE, ident("square"), LPAREN, ident("x"), RPAREN, STAR, ident("x"), RBRACE,
        ];
        const { result } = checkOk(tokens);
        expect(result.errors).toHaveLength(0);
    });
});
describe("UnificationEngine", () => {
    beforeEach(() => {
        resetVarCounter();
    });
    // 41. Unify identical primitives
    it("unifies identical primitive types", () => {
        const engine = new UnificationEngine();
        expect(() => engine.unify(INT, INT, span)).not.toThrow();
        expect(() => engine.unify(STRING, STRING, span)).not.toThrow();
        expect(() => engine.unify(BOOL, BOOL, span)).not.toThrow();
    });
    // 42. Unify var with concrete
    it("unifies variable with concrete type", () => {
        const engine = new UnificationEngine();
        const v = { kind: "var", id: 0 };
        engine.unify(v, INT, span);
        expect(engine.resolve(v).kind).toBe("int");
    });
    // 43. Fail to unify different concrete types
    it("fails to unify Int with String", () => {
        const engine = new UnificationEngine();
        expect(() => engine.unify(INT, STRING, span)).toThrow(TypeError);
    });
    // 44. Occurs check
    it("detects infinite type via occurs check", () => {
        const engine = new UnificationEngine();
        const v = { kind: "var", id: 0 };
        const listV = { kind: "list", element: v };
        expect(() => engine.unify(v, listV, span)).toThrow();
    });
    // 45. Unify function types
    it("unifies function types with matching structure", () => {
        const engine = new UnificationEngine();
        const fn1 = {
            kind: "fn",
            params: [INT],
            ret: INT,
            effects: { effects: new Set(), open: false },
        };
        const fn2 = {
            kind: "fn",
            params: [INT],
            ret: INT,
            effects: { effects: new Set(), open: false },
        };
        expect(() => engine.unify(fn1, fn2, span)).not.toThrow();
    });
    // 46. Fail to unify functions with different param counts
    it("fails to unify functions with different param counts", () => {
        const engine = new UnificationEngine();
        const fn1 = {
            kind: "fn",
            params: [INT, INT],
            ret: INT,
            effects: { effects: new Set(), open: false },
        };
        const fn2 = {
            kind: "fn",
            params: [INT],
            ret: INT,
            effects: { effects: new Set(), open: false },
        };
        expect(() => engine.unify(fn1, fn2, span)).toThrow(TypeError);
    });
    // 47. Deep resolve
    it("deeply resolves chained variables", () => {
        const engine = new UnificationEngine();
        const v1 = { kind: "var", id: 0 };
        const v2 = { kind: "var", id: 1 };
        engine.unify(v1, v2, span);
        engine.unify(v2, INT, span);
        expect(engine.deepResolve(v1).kind).toBe("int");
    });
    // 48. Unify list types
    it("unifies list types", () => {
        const engine = new UnificationEngine();
        const list1 = { kind: "list", element: INT };
        const list2 = { kind: "list", element: INT };
        expect(() => engine.unify(list1, list2, span)).not.toThrow();
    });
    // 49. Fail to unify list with different element types
    it("fails to unify List[Int] with List[String]", () => {
        const engine = new UnificationEngine();
        const list1 = { kind: "list", element: INT };
        const list2 = { kind: "list", element: STRING };
        expect(() => engine.unify(list1, list2, span)).toThrow(TypeError);
    });
});
describe("EffectChecker", () => {
    beforeEach(() => {
        resetVarCounter();
    });
    // 50. Pure function with IO is an error (via module check)
    it("reports error when pure function has IO effect", () => {
        const checker = new EffectChecker();
        const declared = { effects: new Set(), open: false }; // pure
        const actual = { effects: new Set(["io"]), open: false };
        const err = checker.checkPurity(declared, actual, span);
        expect(err).not.toBeNull();
        expect(err.message).toContain("pure");
    });
});
describe("Type pretty printing", () => {
    // 51. typeToString
    it("pretty prints types correctly", () => {
        expect(typeToString(INT)).toBe("Int");
        expect(typeToString(FLOAT)).toBe("Float");
        expect(typeToString(BYTE)).toBe("Byte");
        expect(typeToString(STRING)).toBe("String");
        expect(typeToString(BOOL)).toBe("Bool");
        expect(typeToString(UNIT)).toBe("Unit");
        expect(typeToString({ kind: "list", element: INT })).toBe("List[Int]");
        expect(typeToString({ kind: "option", some: STRING })).toBe("Option[String]");
        expect(typeToString({ kind: "result", ok: INT, err: STRING })).toBe("Result[Int, String]");
        expect(typeToString({
            kind: "fn",
            params: [INT, INT],
            ret: INT,
            effects: { effects: new Set(), open: false },
        })).toBe("fn(Int, Int) -> Int");
    });
});
// ─── Gap 2: No implicit Int→Float promotion ───
describe("Strict numeric types (no implicit promotion)", () => {
    beforeEach(() => {
        resetVarCounter();
    });
    it("rejects Int + Float (no implicit promotion)", () => {
        const r = checkSource('fn f(x: Int, y: Float) -> Float { x + y }');
        expect(r.result.errors.length).toBeGreaterThan(0);
        expect(r.result.errors[0].message).toContain('Cannot mix');
    });
    it("allows Int + Int", () => {
        const r = checkSource('fn f(x: Int, y: Int) -> Int { x + y }');
        expect(r.result.errors).toHaveLength(0);
    });
    it("allows Float + Float", () => {
        const r = checkSource('fn f(x: Float, y: Float) -> Float { x + y }');
        expect(r.result.errors).toHaveLength(0);
    });
    it("rejects Float + Int (no implicit promotion)", () => {
        const r = checkSource('fn f(x: Float, y: Int) -> Float { x + y }');
        expect(r.result.errors.length).toBeGreaterThan(0);
        expect(r.result.errors[0].message).toContain('Cannot mix');
    });
});
// ─── Gap 4: Byte type ───
describe("Byte type", () => {
    beforeEach(() => {
        resetVarCounter();
    });
    it("type checks Byte arithmetic", () => {
        const r = checkSource('fn f(x: Byte, y: Byte) -> Byte { x + y }');
        expect(r.result.errors).toHaveLength(0);
    });
    it("rejects Byte + Int", () => {
        const r = checkSource('fn f(x: Byte, y: Int) -> Int { x + y }');
        expect(r.result.errors.length).toBeGreaterThan(0);
        expect(r.result.errors[0].message).toContain('Cannot mix');
    });
    it("rejects Int + Byte", () => {
        const r = checkSource('fn f(x: Int, y: Byte) -> Int { x + y }');
        expect(r.result.errors.length).toBeGreaterThan(0);
        expect(r.result.errors[0].message).toContain('Cannot mix');
    });
    it("rejects Byte + Float", () => {
        const r = checkSource('fn f(x: Byte, y: Float) -> Float { x + y }');
        expect(r.result.errors.length).toBeGreaterThan(0);
        expect(r.result.errors[0].message).toContain('Cannot mix');
    });
});
//# sourceMappingURL=checker.test.js.map
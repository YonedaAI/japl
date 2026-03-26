import { describe, it, expect } from 'vitest';
import { Token, TokenKind, Span } from '../lexer/token.js';
import { Parser, ParseError } from './parser.js';
import * as AST from './ast.js';

// ─── Token builder helpers ───

const span: Span = { start: 0, end: 0, line: 1, col: 1 };

function tok(kind: TokenKind, value: string = ""): Token {
  return { kind, value, span };
}

function ident(name: string): Token {
  return tok(TokenKind.Ident, name);
}

function upper(name: string): Token {
  return tok(TokenKind.UpperIdent, name);
}

function int(value: number): Token {
  return tok(TokenKind.Int, String(value));
}

function float(value: number): Token {
  return tok(TokenKind.Float, String(value));
}

function str(value: string): Token {
  return tok(TokenKind.String, value);
}

function eof(): Token {
  return tok(TokenKind.EOF);
}

// Shorthand keyword/operator tokens
const FN = tok(TokenKind.Fn, "fn");
const LET = tok(TokenKind.Let, "let");
const TYPE = tok(TokenKind.Type, "type");
const MATCH = tok(TokenKind.Match, "match");
const IF = tok(TokenKind.If, "if");
const ELSE = tok(TokenKind.Else, "else");
const IMPORT = tok(TokenKind.Import, "import");
const TEST = tok(TokenKind.Test, "test");
const RECEIVE = tok(TokenKind.Receive, "receive");
const TRAIT = tok(TokenKind.Trait, "trait");
const IMPL = tok(TokenKind.Impl, "impl");
const PUB = tok(TokenKind.Pub, "pub");
const FOREIGN = tok(TokenKind.Foreign, "foreign");
const RETURN = tok(TokenKind.Return, "return");
const TRUE = tok(TokenKind.True, "true");
const FALSE = tok(TokenKind.False, "false");

const LPAREN = tok(TokenKind.LParen, "(");
const RPAREN = tok(TokenKind.RParen, ")");
const LBRACE = tok(TokenKind.LBrace, "{");
const RBRACE = tok(TokenKind.RBrace, "}");
const LBRACKET = tok(TokenKind.LBracket, "[");
const RBRACKET = tok(TokenKind.RBracket, "]");
const COMMA = tok(TokenKind.Comma, ",");
const COLON = tok(TokenKind.Colon, ":");
const SEMI = tok(TokenKind.Semicolon, ";");
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
const NEQ = tok(TokenKind.NotEq, "!=");
const LT = tok(TokenKind.Lt, "<");
const GT = tok(TokenKind.Gt, ">");
const LTEQ = tok(TokenKind.LtEq, "<=");
const GTEQ = tok(TokenKind.GtEq, ">=");
const AND = tok(TokenKind.And, "&&");
const OR = tok(TokenKind.Or, "||");
const NOT = tok(TokenKind.Not, "!");
const QUESTION = tok(TokenKind.Question, "?");
const CONCAT = tok(TokenKind.Concat, "<>");
const PERCENT = tok(TokenKind.Percent, "%");

function parse(tokens: Token[]): AST.Module {
  const parser = new Parser([...tokens, eof()]);
  return parser.parse();
}

function parseExpr(tokens: Token[]): AST.Expr {
  // Wrap in a function to parse as an expression
  const parser = new Parser([...tokens, eof()]);
  return parser.parseExpr();
}

function parseType(tokens: Token[]): AST.TypeExpr {
  const parser = new Parser([...tokens, eof()]);
  return parser.parseTypeExpr();
}

// ─── Tests ───

describe("Parser", () => {

  // 1. Simple function declaration
  it("parses simple function: fn add(x: Int, y: Int) -> Int { x + y }", () => {
    const tokens = [
      FN, ident("add"), LPAREN,
      ident("x"), COLON, upper("Int"), COMMA,
      ident("y"), COLON, upper("Int"),
      RPAREN, ARROW, upper("Int"),
      LBRACE, ident("x"), PLUS, ident("y"), RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const fn = mod.decls[0];
    expect(fn.kind).toBe("fn");
    if (fn.kind === "fn") {
      expect(fn.name).toBe("add");
      expect(fn.params).toHaveLength(2);
      expect(fn.params[0].name).toBe("x");
      expect(fn.params[1].name).toBe("y");
      expect(fn.pub).toBe(false);
      expect(fn.body.kind).toBe("binop");
    }
  });

  // 2. Type declaration (sum type)
  it("parses type declaration: type Option(a) = | Some(a) | None", () => {
    const tokens = [
      TYPE, upper("Option"), LPAREN, ident("a"), RPAREN, ASSIGN,
      BAR, upper("Some"), LPAREN, ident("a"), RPAREN,
      BAR, upper("None"),
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("type");
    if (decl.kind === "type") {
      expect(decl.name).toBe("Option");
      expect(decl.typeParams).toEqual(["a"]);
      expect(decl.variants).toHaveLength(2);
      expect(decl.variants[0].name).toBe("Some");
      expect(decl.variants[0].fields).toHaveLength(1);
      expect(decl.variants[1].name).toBe("None");
      expect(decl.variants[1].fields).toHaveLength(0);
    }
  });

  // 3. Record type
  it("parses record type: type User = { name: String, age: Int }", () => {
    const tokens = [
      TYPE, upper("User"), ASSIGN,
      LBRACE,
      ident("name"), COLON, upper("String"), COMMA,
      ident("age"), COLON, upper("Int"),
      RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("record_type");
    if (decl.kind === "record_type") {
      expect(decl.name).toBe("User");
      expect(decl.fields).toHaveLength(2);
      expect(decl.fields[0].name).toBe("name");
      expect(decl.fields[1].name).toBe("age");
    }
  });

  // 4. Let expression
  it("parses let expression: let x = 42", () => {
    const tokens = [LET, ident("x"), ASSIGN, int(42)];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("let");
    if (expr.kind === "let") {
      expect(expr.name).toBe("x");
      expect(expr.value.kind).toBe("int");
      if (expr.value.kind === "int") expect(expr.value.value).toBe(42);
    }
  });

  // 5. Match expression
  it("parses match expression: match x { Some(v) => v, None => 0 }", () => {
    const tokens = [
      MATCH, ident("x"), LBRACE,
      upper("Some"), LPAREN, ident("v"), RPAREN, FAT_ARROW, ident("v"), COMMA,
      upper("None"), FAT_ARROW, int(0),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("match");
    if (expr.kind === "match") {
      expect(expr.arms).toHaveLength(2);
      expect(expr.arms[0].pattern.kind).toBe("pconstructor");
      expect(expr.arms[1].pattern.kind).toBe("pconstructor");
    }
  });

  // 6. If expression
  it("parses if expression: if x > 0 { x } else { -x }", () => {
    const tokens = [
      IF, ident("x"), GT, int(0),
      LBRACE, ident("x"), RBRACE,
      ELSE, LBRACE, MINUS, ident("x"), RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("if");
    if (expr.kind === "if") {
      expect(expr.condition.kind).toBe("binop");
      expect(expr.then.kind).toBe("var");
      expect(expr.else).toBeDefined();
      expect(expr.else?.kind).toBe("unaryop");
    }
  });

  // 7. Lambda
  it("parses lambda: fn(x) { x + 1 }", () => {
    const tokens = [
      FN, LPAREN, ident("x"), RPAREN,
      LBRACE, ident("x"), PLUS, int(1), RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("lambda");
    if (expr.kind === "lambda") {
      expect(expr.params).toHaveLength(1);
      expect(expr.params[0].name).toBe("x");
      expect(expr.body.kind).toBe("binop");
    }
  });

  // 8. Pipe chain: list |> map(f) |> filter(g)
  it("parses pipe chain: list |> map(f) |> filter(g)", () => {
    const tokens = [
      ident("list"), PIPE,
      ident("map"), LPAREN, ident("f"), RPAREN, PIPE,
      ident("filter"), LPAREN, ident("g"), RPAREN,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("pipe");
    if (expr.kind === "pipe") {
      // Left-assoc: ((list |> map(f)) |> filter(g))
      expect(expr.left.kind).toBe("pipe");
      expect(expr.right.kind).toBe("app");
    }
  });

  // 9. Record literal
  it('parses record literal: { name: "alice", age: 30 }', () => {
    const tokens = [
      LBRACE,
      ident("name"), COLON, str("alice"), COMMA,
      ident("age"), COLON, int(30),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("record");
    if (expr.kind === "record") {
      expect(expr.fields).toHaveLength(2);
      expect(expr.fields[0][0]).toBe("name");
      expect(expr.fields[1][0]).toBe("age");
    }
  });

  // 10. Record update
  it("parses record update: { user | age: 31 }", () => {
    const tokens = [
      LBRACE, ident("user"), BAR,
      ident("age"), COLON, int(31),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("record_update");
    if (expr.kind === "record_update") {
      expect(expr.record.kind).toBe("var");
      expect(expr.fields).toHaveLength(1);
      expect(expr.fields[0][0]).toBe("age");
    }
  });

  // 11. List literal
  it("parses list literal: [1, 2, 3]", () => {
    const tokens = [LBRACKET, int(1), COMMA, int(2), COMMA, int(3), RBRACKET];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("list");
    if (expr.kind === "list") {
      expect(expr.elements).toHaveLength(3);
    }
  });

  // 12. Pattern matching with constructors
  it("parses match with constructor pattern: match msg { Tick(id) => handle(id) }", () => {
    const tokens = [
      MATCH, ident("msg"), LBRACE,
      upper("Tick"), LPAREN, ident("id"), RPAREN, FAT_ARROW,
      ident("handle"), LPAREN, ident("id"), RPAREN,
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("match");
    if (expr.kind === "match") {
      expect(expr.arms).toHaveLength(1);
      const arm = expr.arms[0];
      expect(arm.pattern.kind).toBe("pconstructor");
      if (arm.pattern.kind === "pconstructor") {
        expect(arm.pattern.name).toBe("Tick");
        expect(arm.pattern.args).toHaveLength(1);
      }
      expect(arm.body.kind).toBe("app");
    }
  });

  // 13. Import
  it("parses import: import List.{map, filter}", () => {
    const tokens = [
      IMPORT, upper("List"), DOT,
      LBRACE, ident("map"), COMMA, ident("filter"), RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("import");
    if (decl.kind === "import") {
      expect(decl.path).toEqual(["List"]);
      expect(decl.items).toEqual(["map", "filter"]);
    }
  });

  // 14. Test block
  it('parses test block: test "adds numbers" { assert add(1, 2) == 3 }', () => {
    // test "adds numbers" { add(1, 2) == 3 }
    const tokens = [
      TEST, str("adds numbers"), LBRACE,
      ident("add"), LPAREN, int(1), COMMA, int(2), RPAREN, EQ, int(3),
      RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("test");
    if (decl.kind === "test") {
      expect(decl.name).toBe("adds numbers");
    }
  });

  // 15. Nested expressions: f(g(x), h(y))
  it("parses nested expressions: f(g(x), h(y))", () => {
    const tokens = [
      ident("f"), LPAREN,
      ident("g"), LPAREN, ident("x"), RPAREN, COMMA,
      ident("h"), LPAREN, ident("y"), RPAREN,
      RPAREN,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("app");
    if (expr.kind === "app") {
      expect(expr.args).toHaveLength(2);
      expect(expr.args[0].kind).toBe("app");
      expect(expr.args[1].kind).toBe("app");
    }
  });

  // 16. Operator precedence: 1 + 2 * 3 => 1 + (2 * 3)
  it("respects operator precedence: 1 + 2 * 3", () => {
    const tokens = [int(1), PLUS, int(2), STAR, int(3)];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("binop");
    if (expr.kind === "binop") {
      expect(expr.op).toBe("+");
      expect(expr.left.kind).toBe("int");
      expect(expr.right.kind).toBe("binop");
      if (expr.right.kind === "binop") {
        expect(expr.right.op).toBe("*");
      }
    }
  });

  // 17. Try operator: get_user(id)?
  it("parses try operator: get_user(id)?", () => {
    const tokens = [
      ident("get_user"), LPAREN, ident("id"), RPAREN, QUESTION,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("try");
    if (expr.kind === "try") {
      expect(expr.expr.kind).toBe("app");
    }
  });

  // 18. Receive expression
  it("parses receive: receive { Msg(x) => x }", () => {
    const tokens = [
      RECEIVE, LBRACE,
      upper("Msg"), LPAREN, ident("x"), RPAREN, FAT_ARROW, ident("x"),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("receive");
    if (expr.kind === "receive") {
      expect(expr.arms).toHaveLength(1);
    }
  });

  // 19. Spawn: spawn(worker_loop(init)) - parsed as function call
  it("parses spawn as function call: spawn(worker_loop(init))", () => {
    // spawn is a keyword but we parse spawn(expr) like calling a function named spawn
    // Actually spawn is TokenKind.Spawn keyword.
    // We'll treat it as: the user writes spawn(...) which the lexer gives as Ident("spawn")
    // But since spawn is a keyword, let's ensure the parser handles it.
    // For now, spawn as a regular call: we use ident("spawn")
    const tokens = [
      ident("spawn"), LPAREN,
      ident("worker_loop"), LPAREN, ident("init"), RPAREN,
      RPAREN,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("app");
    if (expr.kind === "app") {
      expect((expr.fn as AST.Expr & { kind: "var" }).name).toBe("spawn");
      expect(expr.args).toHaveLength(1);
      expect(expr.args[0].kind).toBe("app");
    }
  });

  // 20. Block with multiple expressions
  it("parses block with multiple expressions", () => {
    const tokens = [
      LBRACE,
      ident("x"), SEMI,
      ident("y"), SEMI,
      ident("z"),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("block");
    if (expr.kind === "block") {
      expect(expr.exprs).toHaveLength(3);
    }
  });

  // 21. Public function
  it("parses pub fn declaration", () => {
    const tokens = [
      PUB, FN, ident("greet"), LPAREN, RPAREN, ARROW, upper("String"),
      LBRACE, str("hello"), RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("fn");
    if (decl.kind === "fn") {
      expect(decl.pub).toBe(true);
      expect(decl.name).toBe("greet");
    }
  });

  // 22. Field access
  it("parses field access: user.name", () => {
    const tokens = [ident("user"), DOT, ident("name")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("field_access");
    if (expr.kind === "field_access") {
      expect(expr.field).toBe("name");
      expect(expr.expr.kind).toBe("var");
    }
  });

  // 23. Chained field access: a.b.c
  it("parses chained field access: a.b.c", () => {
    const tokens = [ident("a"), DOT, ident("b"), DOT, ident("c")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("field_access");
    if (expr.kind === "field_access") {
      expect(expr.field).toBe("c");
      expect(expr.expr.kind).toBe("field_access");
    }
  });

  // 24. Boolean operators
  it("parses boolean operators: a && b || c", () => {
    const tokens = [ident("a"), AND, ident("b"), OR, ident("c")];
    const expr = parseExpr(tokens);
    // || has lower precedence than &&, so: (a && b) || c
    expect(expr.kind).toBe("binop");
    if (expr.kind === "binop") {
      expect(expr.op).toBe("||");
      expect(expr.left.kind).toBe("binop");
      if (expr.left.kind === "binop") {
        expect(expr.left.op).toBe("&&");
      }
    }
  });

  // 25. Unary negation
  it("parses unary negation: -x", () => {
    const tokens = [MINUS, ident("x")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("unaryop");
    if (expr.kind === "unaryop") {
      expect(expr.op).toBe("-");
      expect(expr.operand.kind).toBe("var");
    }
  });

  // 26. Empty list
  it("parses empty list: []", () => {
    const tokens = [LBRACKET, RBRACKET];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("list");
    if (expr.kind === "list") {
      expect(expr.elements).toHaveLength(0);
    }
  });

  // 27. Unit expression
  it("parses unit: ()", () => {
    const tokens = [LPAREN, RPAREN];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("unit");
  });

  // 28. Constructor with no args
  it("parses nullary constructor: None", () => {
    const tokens = [upper("None")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("constructor");
    if (expr.kind === "constructor") {
      expect(expr.name).toBe("None");
      expect(expr.args).toHaveLength(0);
    }
  });

  // 29. Constructor with args: Some(42)
  it("parses constructor with args: Some(42)", () => {
    const tokens = [upper("Some"), LPAREN, int(42), RPAREN];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("constructor");
    if (expr.kind === "constructor") {
      expect(expr.name).toBe("Some");
      expect(expr.args).toHaveLength(1);
    }
  });

  // 30. Foreign function
  it("parses foreign fn declaration", () => {
    const tokens = [
      FOREIGN, FN, ident("console_log"), LPAREN,
      ident("msg"), COLON, upper("String"),
      RPAREN, ARROW, upper("Unit"),
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("foreign");
    if (decl.kind === "foreign") {
      expect(decl.name).toBe("console_log");
      expect(decl.params).toHaveLength(1);
    }
  });

  // 31. Type expression: fn(Int, Int) -> Int
  it("parses function type expression", () => {
    const tokens = [FN, LPAREN, upper("Int"), COMMA, upper("Int"), RPAREN, ARROW, upper("Int")];
    const ty = parseType(tokens);
    expect(ty.kind).toBe("tfn");
    if (ty.kind === "tfn") {
      expect(ty.params).toHaveLength(2);
      expect(ty.ret.kind).toBe("tnamed");
    }
  });

  // 32. Comparison operators
  it("parses comparison: x <= 10", () => {
    const tokens = [ident("x"), LTEQ, int(10)];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("binop");
    if (expr.kind === "binop") {
      expect(expr.op).toBe("<=");
    }
  });

  // 33. Let with type annotation
  it("parses let with type annotation: let x: Int = 42", () => {
    const tokens = [LET, ident("x"), COLON, upper("Int"), ASSIGN, int(42)];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("let");
    if (expr.kind === "let") {
      expect(expr.name).toBe("x");
      expect(expr.type).toBeDefined();
      expect(expr.type?.kind).toBe("tnamed");
    }
  });

  // 34. Wildcard pattern
  it("parses wildcard pattern in match", () => {
    const tokens = [
      MATCH, ident("x"), LBRACE,
      ident("_"), FAT_ARROW, int(0),
      RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("match");
    if (expr.kind === "match") {
      expect(expr.arms[0].pattern.kind).toBe("pwildcard");
    }
  });

  // 35. If without else
  it("parses if without else", () => {
    const tokens = [
      IF, TRUE, LBRACE, int(1), RBRACE,
    ];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("if");
    if (expr.kind === "if") {
      expect(expr.else).toBeUndefined();
    }
  });

  // 36. Multiple declarations
  it("parses multiple top-level declarations", () => {
    const tokens = [
      FN, ident("a"), LPAREN, RPAREN, LBRACE, int(1), RBRACE,
      FN, ident("b"), LPAREN, RPAREN, LBRACE, int(2), RBRACE,
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(2);
  });

  // 37. Concat operator
  it("parses concat operator: a <> b", () => {
    const tokens = [ident("a"), CONCAT, ident("b")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("binop");
    if (expr.kind === "binop") {
      expect(expr.op).toBe("<>");
    }
  });

  // 38. Function with = body (not block)
  it("parses function with = body: fn id(x) = x", () => {
    const tokens = [
      FN, ident("id"), LPAREN, ident("x"), RPAREN, ASSIGN, ident("x"),
    ];
    const mod = parse(tokens);
    expect(mod.decls).toHaveLength(1);
    const decl = mod.decls[0];
    expect(decl.kind).toBe("fn");
    if (decl.kind === "fn") {
      expect(decl.body.kind).toBe("var");
    }
  });

  // 39. Modulo operator
  it("parses modulo: x % 2", () => {
    const tokens = [ident("x"), PERCENT, int(2)];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("binop");
    if (expr.kind === "binop") {
      expect(expr.op).toBe("%");
    }
  });

  // 40. Not operator
  it("parses not operator: !x", () => {
    const tokens = [NOT, ident("x")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("unaryop");
    if (expr.kind === "unaryop") {
      expect(expr.op).toBe("!");
    }
  });

  // 41. Error on unexpected token
  it("produces error on unexpected token", () => {
    const tokens = [PLUS, eof()];
    const parser = new Parser(tokens);
    const mod = parser.parse();
    expect(parser.getErrors().length).toBeGreaterThan(0);
  });

  // 42. Parameterized type: List(Int)
  it("parses parameterized type: List(Int)", () => {
    const tokens = [upper("List"), LPAREN, upper("Int"), RPAREN];
    const ty = parseType(tokens);
    expect(ty.kind).toBe("tnamed");
    if (ty.kind === "tnamed") {
      expect(ty.name).toBe("List");
      expect(ty.args).toHaveLength(1);
    }
  });

  // 43. Boolean literal
  it("parses boolean literals", () => {
    const tokens = [TRUE];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("bool");
    if (expr.kind === "bool") {
      expect(expr.value).toBe(true);
    }
  });

  // 44. String literal
  it("parses string literal", () => {
    const tokens = [str("hello world")];
    const expr = parseExpr(tokens);
    expect(expr.kind).toBe("string");
    if (expr.kind === "string") {
      expect(expr.value).toBe("hello world");
    }
  });
});

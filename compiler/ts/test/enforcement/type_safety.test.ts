import { describe, it, expect } from 'vitest';
import { buildToWat } from '../../src/cli/build.js';
import { TypeChecker } from '../../src/checker/infer.js';
import { LinearityChecker } from '../../src/checker/linearity.js';
import { Lexer } from '../../src/lexer/index.js';
import { Parser } from '../../src/parser/index.js';

// Helper: parse source to AST
function parse(source: string) {
  const lexer = new Lexer(source);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const errors = parser.getErrors();
  if (errors.length > 0) {
    throw new Error(`Parse errors: ${errors.map(e => e.message).join(', ')}`);
  }
  return ast;
}

// Helper: type check and return errors
function typeCheck(source: string) {
  const ast = parse(source);
  const checker = new TypeChecker();
  return checker.checkModule(ast);
}

// ═══════════════════════════════════════════════════════════════════════
// Feature 1: Effect Enforcement
// ═══════════════════════════════════════════════════════════════════════

describe('Feature 1: Effect Enforcement', () => {

  it('flags effect violation: pure function with println', () => {
    const source = `
fn pure_function(x: Int) -> Int {
  println("side effect!")
  x + 1
}
`;
    const typed = typeCheck(source);
    const effectErrors = typed.errors.filter(e =>
      e.message.includes('effect') || e.message.includes('pure') || e.message.includes('Effect') || e.message.includes('Pure')
    );
    expect(effectErrors.length).toBeGreaterThan(0);
    expect(effectErrors[0].message).toContain('IO');
  });

  it('allows IO function with effect annotation', () => {
    const source = `
fn greet(name: String) -> Unit ![IO] {
  println(name)
}
`;
    const typed = typeCheck(source);
    const effectErrors = typed.errors.filter(e =>
      e.message.includes('effect') || e.message.includes('pure') || e.message.includes('Effect') || e.message.includes('Pure')
    );
    expect(effectErrors.length).toBe(0);
  });

  it('strict mode blocks compilation on effect violation', () => {
    const source = `
fn bad(x: Int) -> Int {
  println("oops")
  x
}
`;
    expect(() => buildToWat(source, { strict: true }))
      .toThrow(/[Ee]ffect/);
  });

  it('non-strict mode allows effect violations (backward compat)', () => {
    const source = `
fn bad(x: Int) -> Int {
  println("oops")
  x
}
fn main() { println(show(bad(1))) }
`;
    // Should not throw without strict mode
    const output = buildToWat(source);
    expect(output).toContain('(module');
  });

  it('pure function without IO is fine', () => {
    const source = `
fn add(x: Int, y: Int) -> Int {
  x + y
}
`;
    const typed = typeCheck(source);
    const effectErrors = typed.errors.filter(e =>
      e.message.includes('effect') || e.message.includes('pure') || e.message.includes('Effect') || e.message.includes('Pure')
    );
    expect(effectErrors.length).toBe(0);
  });

  it('function with no return type annotation does not flag effects', () => {
    // No return type annotation = effects are inferred, not enforced
    const source = `
fn greet(name) {
  println(name)
}
`;
    const typed = typeCheck(source);
    const effectErrors = typed.errors.filter(e =>
      e.message.includes('declared pure')
    );
    expect(effectErrors.length).toBe(0);
  });
});

// ═══════════════════════════════════════════════════════════════════════
// Feature 2: Linearity Enforcement
// ═══════════════════════════════════════════════════════════════════════

describe('Feature 2: Linearity Enforcement', () => {

  it('flags double use of Owned value', () => {
    const source = `
fn consume(res: Owned(Int)) -> Int {
  res + res
}
`;
    const ast = parse(source);
    const checker = new LinearityChecker();
    const errors = checker.checkModule(ast);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors[0].message).toContain('used more than once');
  });

  it('allows single use of Owned value', () => {
    const source = `
fn consume(res: Owned(Int)) -> Int {
  res + 1
}
`;
    const ast = parse(source);
    const checker = new LinearityChecker();
    const errors = checker.checkModule(ast);
    expect(errors.length).toBe(0);
  });

  it('flags unused Owned value', () => {
    const source = `
fn waste(res: Owned(Int)) -> Int {
  42
}
`;
    const ast = parse(source);
    const checker = new LinearityChecker();
    const errors = checker.checkModule(ast);
    expect(errors.length).toBeGreaterThan(0);
    expect(errors[0].message).toContain('never used');
  });

  it('strict mode blocks compilation on linearity violation', () => {
    const source = `
fn double_use(r: Owned(Int)) -> Int {
  r + r
}
`;
    expect(() => buildToWat(source, { strict: true }))
      .toThrow(/[Ll]inearity/);
  });

  it('non-Owned parameters are not linearity-checked', () => {
    const source = `
fn normal(x: Int) -> Int {
  x + x
}
`;
    const ast = parse(source);
    const checker = new LinearityChecker();
    const errors = checker.checkModule(ast);
    expect(errors.length).toBe(0);
  });

  it('Owned value used in let binding counts correctly', () => {
    const source = `
fn transfer(res: Owned(Int)) -> Int {
  let y = res
  y
}
`;
    const ast = parse(source);
    const checker = new LinearityChecker();
    const errors = checker.checkModule(ast);
    // res is used once (in the let binding)
    expect(errors.length).toBe(0);
  });
});

// ═══════════════════════════════════════════════════════════════════════
// Feature 3: Exhaustive Pattern Matching
// ═══════════════════════════════════════════════════════════════════════

describe('Feature 3: Exhaustive Pattern Matching', () => {

  it('flags non-exhaustive match on sum type', () => {
    const source = `
type Shape = | Circle(Float) | Rectangle(Float, Float) | Triangle(Float, Float, Float)

fn area(shape: Shape) -> Float {
  match shape {
    Circle(r) => 3.14 * r * r
    Rectangle(w, h) => w * h
  }
}
`;
    const typed = typeCheck(source);
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive') || e.message.includes('exhaustive')
    );
    expect(exhaustErrors.length).toBeGreaterThan(0);
    expect(exhaustErrors[0].message).toContain('Triangle');
  });

  it('allows exhaustive match with all constructors', () => {
    const source = `
type Shape = | Circle(Float) | Rectangle(Float, Float)

fn area(shape: Shape) -> Float {
  match shape {
    Circle(r) => 3.14 * r * r
    Rectangle(w, h) => w * h
  }
}
`;
    const typed = typeCheck(source);
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive') || e.message.includes('exhaustive')
    );
    expect(exhaustErrors.length).toBe(0);
  });

  it('allows wildcard to cover remaining cases', () => {
    const source = `
type Shape = | Circle(Float) | Rectangle(Float, Float) | Triangle(Float, Float, Float)

fn area(shape: Shape) -> Float {
  match shape {
    Circle(r) => 3.14 * r * r
    _ => 0.0
  }
}
`;
    const typed = typeCheck(source);
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive') || e.message.includes('exhaustive')
    );
    expect(exhaustErrors.length).toBe(0);
  });

  it('allows variable pattern to cover remaining cases', () => {
    const source = `
type Color = | Red | Green | Blue

fn name(c: Color) -> Int {
  match c {
    Red => 1
    other => 0
  }
}
`;
    const typed = typeCheck(source);
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive')
    );
    expect(exhaustErrors.length).toBe(0);
  });

  it('strict mode blocks compilation on non-exhaustive match', () => {
    const source = `
type Dir = | North | South | East | West

fn go(d: Dir) -> Int {
  match d {
    North => 1
    South => 2
  }
}
`;
    expect(() => buildToWat(source, { strict: true }))
      .toThrow(/[Ee]xhaustive/);
  });

  it('flags multiple missing constructors', () => {
    const source = `
type Dir = | North | South | East | West

fn go(d: Dir) -> Int {
  match d {
    North => 1
  }
}
`;
    const typed = typeCheck(source);
    const exhaustErrors = typed.errors.filter(e =>
      e.message.includes('Non-exhaustive')
    );
    expect(exhaustErrors.length).toBeGreaterThan(0);
    expect(exhaustErrors[0].message).toContain('South');
    expect(exhaustErrors[0].message).toContain('East');
    expect(exhaustErrors[0].message).toContain('West');
  });
});

// ═══════════════════════════════════════════════════════════════════════
// Feature 4: Tail Call Optimization
// NOTE: TCO output tests removed — they tested TS-specific codegen.
// These will be rewritten for WASM by another agent.
// ═══════════════════════════════════════════════════════════════════════

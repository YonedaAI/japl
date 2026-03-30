import { describe, it, expect } from 'vitest';
import { Lexer } from '../lexer/index.js';
import { Parser } from '../parser/index.js';
import { Formatter } from './formatter.js';

function formatSource(input: string): string {
  const lexer = new Lexer(input);
  const tokens = lexer.tokenize();
  const parser = new Parser(tokens);
  const ast = parser.parse();
  const formatter = new Formatter();
  return formatter.format(ast);
}

describe('Formatter', () => {
  it('formats function declaration with proper spacing', () => {
    const input = 'fn add(x:Int,y:Int)->Int{x+y}';
    const formatted = formatSource(input);
    expect(formatted).toContain('fn add(x: Int, y: Int) -> Int {');
    expect(formatted).toContain('  x + y');
    expect(formatted).toContain('}');
  });

  it('formats type declaration with variants on separate lines', () => {
    const input = 'type Light=|Red|Green|Yellow';
    const formatted = formatSource(input);
    expect(formatted).toContain('type Light =');
    expect(formatted).toContain('  | Red');
    expect(formatted).toContain('  | Green');
    expect(formatted).toContain('  | Yellow');
  });

  it('formats type with fields', () => {
    const input = 'type Option(a)=|Some(a)|None';
    const formatted = formatSource(input);
    expect(formatted).toContain('type Option(a) =');
    expect(formatted).toContain('  | Some(a)');
    expect(formatted).toContain('  | None');
  });

  it('formats let binding inside function body', () => {
    const input = 'fn main(){let x=add(1,2)\nprintln(show(x))}';
    const formatted = formatSource(input);
    expect(formatted).toContain('  let x = add(1, 2)');
    expect(formatted).toContain('  println(show(x))');
  });

  it('formats import declaration', () => {
    const input = 'import List.{map,filter}';
    const formatted = formatSource(input);
    expect(formatted).toContain('import List.{map, filter}');
  });

  it('formats import without items', () => {
    const input = 'import Std.IO';
    const formatted = formatSource(input);
    expect(formatted).toContain('import Std.IO');
  });

  it('formats match expression', () => {
    const input = 'fn describe(l:Light)->String{match l{Red=>\"stop\"\nGreen=>\"go\"\nYellow=>\"caution\"}}';
    const formatted = formatSource(input);
    expect(formatted).toContain('match l {');
    expect(formatted).toContain('Red => "stop"');
    expect(formatted).toContain('Green => "go"');
    expect(formatted).toContain('Yellow => "caution"');
  });

  it('formats pub function', () => {
    const input = 'pub fn greet(name:String)->String{"hello"}';
    const formatted = formatSource(input);
    expect(formatted).toContain('pub fn greet(name: String) -> String {');
  });

  it('formats multiple top-level declarations with blank lines between', () => {
    const input = 'fn a()->Int{1}\nfn b()->Int{2}';
    const formatted = formatSource(input);
    const lines = formatted.split('\n');
    // Find the closing brace of first fn and opening of second
    const firstClose = lines.indexOf('}');
    expect(firstClose).toBeGreaterThan(0);
    // Next non-empty line should be the second function
    expect(lines[firstClose + 1]).toBe('');
    expect(lines[firstClose + 2]).toContain('fn b');
  });

  it('idempotent formatting', () => {
    const input = 'fn main() {\n  println("hello")\n}\n';
    const formatted = formatSource(input);
    expect(formatted).toBe(input);
    // Format again — should be identical
    const formatted2 = formatSource(formatted);
    expect(formatted2).toBe(formatted);
  });

  it('formats constructor expressions with spaces after commas', () => {
    const input = 'fn make()->Light{Some(1)}';
    const formatted = formatSource(input);
    expect(formatted).toContain('Some(1)');
  });

  it('formats binary operations with spaces around operators', () => {
    const input = 'fn calc(a:Int,b:Int)->Int{a+b*2}';
    const formatted = formatSource(input);
    // Parser will create binop nodes, formatter adds spaces
    expect(formatted).toMatch(/a \+ b \* 2|a \+ \(b \* 2\)/);
  });

  it('formats if expression', () => {
    const input = 'fn check(x:Int)->String{if x>0{"pos"} else {"neg"}}';
    const formatted = formatSource(input);
    expect(formatted).toContain('if x > 0 {');
    expect(formatted).toContain('} else {');
  });

  it('formats list expressions', () => {
    const input = 'fn nums()->List(Int){[1,2,3]}';
    const formatted = formatSource(input);
    expect(formatted).toContain('[1, 2, 3]');
  });

  it('formats foreign declaration', () => {
    const input = 'foreign "node:fs" fn read_file as "readFileSync"(path:String)->String';
    const formatted = formatSource(input);
    expect(formatted).toContain('foreign "node:fs" fn read_file as "readFileSync"(path: String) -> String');
  });
});

import { describe, it, expect } from 'vitest';
import { Lexer } from './lexer.js';
import { TokenKind, tokenKindName, KEYWORDS } from './token.js';
/** Helper: tokenize and strip EOF */
function lex(source) {
    const tokens = new Lexer(source).tokenize();
    return tokens.filter((t) => t.kind !== TokenKind.EOF);
}
/** Helper: tokenize and strip EOF + Newline + Comment */
function lexSignificant(source) {
    const tokens = new Lexer(source).tokenize();
    return tokens.filter((t) => t.kind !== TokenKind.EOF && t.kind !== TokenKind.Newline && t.kind !== TokenKind.Comment);
}
describe('Lexer', () => {
    // 1. Simple single-character operators
    it('tokenizes simple operators: + - * / =', () => {
        const tokens = lexSignificant('+ - * / =');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Plus,
            TokenKind.Minus,
            TokenKind.Star,
            TokenKind.Slash,
            TokenKind.Assign,
        ]);
    });
    // 2. Two-character operators: |> -> => == != <= >= <>
    it('tokenizes two-char operators: |> -> => == != <= >= <>', () => {
        const tokens = lexSignificant('|> -> => == != <= >= <>');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Pipe,
            TokenKind.Arrow,
            TokenKind.FatArrow,
            TokenKind.Eq,
            TokenKind.NotEq,
            TokenKind.LtEq,
            TokenKind.GtEq,
            TokenKind.Concat,
        ]);
    });
    // 3. Additional two-char operators: && || :: .. >>
    it('tokenizes && || :: .. >>', () => {
        const tokens = lexSignificant('&& || :: .. >>');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.And,
            TokenKind.Or,
            TokenKind.ColonColon,
            TokenKind.DotDot,
            TokenKind.Compose,
        ]);
    });
    // 4. Keywords vs identifiers
    it('distinguishes keywords from identifiers', () => {
        const tokens = lexSignificant('fn foo let bar');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Fn,
            TokenKind.Ident,
            TokenKind.Let,
            TokenKind.Ident,
        ]);
    });
    // 5. Upper vs lower ident
    it('distinguishes UpperIdent from Ident', () => {
        const tokens = lexSignificant('Some some None none');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.UpperIdent,
            TokenKind.Ident,
            TokenKind.UpperIdent,
            TokenKind.Ident,
        ]);
    });
    // 6. String literal: simple
    it('tokenizes simple string literals', () => {
        const tokens = lexSignificant('"hello"');
        expect(tokens).toHaveLength(1);
        expect(tokens[0].kind).toBe(TokenKind.String);
        expect(tokens[0].value).toBe('"hello"');
    });
    // 7. String literal: escape sequences
    it('tokenizes string literals with escape sequences', () => {
        const tokens = lexSignificant('"with \\"escape\\""');
        expect(tokens).toHaveLength(1);
        expect(tokens[0].kind).toBe(TokenKind.String);
        expect(tokens[0].value).toBe('"with \\"escape\\""');
    });
    // 8. String literal: newline escape
    it('tokenizes string literals with \\n escape', () => {
        const tokens = lexSignificant('"line\\nbreak"');
        expect(tokens).toHaveLength(1);
        expect(tokens[0].kind).toBe(TokenKind.String);
        expect(tokens[0].value).toBe('"line\\nbreak"');
    });
    // 9. Integer literals
    it('tokenizes integer literals', () => {
        const tokens = lexSignificant('42 0 100');
        expect(tokens.map((t) => t.kind)).toEqual([TokenKind.Int, TokenKind.Int, TokenKind.Int]);
        expect(tokens.map((t) => t.value)).toEqual(['42', '0', '100']);
    });
    // 10. Float literals
    it('tokenizes float literals', () => {
        const tokens = lexSignificant('3.14 0.5');
        expect(tokens.map((t) => t.kind)).toEqual([TokenKind.Float, TokenKind.Float]);
        expect(tokens.map((t) => t.value)).toEqual(['3.14', '0.5']);
    });
    // 11. Line comments
    it('tokenizes line comments', () => {
        const tokens = lex('// this is a comment\nfoo');
        const comment = tokens.find((t) => t.kind === TokenKind.Comment);
        expect(comment).toBeDefined();
        expect(comment.value).toBe('// this is a comment');
        const ident = tokens.find((t) => t.kind === TokenKind.Ident);
        expect(ident).toBeDefined();
        expect(ident.value).toBe('foo');
    });
    // 12. Block comments
    it('tokenizes block comments', () => {
        const tokens = lex('/* also ignored */ foo');
        const comment = tokens.find((t) => t.kind === TokenKind.Comment);
        expect(comment).toBeDefined();
        expect(comment.value).toBe('/* also ignored */');
        const ident = tokens.find((t) => t.kind === TokenKind.Ident);
        expect(ident).toBeDefined();
        expect(ident.value).toBe('foo');
    });
    // 13. Full expression: fn add(x: Int, y: Int) -> Int { x + y }
    it('tokenizes a full function definition', () => {
        const tokens = lexSignificant('fn add(x: Int, y: Int) -> Int { x + y }');
        const kinds = tokens.map((t) => t.kind);
        expect(kinds).toEqual([
            TokenKind.Fn,
            TokenKind.Ident, // add
            TokenKind.LParen,
            TokenKind.Ident, // x
            TokenKind.Colon,
            TokenKind.UpperIdent, // Int
            TokenKind.Comma,
            TokenKind.Ident, // y
            TokenKind.Colon,
            TokenKind.UpperIdent, // Int
            TokenKind.RParen,
            TokenKind.Arrow,
            TokenKind.UpperIdent, // Int
            TokenKind.LBrace,
            TokenKind.Ident, // x
            TokenKind.Plus,
            TokenKind.Ident, // y
            TokenKind.RBrace,
        ]);
    });
    // 14. Pattern matching: match msg { Tick(id) => handle(id) }
    it('tokenizes pattern matching expression', () => {
        const tokens = lexSignificant('match msg { Tick(id) => handle(id) }');
        const kinds = tokens.map((t) => t.kind);
        expect(kinds).toEqual([
            TokenKind.Match,
            TokenKind.Ident, // msg
            TokenKind.LBrace,
            TokenKind.UpperIdent, // Tick
            TokenKind.LParen,
            TokenKind.Ident, // id
            TokenKind.RParen,
            TokenKind.FatArrow,
            TokenKind.Ident, // handle
            TokenKind.LParen,
            TokenKind.Ident, // id
            TokenKind.RParen,
            TokenKind.RBrace,
        ]);
    });
    // 15. Pipe operator: list |> map(f) |> filter(g)
    it('tokenizes pipe expressions', () => {
        const tokens = lexSignificant('list |> map(f) |> filter(g)');
        const kinds = tokens.map((t) => t.kind);
        expect(kinds).toEqual([
            TokenKind.Ident, // list
            TokenKind.Pipe,
            TokenKind.Ident, // map
            TokenKind.LParen,
            TokenKind.Ident, // f
            TokenKind.RParen,
            TokenKind.Pipe,
            TokenKind.Ident, // filter
            TokenKind.LParen,
            TokenKind.Ident, // g
            TokenKind.RParen,
        ]);
    });
    // 16. Span tracking: verify line/col numbers
    it('tracks line and column numbers correctly', () => {
        const tokens = new Lexer('fn\nadd').tokenize();
        const fn = tokens.find((t) => t.kind === TokenKind.Fn);
        expect(fn.span.line).toBe(1);
        expect(fn.span.col).toBe(1);
        const add = tokens.find((t) => t.kind === TokenKind.Ident);
        expect(add.span.line).toBe(2);
        expect(add.span.col).toBe(1);
    });
    // 17. Span tracking with column offset
    it('tracks column offset within a line', () => {
        const tokens = lexSignificant('  foo  bar');
        expect(tokens).toHaveLength(2);
        expect(tokens[0].span.col).toBe(3); // foo starts at col 3
        expect(tokens[1].span.col).toBe(8); // bar starts at col 8
    });
    // 18. Boolean keywords: true, false
    it('tokenizes true and false as keywords', () => {
        const tokens = lexSignificant('true false');
        expect(tokens.map((t) => t.kind)).toEqual([TokenKind.True, TokenKind.False]);
    });
    // 19. Delimiters
    it('tokenizes all delimiters', () => {
        const tokens = lexSignificant('( ) { } [ ]');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.LParen,
            TokenKind.RParen,
            TokenKind.LBrace,
            TokenKind.RBrace,
            TokenKind.LBracket,
            TokenKind.RBracket,
        ]);
    });
    // 20. All 38 keywords are in the KEYWORDS map
    it('has all 38 keywords plus true/false in KEYWORDS map', () => {
        // 38 keywords + true + false = 40
        expect(KEYWORDS.size).toBe(40);
    });
    // 21. tokenKindName helper
    it('returns correct name from tokenKindName', () => {
        expect(tokenKindName(TokenKind.Fn)).toBe('Fn');
        expect(tokenKindName(TokenKind.Pipe)).toBe('Pipe');
        expect(tokenKindName(TokenKind.Ident)).toBe('Ident');
        expect(tokenKindName(TokenKind.EOF)).toBe('EOF');
    });
    // 22. Newline tokens are emitted
    it('emits Newline tokens', () => {
        const tokens = lex('a\nb');
        const kinds = tokens.map((t) => t.kind);
        expect(kinds).toEqual([TokenKind.Ident, TokenKind.Newline, TokenKind.Ident]);
    });
    // 23. EOF is always the last token
    it('always ends with EOF', () => {
        const tokens = new Lexer('').tokenize();
        expect(tokens).toHaveLength(1);
        expect(tokens[0].kind).toBe(TokenKind.EOF);
    });
    // 24. Mixed expression with let binding
    it('tokenizes let binding: let x = 42', () => {
        const tokens = lexSignificant('let x = 42');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Let,
            TokenKind.Ident,
            TokenKind.Assign,
            TokenKind.Int,
        ]);
    });
    // 25. Identifiers with underscores
    it('tokenizes identifiers with underscores', () => {
        const tokens = lexSignificant('my_func _private __x');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Ident,
            TokenKind.Ident,
            TokenKind.Ident,
        ]);
        expect(tokens.map((t) => t.value)).toEqual(['my_func', '_private', '__x']);
    });
    // 26. Bar and question mark
    it('tokenizes | and ? operators', () => {
        const tokens = lexSignificant('| ?');
        expect(tokens.map((t) => t.kind)).toEqual([TokenKind.Bar, TokenKind.Question]);
    });
    // 27. Semicolon and ampersand
    it('tokenizes ; and &', () => {
        const tokens = lexSignificant('; &');
        expect(tokens.map((t) => t.kind)).toEqual([TokenKind.Semicolon, TokenKind.Ampersand]);
    });
    // 28. Span end position is correct
    it('sets correct span end positions', () => {
        const tokens = lexSignificant('fn');
        expect(tokens[0].span.start).toBe(0);
        expect(tokens[0].span.end).toBe(2);
    });
    // 29. Number followed by dot-dot (range)
    it('distinguishes number from dot-dot operator', () => {
        const tokens = lexSignificant('1..10');
        expect(tokens.map((t) => t.kind)).toEqual([
            TokenKind.Int,
            TokenKind.DotDot,
            TokenKind.Int,
        ]);
        expect(tokens.map((t) => t.value)).toEqual(['1', '..', '10']);
    });
    // 30. Hex literals
    it('tokenizes hex literals', () => {
        const tokens = lexSignificant('0xFF 0xDEAD 0x0');
        expect(tokens.map(t => t.kind)).toEqual([TokenKind.Int, TokenKind.Int, TokenKind.Int]);
        expect(tokens.map(t => t.value)).toEqual(['0xFF', '0xDEAD', '0x0']);
    });
    // 31. Binary literals
    it('tokenizes binary literals', () => {
        const tokens = lexSignificant('0b1010 0b0 0b11111111');
        expect(tokens.map(t => t.kind)).toEqual([TokenKind.Int, TokenKind.Int, TokenKind.Int]);
        expect(tokens.map(t => t.value)).toEqual(['0b1010', '0b0', '0b11111111']);
    });
    // 32. Digit separators
    it('tokenizes numbers with separators', () => {
        const tokens = lexSignificant('1_000_000 3.14_159 0xFF_FF');
        expect(tokens[0].kind).toBe(TokenKind.Int);
        expect(tokens[0].value).toBe('1_000_000');
        expect(tokens[1].kind).toBe(TokenKind.Float);
        expect(tokens[1].value).toBe('3.14_159');
        expect(tokens[2].kind).toBe(TokenKind.Int);
        expect(tokens[2].value).toBe('0xFF_FF');
    });
    // 33. Scientific notation
    it('tokenizes scientific notation', () => {
        const tokens = lexSignificant('1e10 1.5e-3 2.4E+5');
        expect(tokens.map(t => t.kind)).toEqual([TokenKind.Float, TokenKind.Float, TokenKind.Float]);
        expect(tokens.map(t => t.value)).toEqual(['1e10', '1.5e-3', '2.4E+5']);
    });
    // 34. Binary with separators
    it('tokenizes binary with separators', () => {
        const tokens = lexSignificant('0b1111_0000');
        expect(tokens[0].value).toBe('0b1111_0000');
    });
    // 35. Complex process expression
    it('tokenizes a spawn/send/receive expression', () => {
        const tokens = lexSignificant('spawn fn() { receive { msg => done } }');
        const kinds = tokens.map((t) => t.kind);
        expect(kinds).toEqual([
            TokenKind.Spawn,
            TokenKind.Fn,
            TokenKind.LParen,
            TokenKind.RParen,
            TokenKind.LBrace,
            TokenKind.Receive,
            TokenKind.LBrace,
            TokenKind.Ident, // msg
            TokenKind.FatArrow,
            TokenKind.Done,
            TokenKind.RBrace,
            TokenKind.RBrace,
        ]);
    });
});
//# sourceMappingURL=lexer.test.js.map
import { TokenKind, KEYWORDS } from './token.js';
export class Lexer {
    source;
    pos;
    line;
    col;
    tokens;
    constructor(source) {
        this.source = source;
        this.pos = 0;
        this.line = 1;
        this.col = 1;
        this.tokens = [];
    }
    tokenize() {
        while (this.pos < this.source.length) {
            const ch = this.peek();
            // Newlines
            if (ch === '\n') {
                const start = this.pos;
                const line = this.line;
                const col = this.col;
                this.advance();
                this.tokens.push(this.makeToken(TokenKind.Newline, '\n', start, line, col));
                continue;
            }
            // Skip whitespace (spaces, tabs, carriage returns)
            if (ch === ' ' || ch === '\t' || ch === '\r') {
                this.advance();
                continue;
            }
            // Line comments: //
            if (ch === '/' && this.peekNext() === '/') {
                this.skipLineComment();
                continue;
            }
            // Block comments: /* */
            if (ch === '/' && this.peekNext() === '*') {
                this.skipBlockComment();
                continue;
            }
            // String literals
            if (ch === '"') {
                this.tokens.push(this.readString());
                continue;
            }
            // Number literals
            if (isDigit(ch)) {
                this.tokens.push(this.readNumber());
                continue;
            }
            // Identifiers and keywords
            if (isAlpha(ch) || ch === '_') {
                this.tokens.push(this.readIdentOrKeyword());
                continue;
            }
            // Operators and delimiters
            this.tokens.push(this.readOperator());
        }
        this.tokens.push(this.makeToken(TokenKind.EOF, '', this.pos, this.line, this.col));
        return this.tokens;
    }
    advance() {
        const ch = this.source[this.pos];
        this.pos++;
        if (ch === '\n') {
            this.line++;
            this.col = 1;
        }
        else {
            this.col++;
        }
        return ch;
    }
    peek() {
        if (this.pos >= this.source.length)
            return '\0';
        return this.source[this.pos];
    }
    peekNext() {
        if (this.pos + 1 >= this.source.length)
            return '\0';
        return this.source[this.pos + 1];
    }
    skipWhitespace() {
        while (this.pos < this.source.length) {
            const ch = this.peek();
            if (ch === ' ' || ch === '\t' || ch === '\r') {
                this.advance();
            }
            else {
                break;
            }
        }
    }
    skipLineComment() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        // Skip the //
        this.advance();
        this.advance();
        while (this.pos < this.source.length && this.peek() !== '\n') {
            this.advance();
        }
        const value = this.source.slice(start, this.pos);
        this.tokens.push(this.makeToken(TokenKind.Comment, value, start, line, col));
    }
    skipBlockComment() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        // Skip /*
        this.advance();
        this.advance();
        let depth = 1;
        while (this.pos < this.source.length && depth > 0) {
            if (this.peek() === '/' && this.peekNext() === '*') {
                this.advance();
                this.advance();
                depth++;
            }
            else if (this.peek() === '*' && this.peekNext() === '/') {
                this.advance();
                this.advance();
                depth--;
            }
            else {
                this.advance();
            }
        }
        const value = this.source.slice(start, this.pos);
        this.tokens.push(this.makeToken(TokenKind.Comment, value, start, line, col));
    }
    readString() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        // Skip opening quote
        this.advance();
        let value = '"';
        while (this.pos < this.source.length && this.peek() !== '"') {
            if (this.peek() === '\\') {
                value += this.advance(); // backslash
                if (this.pos < this.source.length) {
                    value += this.advance(); // escaped char
                }
            }
            else {
                value += this.advance();
            }
        }
        if (this.pos < this.source.length) {
            value += this.advance(); // closing quote
        }
        return this.makeToken(TokenKind.String, value, start, line, col);
    }
    readNumber() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        let isFloat = false;
        while (this.pos < this.source.length && isDigit(this.peek())) {
            this.advance();
        }
        // Check for decimal point (but not .. operator)
        if (this.peek() === '.' && this.peekNext() !== '.' && isDigit(this.peekNext())) {
            isFloat = true;
            this.advance(); // consume '.'
            while (this.pos < this.source.length && isDigit(this.peek())) {
                this.advance();
            }
        }
        const value = this.source.slice(start, this.pos);
        const kind = isFloat ? TokenKind.Float : TokenKind.Int;
        return this.makeToken(kind, value, start, line, col);
    }
    readIdentOrKeyword() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        while (this.pos < this.source.length && isAlphaNumOrUnderscore(this.peek())) {
            this.advance();
        }
        const value = this.source.slice(start, this.pos);
        // Check keywords map
        const kwKind = KEYWORDS.get(value);
        if (kwKind !== undefined) {
            return this.makeToken(kwKind, value, start, line, col);
        }
        // Distinguish Ident from UpperIdent by first character
        const kind = isUpper(value[0]) ? TokenKind.UpperIdent : TokenKind.Ident;
        return this.makeToken(kind, value, start, line, col);
    }
    readOperator() {
        const start = this.pos;
        const line = this.line;
        const col = this.col;
        const ch = this.advance();
        const next = this.peek();
        switch (ch) {
            case '+':
                return this.makeToken(TokenKind.Plus, '+', start, line, col);
            case '-':
                if (next === '>') {
                    this.advance();
                    return this.makeToken(TokenKind.Arrow, '->', start, line, col);
                }
                return this.makeToken(TokenKind.Minus, '-', start, line, col);
            case '*':
                return this.makeToken(TokenKind.Star, '*', start, line, col);
            case '/':
                return this.makeToken(TokenKind.Slash, '/', start, line, col);
            case '%':
                return this.makeToken(TokenKind.Percent, '%', start, line, col);
            case '=':
                if (next === '=') {
                    this.advance();
                    return this.makeToken(TokenKind.Eq, '==', start, line, col);
                }
                if (next === '>') {
                    this.advance();
                    return this.makeToken(TokenKind.FatArrow, '=>', start, line, col);
                }
                return this.makeToken(TokenKind.Assign, '=', start, line, col);
            case '!':
                if (next === '=') {
                    this.advance();
                    return this.makeToken(TokenKind.NotEq, '!=', start, line, col);
                }
                return this.makeToken(TokenKind.Not, '!', start, line, col);
            case '<':
                if (next === '=') {
                    this.advance();
                    return this.makeToken(TokenKind.LtEq, '<=', start, line, col);
                }
                if (next === '>') {
                    this.advance();
                    return this.makeToken(TokenKind.Concat, '<>', start, line, col);
                }
                return this.makeToken(TokenKind.Lt, '<', start, line, col);
            case '>':
                if (next === '=') {
                    this.advance();
                    return this.makeToken(TokenKind.GtEq, '>=', start, line, col);
                }
                if (next === '>') {
                    this.advance();
                    return this.makeToken(TokenKind.Compose, '>>', start, line, col);
                }
                return this.makeToken(TokenKind.Gt, '>', start, line, col);
            case '&':
                if (next === '&') {
                    this.advance();
                    return this.makeToken(TokenKind.And, '&&', start, line, col);
                }
                return this.makeToken(TokenKind.Ampersand, '&', start, line, col);
            case '|':
                if (next === '>') {
                    this.advance();
                    return this.makeToken(TokenKind.Pipe, '|>', start, line, col);
                }
                if (next === '|') {
                    this.advance();
                    return this.makeToken(TokenKind.Or, '||', start, line, col);
                }
                return this.makeToken(TokenKind.Bar, '|', start, line, col);
            case '?':
                return this.makeToken(TokenKind.Question, '?', start, line, col);
            case '.':
                if (next === '.') {
                    this.advance();
                    return this.makeToken(TokenKind.DotDot, '..', start, line, col);
                }
                return this.makeToken(TokenKind.Dot, '.', start, line, col);
            case ':':
                if (next === ':') {
                    this.advance();
                    return this.makeToken(TokenKind.ColonColon, '::', start, line, col);
                }
                return this.makeToken(TokenKind.Colon, ':', start, line, col);
            case ',':
                return this.makeToken(TokenKind.Comma, ',', start, line, col);
            case ';':
                return this.makeToken(TokenKind.Semicolon, ';', start, line, col);
            case '(':
                return this.makeToken(TokenKind.LParen, '(', start, line, col);
            case ')':
                return this.makeToken(TokenKind.RParen, ')', start, line, col);
            case '{':
                return this.makeToken(TokenKind.LBrace, '{', start, line, col);
            case '}':
                return this.makeToken(TokenKind.RBrace, '}', start, line, col);
            case '[':
                return this.makeToken(TokenKind.LBracket, '[', start, line, col);
            case ']':
                return this.makeToken(TokenKind.RBracket, ']', start, line, col);
            default:
                // Unknown character - skip it and produce an Ident token with the character
                // This is a fallback; in a real compiler we'd report an error.
                return this.makeToken(TokenKind.Ident, ch, start, line, col);
        }
    }
    makeToken(kind, value, start, line, col) {
        return {
            kind,
            value,
            span: { start, end: this.pos, line, col },
        };
    }
}
function isDigit(ch) {
    return ch >= '0' && ch <= '9';
}
function isAlpha(ch) {
    return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_';
}
function isUpper(ch) {
    return ch >= 'A' && ch <= 'Z';
}
function isAlphaNumOrUnderscore(ch) {
    return isAlpha(ch) || isDigit(ch);
}
//# sourceMappingURL=lexer.js.map
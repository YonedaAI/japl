import { Token } from './token.js';
export declare class Lexer {
    private source;
    private pos;
    private line;
    private col;
    private tokens;
    constructor(source: string);
    tokenize(): Token[];
    private advance;
    private peek;
    private peekNext;
    private skipWhitespace;
    private skipLineComment;
    private skipBlockComment;
    private readString;
    private readNumber;
    private readIdentOrKeyword;
    private readOperator;
    private makeToken;
}

import { Span } from '../lexer/token.js';
export declare class TypeError extends Error {
    message: string;
    span: Span;
    notes?: string[] | undefined;
    constructor(message: string, span: Span, notes?: string[] | undefined);
    toString(): string;
}

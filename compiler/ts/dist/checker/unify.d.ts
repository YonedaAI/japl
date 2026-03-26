import { Type } from './types.js';
import { Span } from '../lexer/token.js';
export declare class UnificationEngine {
    private substitution;
    /** Unify two types, or throw TypeError on failure. */
    unify(a: Type, b: Type, span: Span): void;
    /** Follow substitution chain to resolve a type. */
    resolve(t: Type): Type;
    /** Deeply resolve all variables in a type. */
    deepResolve(t: Type): Type;
    private unifyRecord;
    private unifyEffects;
    private occursCheck;
}

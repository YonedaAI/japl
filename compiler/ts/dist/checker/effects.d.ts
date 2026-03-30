import { EffectRow } from './types.js';
import { TypeError } from './errors.js';
import { Span } from '../lexer/token.js';
export declare class EffectChecker {
    /**
     * Check that a declared effect annotation is satisfied by the inferred effects.
     * If no annotation, the function's effects are inferred (no check needed).
     */
    checkPurity(declaredEffects: EffectRow | undefined, inferredEffects: EffectRow, span: Span): TypeError | null;
    /**
     * Check that actual effects are subsumed by declared effects.
     * declared must be a superset of actual.
     */
    checkSubsumption(declared: EffectRow, actual: EffectRow, span: Span): TypeError | null;
    /** Check if sub effect row is a sub-effect of sup. */
    isSubEffect(sub: EffectRow, sup: EffectRow): boolean;
    /**
     * Effect subsumption rules:
     * - IO subsumes pure (anything can be called from IO context)
     * - Process subsumes Async
     * - LLM subsumes pure (LLM functions can call pure functions)
     * - LLM does NOT subsume IO (they are separate effects)
     */
    private isSubsumedByDeclared;
}

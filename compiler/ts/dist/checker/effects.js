import { effectRowToString } from './types.js';
import { TypeError } from './errors.js';
export class EffectChecker {
    /**
     * Check that a declared effect annotation is satisfied by the inferred effects.
     * If no annotation, the function's effects are inferred (no check needed).
     */
    checkPurity(declaredEffects, inferredEffects, span) {
        if (!declaredEffects)
            return null; // no annotation: effects are inferred
        // A function with no effects declared must be pure
        if (declaredEffects.effects.size === 0 && !declaredEffects.open) {
            if (inferredEffects.effects.size > 0) {
                const effStr = effectRowToString(inferredEffects);
                return new TypeError(`Function is declared pure but has effects: ${effStr}`, span, ["Remove the effectful operations or add an effect annotation"]);
            }
            return null;
        }
        return this.checkSubsumption(declaredEffects, inferredEffects, span);
    }
    /**
     * Check that actual effects are subsumed by declared effects.
     * declared must be a superset of actual.
     */
    checkSubsumption(declared, actual, span) {
        if (declared.open)
            return null; // open row allows anything
        for (const eff of actual.effects) {
            if (!declared.effects.has(eff) && !this.isSubsumedByDeclared(eff, declared)) {
                return new TypeError(`Cannot use ${eff.toUpperCase()} operation in function declared with effects ${effectRowToString(declared) || "Pure"}`, span, [`Add '${eff}' to the function's effect annotation`]);
            }
        }
        return null;
    }
    /** Check if sub effect row is a sub-effect of sup. */
    isSubEffect(sub, sup) {
        if (sup.open)
            return true;
        for (const eff of sub.effects) {
            if (!sup.effects.has(eff) && !this.isSubsumedByDeclared(eff, sup)) {
                return false;
            }
        }
        return true;
    }
    /**
     * Effect subsumption rules:
     * - IO subsumes pure (anything can be called from IO context)
     * - Process subsumes Async
     */
    isSubsumedByDeclared(eff, declared) {
        // pure is always subsumed
        if (eff === "pure")
            return true;
        // async is subsumed by process
        if (eff === "async" && declared.effects.has("process"))
            return true;
        return false;
    }
}
//# sourceMappingURL=effects.js.map
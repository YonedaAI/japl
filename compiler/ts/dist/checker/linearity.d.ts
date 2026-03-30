import * as AST from '../parser/ast.js';
import { TypeError } from './errors.js';
export declare class LinearityChecker {
    private errors;
    /**
     * Check a module for linearity violations.
     * Returns errors for any Owned<T> values used more than once.
     */
    checkModule(mod: AST.Module): TypeError[];
    private checkFnDecl;
    private isOwnedType;
    private countVarUses;
}

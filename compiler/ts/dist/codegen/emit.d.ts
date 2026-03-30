import * as IR from '../ir/ir.js';
export interface EmitModuleOptions {
    /** Whether this is the entry file (calls main, etc.) */
    isEntry: boolean;
    /** Map from module name to relative .js path for import rewriting */
    importRewrites: Map<string, string>;
}
export declare class TsEmitter {
    private output;
    private indent;
    private usedRuntimeImports;
    private matchCounter;
    private foreignImports;
    private foreignBuiltinNames;
    private importRewrites;
    private isModuleBuild;
    private currentTcoParams;
    private locallyDefinedConstructors;
    emit(module: IR.IrModule): string;
    emitModule(module: IR.IrModule, options: EmitModuleOptions): string;
    private scanImports;
    private scanExprImports;
    private emitRuntimeImports;
    private emitForeignImports;
    private emitDecl;
    private emitFnDecl;
    private emitTypeDecl;
    private emitRecordTypeDecl;
    private emitTestDecl;
    private emitImportDecl;
    private emitExpr;
    private emitBinop;
    private mapOp;
    private emitLambda;
    private emitConstruct;
    private emitLetAsIife;
    private emitIfAsIife;
    private emitMatchAsIife;
    private emitBlockAsIife;
    private emitReceiveAsIife;
    private emitTry;
    private emitExprAsStatements;
    private emitLetChain;
    private emitExprAsReturn;
    private emitMatchStatements;
    private emitPatternCondition;
    private emitPatternBindings;
    private isSimpleExpr;
    private mapType;
    private sanitizeName;
    private line;
    private indentStr;
    private indented;
}

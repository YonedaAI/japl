import * as AST from '../parser/ast.js';
import { Span } from '../lexer/token.js';
import {
  Type, TypeScheme, EffectRow, Effect,
  INT, FLOAT, BYTE, STRING, BOOL, UNIT, NEVER, PURE, IO,
  freshVar, freeVars, typeToString, monotype, resetVarCounter,
} from './types.js';
import { UnificationEngine } from './unify.js';
import { TypeEnv, ConstructorInfo } from './env.js';
import { EffectChecker } from './effects.js';
import { TypeError } from './errors.js';

// ─── Typed output ───

export type TypedModule = {
  decls: AST.Decl[];
  types: Map<AST.Expr, Type>;
  errors: TypeError[];
};

// ─── Type Checker ───

export class TypeChecker {
  private unifier: UnificationEngine = new UnificationEngine();
  private env: TypeEnv = new TypeEnv();
  private effectChecker: EffectChecker = new EffectChecker();
  private errors: TypeError[] = [];
  private exprTypes: Map<AST.Expr, Type> = new Map();

  constructor() {
    this.env.seedBuiltins();
  }

  checkModule(mod: AST.Module): TypedModule {
    for (const decl of mod.decls) {
      this.inferDecl(decl);
    }
    return {
      decls: mod.decls,
      types: this.exprTypes,
      errors: this.errors,
    };
  }

  getErrors(): TypeError[] {
    return this.errors;
  }

  getEnv(): TypeEnv {
    return this.env;
  }

  getUnifier(): UnificationEngine {
    return this.unifier;
  }

  // ─── Declaration Inference ───

  private inferDecl(decl: AST.Decl): void {
    switch (decl.kind) {
      case "fn":
        this.inferFnDecl(decl);
        break;
      case "type":
        this.inferTypeDecl(decl);
        break;
      case "record_type":
        this.inferRecordTypeDecl(decl);
        break;
      case "trait":
        // Register the trait
        this.env.defineTrait(decl.name, {
          name: decl.name,
          typeParam: decl.typeParam,
          supertraits: decl.supertraits,
          methods: [],
        });
        break;
      case "impl":
        for (const method of decl.methods) {
          this.inferDecl(method);
        }
        break;
      case "test":
        this.env.pushScope();
        this.inferExpr(decl.body);
        this.env.popScope();
        break;
      case "module":
        this.env.pushScope();
        for (const d of decl.decls) {
          this.inferDecl(d);
        }
        this.env.popScope();
        break;
      case "import":
      case "supervisor":
      case "foreign":
        break; // handled elsewhere or stubs
    }
  }

  private inferFnDecl(
    decl: Extract<AST.Decl, { kind: "fn" }>,
  ): void {
    this.env.pushScope();

    // Determine parameter types from annotations or fresh vars
    const paramTypes: Type[] = decl.params.map(p => {
      const t = p.type ? this.lowerTypeExpr(p.type) : freshVar();
      this.env.bind(p.name, monotype(t));
      return t;
    });

    // Determine declared return type
    const declaredRet = decl.returnType ? this.lowerTypeExpr(decl.returnType) : freshVar();

    // Determine declared effects
    const declaredEffects = decl.effects ? this.lowerEffectExpr(decl.effects) : undefined;

    // For recursive functions, bind the name before checking body
    const fnType: Type = {
      kind: "fn",
      params: paramTypes,
      ret: declaredRet,
      effects: declaredEffects ?? { effects: new Set(), open: true },
    };
    this.env.bind(decl.name, monotype(fnType));

    // Infer body
    const [bodyType, bodyEffects] = this.inferExpr(decl.body);

    // Unify body type with declared return type
    try {
      this.unifier.unify(bodyType, declaredRet, decl.span);
    } catch (e) {
      if (e instanceof TypeError) {
        this.errors.push(e);
      }
    }

    // Check effects
    if (declaredEffects) {
      const err = this.effectChecker.checkPurity(declaredEffects, bodyEffects, decl.span);
      if (err) this.errors.push(err);
    } else if (decl.returnType && !decl.effects && bodyEffects.effects.size > 0) {
      // Function has return type annotation but no effect annotation → implicitly pure
      // Flag if the body has effects (e.g. IO)
      const effNames = [...bodyEffects.effects].map(e => e.toUpperCase()).join(', ');
      this.errors.push(new TypeError(
        `Function '${decl.name}' is declared pure (no effect annotation) but has effects: ${effNames}. Add an effect annotation like ![${effNames}]`,
        decl.span,
        ["Add an effect annotation or remove the effectful operations"],
      ));
    }

    this.env.popScope();

    // Bind in outer scope (generalize)
    const resolvedFn: Type = {
      kind: "fn",
      params: paramTypes.map(t => this.unifier.deepResolve(t)),
      ret: this.unifier.deepResolve(declaredRet),
      effects: declaredEffects ?? bodyEffects,
    };
    const scheme = this.generalize(resolvedFn);
    this.env.bind(decl.name, scheme);
  }

  private inferTypeDecl(
    decl: Extract<AST.Decl, { kind: "type" }>,
  ): void {
    // Create type param mappings
    const typeParamVars: Map<string, Type> = new Map();
    for (const tp of decl.typeParams) {
      typeParamVars.set(tp, freshVar());
    }

    // Build the result type
    const resultType: Type = decl.typeParams.length === 0
      ? { kind: "named", name: decl.name, args: [] }
      : { kind: "named", name: decl.name, args: decl.typeParams.map(tp => typeParamVars.get(tp)!) };

    // Register each variant as a constructor
    const variantDefs = decl.variants.map(v => {
      const fieldTypes = v.fields.map(f => this.lowerTypeExprWithParams(f, typeParamVars));
      return { name: v.name, fields: fieldTypes };
    });

    this.env.defineType(decl.name, {
      name: decl.name,
      typeParams: decl.typeParams,
      variants: variantDefs,
    });

    for (const v of decl.variants) {
      const fieldTypes = v.fields.map(f => this.lowerTypeExprWithParams(f, typeParamVars));
      this.env.defineConstructor(v.name, {
        typeName: decl.name,
        typeParams: decl.typeParams,
        fieldTypes,
        resultType,
      });
    }
  }

  private inferRecordTypeDecl(
    decl: Extract<AST.Decl, { kind: "record_type" }>,
  ): void {
    const typeParamVars: Map<string, Type> = new Map();
    for (const tp of decl.typeParams) {
      typeParamVars.set(tp, freshVar());
    }

    const fields = new Map<string, Type>();
    for (const f of decl.fields) {
      fields.set(f.name, this.lowerTypeExprWithParams(f.type, typeParamVars));
    }

    const recordType: Type = { kind: "record", fields };
    this.env.defineType(decl.name, {
      name: decl.name,
      typeParams: decl.typeParams,
      variants: [],
    });
  }

  // ─── Expression Inference ───

  inferExpr(expr: AST.Expr): [Type, EffectRow] {
    const [type, effects] = this.inferExprInner(expr);
    this.exprTypes.set(expr, type);
    return [type, effects];
  }

  private inferExprInner(expr: AST.Expr): [Type, EffectRow] {
    switch (expr.kind) {
      case "int":
        return [INT, PURE];
      case "float":
        return [FLOAT, PURE];
      case "string":
        return [STRING, PURE];
      case "bool":
        return [BOOL, PURE];
      case "unit":
        return [UNIT, PURE];

      case "var":
        return this.inferVar(expr);
      case "constructor":
        return this.inferConstructor(expr);
      case "app":
        return this.inferApp(expr);
      case "lambda":
        return this.inferLambda(expr);
      case "let":
        return this.inferLet(expr);
      case "match":
        return this.inferMatch(expr);
      case "if":
        return this.inferIf(expr);
      case "pipe":
        return this.inferPipe(expr);
      case "binop":
        return this.inferBinOp(expr);
      case "unaryop":
        return this.inferUnaryOp(expr);
      case "record":
        return this.inferRecord(expr);
      case "field_access":
        return this.inferFieldAccess(expr);
      case "record_update":
        return this.inferRecordUpdate(expr);
      case "list":
        return this.inferList(expr);
      case "block":
        return this.inferBlock(expr);
      case "spawn":
        return this.inferSpawn(expr);
      case "send":
        return this.inferSend(expr);
      case "receive":
        return this.inferReceive(expr);
      case "try":
        return this.inferTry(expr);
      case "return":
        return this.inferReturn(expr);
    }
  }

  private inferVar(expr: Extract<AST.Expr, { kind: "var" }>): [Type, EffectRow] {
    const scheme = this.env.lookup(expr.name);
    if (!scheme) {
      this.errors.push(new TypeError(`Undefined variable '${expr.name}'`, expr.span));
      return [freshVar(), PURE];
    }
    const t = this.instantiate(scheme);
    return [t, PURE];
  }

  private inferConstructor(expr: Extract<AST.Expr, { kind: "constructor" }>): [Type, EffectRow] {
    const info = this.env.lookupConstructor(expr.name);
    if (!info) {
      this.errors.push(new TypeError(`Unknown constructor '${expr.name}'`, expr.span));
      return [freshVar(), PURE];
    }

    // Instantiate with fresh vars
    const freshMapping = new Map<number, Type>();
    const instantiateCtorType = (t: Type): Type => this.instantiateType(t, freshMapping);

    const expectedArgTypes = info.fieldTypes.map(instantiateCtorType);
    const resultType = instantiateCtorType(info.resultType);

    if (expr.args.length !== expectedArgTypes.length) {
      this.errors.push(new TypeError(
        `Constructor ${expr.name} expects ${expectedArgTypes.length} argument${expectedArgTypes.length === 1 ? '' : 's'} but got ${expr.args.length}`,
        expr.span,
      ));
      return [resultType, PURE];
    }

    let effects = PURE;
    for (let i = 0; i < expr.args.length; i++) {
      const [argType, argEff] = this.inferExpr(expr.args[i]);
      try {
        this.unifier.unify(argType, expectedArgTypes[i], expr.args[i].span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      effects = this.mergeEffects(effects, argEff);
    }

    return [resultType, effects];
  }

  private inferApp(expr: Extract<AST.Expr, { kind: "app" }>): [Type, EffectRow] {
    const [fnType, fnEff] = this.inferExpr(expr.fn);

    const resolvedFn = this.unifier.resolve(fnType);

    // If it's already a function type, check against it
    if (resolvedFn.kind === "fn") {
      if (expr.args.length !== resolvedFn.params.length) {
        this.errors.push(new TypeError(
          `Function expects ${resolvedFn.params.length} argument${resolvedFn.params.length === 1 ? '' : 's'} but got ${expr.args.length}`,
          expr.span,
        ));
        return [resolvedFn.ret, fnEff];
      }

      let effects = this.mergeEffects(fnEff, resolvedFn.effects);
      for (let i = 0; i < expr.args.length; i++) {
        const [argType, argEff] = this.inferExpr(expr.args[i]);
        try {
          this.unifier.unify(argType, resolvedFn.params[i], expr.args[i].span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        effects = this.mergeEffects(effects, argEff);
      }
      return [resolvedFn.ret, effects];
    }

    // fnType is a variable or something else: create expected fn type and unify
    const argResults: [Type, EffectRow][] = expr.args.map(a => this.inferExpr(a));
    const retType = freshVar();
    const expectedFnType: Type = {
      kind: "fn",
      params: argResults.map(([t]) => t),
      ret: retType,
      effects: { effects: new Set(), open: true },
    };

    try {
      this.unifier.unify(fnType, expectedFnType, expr.span);
    } catch (e) {
      if (e instanceof TypeError) this.errors.push(e);
    }

    let effects = fnEff;
    for (const [, eff] of argResults) {
      effects = this.mergeEffects(effects, eff);
    }
    return [retType, effects];
  }

  private inferLambda(expr: Extract<AST.Expr, { kind: "lambda" }>): [Type, EffectRow] {
    this.env.pushScope();

    const paramTypes = expr.params.map(p => {
      const t = p.type ? this.lowerTypeExpr(p.type) : freshVar();
      this.env.bind(p.name, monotype(t));
      return t;
    });

    const [bodyType, bodyEffects] = this.inferExpr(expr.body);

    this.env.popScope();

    const fnType: Type = {
      kind: "fn",
      params: paramTypes,
      ret: bodyType,
      effects: bodyEffects,
    };
    return [fnType, PURE]; // The lambda itself is pure; effects are tracked in its type
  }

  private inferLet(expr: Extract<AST.Expr, { kind: "let" }>): [Type, EffectRow] {
    const [valType, valEff] = this.inferExpr(expr.value);

    // If there's a type annotation, unify
    if (expr.type) {
      const annotated = this.lowerTypeExpr(expr.type);
      try {
        this.unifier.unify(valType, annotated, expr.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
    }

    // Generalize and bind
    const scheme = this.generalize(this.unifier.deepResolve(valType));
    this.env.pushScope();
    this.env.bind(expr.name, scheme);

    const [bodyType, bodyEff] = this.inferExpr(expr.body);
    this.env.popScope();

    return [bodyType, this.mergeEffects(valEff, bodyEff)];
  }

  private inferMatch(expr: Extract<AST.Expr, { kind: "match" }>): [Type, EffectRow] {
    const [scrutType, scrutEff] = this.inferExpr(expr.scrutinee);
    const resultType = freshVar();
    let effects = scrutEff;

    for (const arm of expr.arms) {
      this.env.pushScope();

      // Infer pattern bindings
      this.inferPattern(arm.pattern, scrutType);

      // Check guard if present
      if (arm.guard) {
        const [guardType, guardEff] = this.inferExpr(arm.guard);
        try {
          this.unifier.unify(guardType, BOOL, arm.guard.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        effects = this.mergeEffects(effects, guardEff);
      }

      const [armType, armEff] = this.inferExpr(arm.body);
      try {
        this.unifier.unify(armType, resultType, arm.body.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      effects = this.mergeEffects(effects, armEff);

      this.env.popScope();
    }

    // Exhaustiveness check
    this.checkExhaustiveness(expr, scrutType);

    return [resultType, effects];
  }

  private checkExhaustiveness(expr: Extract<AST.Expr, { kind: "match" }>, scrutType: Type): void {
    const resolved = this.unifier.resolve(scrutType);

    // Check for wildcard or variable patterns that cover everything
    const hasWildcard = expr.arms.some(arm =>
      arm.pattern.kind === 'pwildcard' || arm.pattern.kind === 'pvar'
    );
    if (hasWildcard) return; // Wildcard covers all cases

    // For named types (user-defined sum types), check constructor coverage
    if (resolved.kind === 'named') {
      const typeDef = this.env.lookupType(resolved.name);
      if (typeDef && typeDef.variants.length > 0) {
        const coveredTags = new Set(
          expr.arms
            .map(arm => arm.pattern)
            .filter((p): p is Extract<AST.Pattern, { kind: 'pconstructor' }> => p.kind === 'pconstructor')
            .map(p => p.name)
        );
        const missing = typeDef.variants.filter(v => !coveredTags.has(v.name));
        if (missing.length > 0) {
          this.errors.push(new TypeError(
            `Non-exhaustive pattern match: missing ${missing.map(v => v.name).join(', ')}`,
            expr.span,
          ));
        }
      }
    }

    // For built-in Option type
    if (resolved.kind === 'option') {
      const coveredTags = new Set(
        expr.arms
          .map(arm => arm.pattern)
          .filter((p): p is Extract<AST.Pattern, { kind: 'pconstructor' }> => p.kind === 'pconstructor')
          .map(p => p.name)
      );
      const optionVariants = ['Some', 'None'];
      const missing = optionVariants.filter(v => !coveredTags.has(v));
      if (missing.length > 0) {
        this.errors.push(new TypeError(
          `Non-exhaustive pattern match: missing ${missing.join(', ')}`,
          expr.span,
        ));
      }
    }

    // For built-in Result type
    if (resolved.kind === 'result') {
      const coveredTags = new Set(
        expr.arms
          .map(arm => arm.pattern)
          .filter((p): p is Extract<AST.Pattern, { kind: 'pconstructor' }> => p.kind === 'pconstructor')
          .map(p => p.name)
      );
      const resultVariants = ['Ok', 'Err'];
      const missing = resultVariants.filter(v => !coveredTags.has(v));
      if (missing.length > 0) {
        this.errors.push(new TypeError(
          `Non-exhaustive pattern match: missing ${missing.join(', ')}`,
          expr.span,
        ));
      }
    }
  }

  private inferPattern(pat: AST.Pattern, scrutinee: Type): void {
    switch (pat.kind) {
      case "pvar":
        this.env.bind(pat.name, monotype(scrutinee));
        break;

      case "pwildcard":
        break;

      case "pliteral": {
        const [litType] = this.inferExpr(pat.value);
        try {
          this.unifier.unify(litType, scrutinee, pat.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        break;
      }

      case "pconstructor": {
        const info = this.env.lookupConstructor(pat.name);
        if (!info) {
          this.errors.push(new TypeError(`Unknown constructor '${pat.name}'`, pat.span));
          return;
        }
        const freshMapping = new Map<number, Type>();
        const instantiateCtorType = (t: Type): Type => this.instantiateType(t, freshMapping);
        const fieldTypes = info.fieldTypes.map(instantiateCtorType);
        const resultType = instantiateCtorType(info.resultType);

        try {
          this.unifier.unify(scrutinee, resultType, pat.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }

        if (pat.args.length !== fieldTypes.length) {
          this.errors.push(new TypeError(
            `Constructor ${pat.name} expects ${fieldTypes.length} argument${fieldTypes.length === 1 ? '' : 's'} but got ${pat.args.length}`,
            pat.span,
          ));
          return;
        }
        for (let i = 0; i < pat.args.length; i++) {
          this.inferPattern(pat.args[i], fieldTypes[i]);
        }
        break;
      }

      case "precord": {
        for (const [fieldName, fieldPat] of pat.fields) {
          const fieldType = freshVar();
          this.inferPattern(fieldPat, fieldType);
        }
        break;
      }

      case "plist": {
        const elemType = freshVar();
        const listType: Type = { kind: "list", element: elemType };
        try {
          this.unifier.unify(scrutinee, listType, pat.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        for (const elem of pat.elements) {
          this.inferPattern(elem, elemType);
        }
        if (pat.rest) {
          this.env.bind(pat.rest, monotype(listType));
        }
        break;
      }

      case "ptuple": {
        const elemTypes = pat.elements.map(() => freshVar());
        const tupleType: Type = { kind: "tuple", elements: elemTypes };
        try {
          this.unifier.unify(scrutinee, tupleType, pat.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        for (let i = 0; i < pat.elements.length; i++) {
          this.inferPattern(pat.elements[i], elemTypes[i]);
        }
        break;
      }
    }
  }

  private inferIf(expr: Extract<AST.Expr, { kind: "if" }>): [Type, EffectRow] {
    const [condType, condEff] = this.inferExpr(expr.condition);
    try {
      this.unifier.unify(condType, BOOL, expr.condition.span);
    } catch (e) {
      if (e instanceof TypeError) this.errors.push(e);
    }

    const [thenType, thenEff] = this.inferExpr(expr.then);
    let effects = this.mergeEffects(condEff, thenEff);

    if (expr.else) {
      const [elseType, elseEff] = this.inferExpr(expr.else);
      try {
        this.unifier.unify(thenType, elseType, expr.else.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      effects = this.mergeEffects(effects, elseEff);
      return [thenType, effects];
    }

    // No else branch: result is Unit
    return [UNIT, effects];
  }

  private inferPipe(expr: Extract<AST.Expr, { kind: "pipe" }>): [Type, EffectRow] {
    // a |> f  desugars to f(a)
    const [leftType, leftEff] = this.inferExpr(expr.left);

    // If the right side is an application f(x), desugar to f(a, x)
    if (expr.right.kind === "app") {
      const [fnType, fnEff] = this.inferExpr(expr.right.fn);
      const resolvedFn = this.unifier.resolve(fnType);

      // Collect the explicit args
      const argResults: [Type, EffectRow][] = expr.right.args.map(a => this.inferExpr(a));

      // Build all args: piped value first, then explicit args
      const allArgTypes = [leftType, ...argResults.map(([t]) => t)];
      const retType = freshVar();
      const expectedFnType: Type = {
        kind: "fn",
        params: allArgTypes,
        ret: retType,
        effects: { effects: new Set(), open: true },
      };

      try {
        this.unifier.unify(fnType, expectedFnType, expr.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }

      let effects = this.mergeEffects(leftEff, fnEff);
      for (const [, eff] of argResults) {
        effects = this.mergeEffects(effects, eff);
      }
      return [retType, effects];
    }

    // Simple case: a |> f  desugars to f(a)
    const [fnType, fnEff] = this.inferExpr(expr.right);
    const retType = freshVar();
    const expectedFnType: Type = {
      kind: "fn",
      params: [leftType],
      ret: retType,
      effects: { effects: new Set(), open: true },
    };

    try {
      this.unifier.unify(fnType, expectedFnType, expr.span);
    } catch (e) {
      if (e instanceof TypeError) this.errors.push(e);
    }

    return [retType, this.mergeEffects(leftEff, fnEff)];
  }

  private inferBinOp(expr: Extract<AST.Expr, { kind: "binop" }>): [Type, EffectRow] {
    const [leftType, leftEff] = this.inferExpr(expr.left);
    const [rightType, rightEff] = this.inferExpr(expr.right);
    const effects = this.mergeEffects(leftEff, rightEff);

    switch (expr.op) {
      case "+": case "-": case "*": case "/": case "%": {
        // Arithmetic: both operands must be the same numeric type, result is same type
        // No implicit promotion between Int, Float, and Byte
        const resolvedLeft = this.unifier.resolve(leftType);
        const resolvedRight = this.unifier.resolve(rightType);
        const numericKinds = new Set(["int", "float", "byte"]);
        const leftIsNumeric = numericKinds.has(resolvedLeft.kind);
        const rightIsNumeric = numericKinds.has(resolvedRight.kind);

        if (leftIsNumeric && rightIsNumeric && resolvedLeft.kind !== resolvedRight.kind) {
          this.errors.push(new TypeError(
            `Cannot mix ${typeToString(resolvedLeft)} and ${typeToString(resolvedRight)} in arithmetic. Use to_float(x) or to_int(x) for explicit conversion.`,
            expr.span,
          ));
          return [leftType, effects];
        }

        try {
          this.unifier.unify(leftType, rightType, expr.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        const resolved = this.unifier.resolve(leftType);
        if (!numericKinds.has(resolved.kind) && resolved.kind !== "var") {
          this.errors.push(new TypeError(
            `Arithmetic operator '${expr.op}' requires numeric types but got ${typeToString(resolved)}`,
            expr.span,
          ));
        }
        return [leftType, effects];
      }
      case "==": case "!=": {
        // Equality: both operands must be same type, result is Bool
        try {
          this.unifier.unify(leftType, rightType, expr.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        return [BOOL, effects];
      }
      case "<": case ">": case "<=": case ">=": {
        // Comparison: both operands must be same type, result is Bool
        try {
          this.unifier.unify(leftType, rightType, expr.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        return [BOOL, effects];
      }
      case "&&": case "||": {
        // Logical: both operands must be Bool, result is Bool
        try {
          this.unifier.unify(leftType, BOOL, expr.left.span);
          this.unifier.unify(rightType, BOOL, expr.right.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        return [BOOL, effects];
      }
      case "++": {
        // Concatenation: String ++ String -> String or List ++ List -> List
        try {
          this.unifier.unify(leftType, rightType, expr.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        return [leftType, effects];
      }
      default:
        this.errors.push(new TypeError(`Unknown operator '${expr.op}'`, expr.span));
        return [freshVar(), effects];
    }
  }

  private inferUnaryOp(expr: Extract<AST.Expr, { kind: "unaryop" }>): [Type, EffectRow] {
    const [operandType, operandEff] = this.inferExpr(expr.operand);

    switch (expr.op) {
      case "-": {
        const resolved = this.unifier.resolve(operandType);
        if (resolved.kind !== "int" && resolved.kind !== "float" && resolved.kind !== "byte" && resolved.kind !== "var") {
          this.errors.push(new TypeError(
            `Unary '-' requires numeric type but got ${typeToString(resolved)}`,
            expr.span,
          ));
        }
        return [operandType, operandEff];
      }
      case "!": {
        try {
          this.unifier.unify(operandType, BOOL, expr.span);
        } catch (e) {
          if (e instanceof TypeError) this.errors.push(e);
        }
        return [BOOL, operandEff];
      }
      default:
        this.errors.push(new TypeError(`Unknown unary operator '${expr.op}'`, expr.span));
        return [freshVar(), operandEff];
    }
  }

  private inferRecord(expr: Extract<AST.Expr, { kind: "record" }>): [Type, EffectRow] {
    const fields = new Map<string, Type>();
    let effects: EffectRow = PURE;

    for (const [name, value] of expr.fields) {
      const [valType, valEff] = this.inferExpr(value);
      fields.set(name, valType);
      effects = this.mergeEffects(effects, valEff);
    }

    return [{ kind: "record", fields }, effects];
  }

  private inferFieldAccess(expr: Extract<AST.Expr, { kind: "field_access" }>): [Type, EffectRow] {
    const [recType, recEff] = this.inferExpr(expr.expr);
    const resolved = this.unifier.resolve(recType);

    if (resolved.kind === "record") {
      const fieldType = resolved.fields.get(expr.field);
      if (fieldType) {
        return [fieldType, recEff];
      }
      this.errors.push(new TypeError(
        `Record has no field '${expr.field}'`,
        expr.span,
      ));
      return [freshVar(), recEff];
    }

    if (resolved.kind === "var") {
      // Create a record type with the accessed field and a row variable
      const fieldType = freshVar();
      const row = freshVar();
      const rowId = row.kind === "var" ? row.id : undefined;
      const recFields = new Map<string, Type>();
      recFields.set(expr.field, fieldType);
      const expectedRec: Type = { kind: "record", fields: recFields, row: rowId };
      try {
        this.unifier.unify(recType, expectedRec, expr.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      return [fieldType, recEff];
    }

    this.errors.push(new TypeError(
      `Cannot access field '${expr.field}' on type ${typeToString(resolved)}`,
      expr.span,
    ));
    return [freshVar(), recEff];
  }

  private inferRecordUpdate(expr: Extract<AST.Expr, { kind: "record_update" }>): [Type, EffectRow] {
    const [recType, recEff] = this.inferExpr(expr.record);
    let effects = recEff;

    for (const [name, value] of expr.fields) {
      const [valType, valEff] = this.inferExpr(value);
      effects = this.mergeEffects(effects, valEff);
    }

    return [recType, effects]; // Record update preserves the record's type
  }

  private inferList(expr: Extract<AST.Expr, { kind: "list" }>): [Type, EffectRow] {
    if (expr.elements.length === 0) {
      return [{ kind: "list", element: freshVar() }, PURE];
    }

    const [firstType, firstEff] = this.inferExpr(expr.elements[0]);
    let effects = firstEff;

    for (let i = 1; i < expr.elements.length; i++) {
      const [elemType, elemEff] = this.inferExpr(expr.elements[i]);
      try {
        this.unifier.unify(elemType, firstType, expr.elements[i].span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      effects = this.mergeEffects(effects, elemEff);
    }

    return [{ kind: "list", element: firstType }, effects];
  }

  private inferBlock(expr: Extract<AST.Expr, { kind: "block" }>): [Type, EffectRow] {
    if (expr.exprs.length === 0) {
      return [UNIT, PURE];
    }

    let effects: EffectRow = PURE;
    let lastType: Type = UNIT;

    for (const e of expr.exprs) {
      const [t, eff] = this.inferExpr(e);
      lastType = t;
      effects = this.mergeEffects(effects, eff);
    }

    return [lastType, effects];
  }

  private inferSpawn(expr: Extract<AST.Expr, { kind: "spawn" }>): [Type, EffectRow] {
    const [innerType, _innerEff] = this.inferExpr(expr.expr);
    const msgType = freshVar();

    // The spawned expression should produce a Process type
    // For now, we just return Pid with a fresh message type
    const pidType: Type = { kind: "pid", msg: msgType };
    const processEffect: EffectRow = { effects: new Set(["process" as Effect]), open: false };
    return [pidType, processEffect];
  }

  private inferSend(expr: Extract<AST.Expr, { kind: "send" }>): [Type, EffectRow] {
    const [targetType, targetEff] = this.inferExpr(expr.target);
    const [msgType, msgEff] = this.inferExpr(expr.message);

    // Target must be Pid<M>, and message must be M
    const expectedPid: Type = { kind: "pid", msg: msgType };
    try {
      this.unifier.unify(targetType, expectedPid, expr.target.span);
    } catch (e) {
      if (e instanceof TypeError) this.errors.push(e);
    }

    const effects = this.mergeEffects(
      this.mergeEffects(targetEff, msgEff),
      { effects: new Set(["process" as Effect]), open: false },
    );
    return [UNIT, effects];
  }

  private inferReceive(expr: Extract<AST.Expr, { kind: "receive" }>): [Type, EffectRow] {
    // receive { arms } - each arm matches against the message type
    const msgType = freshVar();
    const resultType = freshVar();
    let effects: EffectRow = { effects: new Set(["process" as Effect]), open: false };

    for (const arm of expr.arms) {
      this.env.pushScope();
      this.inferPattern(arm.pattern, msgType);
      const [armType, armEff] = this.inferExpr(arm.body);
      try {
        this.unifier.unify(armType, resultType, arm.body.span);
      } catch (e) {
        if (e instanceof TypeError) this.errors.push(e);
      }
      effects = this.mergeEffects(effects, armEff);
      this.env.popScope();
    }

    return [resultType, effects];
  }

  private inferTry(expr: Extract<AST.Expr, { kind: "try" }>): [Type, EffectRow] {
    const [innerType, innerEff] = this.inferExpr(expr.expr);

    // expr must be Result<Ok, Err>, and try unwraps to Ok, adding Fail effect
    const okType = freshVar();
    const errType = freshVar();
    const expectedResult: Type = { kind: "result", ok: okType, err: errType };

    try {
      this.unifier.unify(innerType, expectedResult, expr.span);
    } catch (e) {
      if (e instanceof TypeError) this.errors.push(e);
    }

    const effects = this.mergeEffects(innerEff, { effects: new Set(["fail" as Effect]), open: false });
    return [okType, effects];
  }

  private inferReturn(expr: Extract<AST.Expr, { kind: "return" }>): [Type, EffectRow] {
    if (expr.expr) {
      const [retType, retEff] = this.inferExpr(expr.expr);
      return [NEVER, retEff]; // return never continues
    }
    return [NEVER, PURE];
  }

  // ─── Type Expression Lowering ───

  lowerTypeExpr(te: AST.TypeExpr): Type {
    return this.lowerTypeExprWithParams(te, new Map());
  }

  private lowerTypeExprWithParams(te: AST.TypeExpr, typeParams: Map<string, Type>): Type {
    switch (te.kind) {
      case "tnamed": {
        // Check for built-in types
        switch (te.name) {
          case "Int": return INT;
          case "Float": return FLOAT;
          case "Byte": return BYTE;
          case "String": return STRING;
          case "Bool": return BOOL;
          case "Unit": return UNIT;
          case "Never": return NEVER;
          case "List": {
            if (te.args.length === 1) {
              return { kind: "list", element: this.lowerTypeExprWithParams(te.args[0], typeParams) };
            }
            break;
          }
          case "Option": {
            if (te.args.length === 1) {
              return { kind: "option", some: this.lowerTypeExprWithParams(te.args[0], typeParams) };
            }
            break;
          }
          case "Result": {
            if (te.args.length === 2) {
              return {
                kind: "result",
                ok: this.lowerTypeExprWithParams(te.args[0], typeParams),
                err: this.lowerTypeExprWithParams(te.args[1], typeParams),
              };
            }
            break;
          }
          case "Process": {
            if (te.args.length === 1) {
              return { kind: "process", msg: this.lowerTypeExprWithParams(te.args[0], typeParams) };
            }
            return { kind: "process", msg: UNIT };
          }
          case "Pid": {
            if (te.args.length === 1) {
              return { kind: "pid", msg: this.lowerTypeExprWithParams(te.args[0], typeParams) };
            }
            break;
          }
        }
        // User-defined type
        const args = te.args.map(a => this.lowerTypeExprWithParams(a, typeParams));
        return { kind: "named", name: te.name, args };
      }
      case "tvar": {
        const mapped = typeParams.get(te.name);
        if (mapped) return mapped;
        return freshVar();
      }
      case "tfn": {
        const params = te.params.map(p => this.lowerTypeExprWithParams(p, typeParams));
        const ret = this.lowerTypeExprWithParams(te.ret, typeParams);
        return { kind: "fn", params, ret, effects: { effects: new Set(), open: true } };
      }
      case "trecord": {
        const fields = new Map<string, Type>();
        for (const [name, typeExpr] of te.fields) {
          fields.set(name, this.lowerTypeExprWithParams(typeExpr, typeParams));
        }
        return { kind: "record", fields };
      }
      case "ttuple": {
        const elements = te.elements.map(e => this.lowerTypeExprWithParams(e, typeParams));
        return { kind: "tuple", elements };
      }
      case "tunit":
        return UNIT;
    }
  }

  private lowerEffectExpr(ee: AST.EffectExpr): EffectRow {
    const effects = new Set<Effect>();
    for (const eff of ee.effects) {
      const lower = eff.toLowerCase();
      if (lower === "io" || lower === "async" || lower === "process" || lower === "fail" || lower === "pure") {
        effects.add(lower as Effect);
      }
    }
    return { effects, open: false };
  }

  // ─── Instantiation & Generalization ───

  instantiate(scheme: TypeScheme): Type {
    if (scheme.vars.length === 0) return scheme.type;
    const mapping = new Map<number, Type>();
    for (const v of scheme.vars) {
      mapping.set(v, freshVar());
    }
    return this.substituteType(scheme.type, mapping);
  }

  private instantiateType(t: Type, mapping: Map<number, Type>): Type {
    // For constructor types that use negative ids as placeholders
    switch (t.kind) {
      case "var": {
        const existing = mapping.get(t.id);
        if (existing) return existing;
        const fresh = freshVar();
        mapping.set(t.id, fresh);
        return fresh;
      }
      case "fn":
        return {
          kind: "fn",
          params: t.params.map(p => this.instantiateType(p, mapping)),
          ret: this.instantiateType(t.ret, mapping),
          effects: t.effects,
        };
      case "named":
        return {
          kind: "named",
          name: t.name,
          args: t.args.map(a => this.instantiateType(a, mapping)),
        };
      case "record": {
        const fields = new Map<string, Type>();
        for (const [k, v] of t.fields) {
          fields.set(k, this.instantiateType(v, mapping));
        }
        return { kind: "record", fields, row: t.row };
      }
      case "tuple":
        return { kind: "tuple", elements: t.elements.map(e => this.instantiateType(e, mapping)) };
      case "list":
        return { kind: "list", element: this.instantiateType(t.element, mapping) };
      case "process":
        return { kind: "process", msg: this.instantiateType(t.msg, mapping) };
      case "pid":
        return { kind: "pid", msg: this.instantiateType(t.msg, mapping) };
      case "result":
        return {
          kind: "result",
          ok: this.instantiateType(t.ok, mapping),
          err: this.instantiateType(t.err, mapping),
        };
      case "option":
        return { kind: "option", some: this.instantiateType(t.some, mapping) };
      default:
        return t;
    }
  }

  private substituteType(t: Type, mapping: Map<number, Type>): Type {
    switch (t.kind) {
      case "var": {
        const replacement = mapping.get(t.id);
        return replacement ?? t;
      }
      case "fn":
        return {
          kind: "fn",
          params: t.params.map(p => this.substituteType(p, mapping)),
          ret: this.substituteType(t.ret, mapping),
          effects: t.effects,
        };
      case "named":
        return {
          kind: "named",
          name: t.name,
          args: t.args.map(a => this.substituteType(a, mapping)),
        };
      case "record": {
        const fields = new Map<string, Type>();
        for (const [k, v] of t.fields) {
          fields.set(k, this.substituteType(v, mapping));
        }
        const row = t.row !== undefined && mapping.has(t.row) ? undefined : t.row;
        return { kind: "record", fields, row };
      }
      case "tuple":
        return { kind: "tuple", elements: t.elements.map(e => this.substituteType(e, mapping)) };
      case "list":
        return { kind: "list", element: this.substituteType(t.element, mapping) };
      case "process":
        return { kind: "process", msg: this.substituteType(t.msg, mapping) };
      case "pid":
        return { kind: "pid", msg: this.substituteType(t.msg, mapping) };
      case "result":
        return {
          kind: "result",
          ok: this.substituteType(t.ok, mapping),
          err: this.substituteType(t.err, mapping),
        };
      case "option":
        return { kind: "option", some: this.substituteType(t.some, mapping) };
      default:
        return t;
    }
  }

  generalize(type: Type): TypeScheme {
    // Collect free variables in the type that are not bound in the environment
    const free = freeVars(type);
    // For simplicity, generalize all free vars
    // A more sophisticated version would subtract env's free vars
    return { vars: [...free], type };
  }

  mergeEffects(a: EffectRow, b: EffectRow): EffectRow {
    const merged = new Set<Effect>([...a.effects, ...b.effects]);
    return { effects: merged, open: a.open || b.open };
  }
}

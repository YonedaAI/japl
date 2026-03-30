// ─── C Code Generator ───
// Takes JAPL IR and produces C source code targeting the JAPL C runtime.
// Set of JAPL builtin function names that map to runtime builtins
const BUILTIN_FNS = {
    println: 'japl_builtin_println',
    print: 'japl_builtin_print',
    show: 'japl_builtin_show',
    int_to_string: 'japl_builtin_int_to_string',
    string_length: 'japl_builtin_string_length',
};
export class CEmitter {
    output = [];
    indent = 0;
    functionDecls = []; // forward declarations
    closureCount = 0; // for generating unique closure names
    lambdaLifts = []; // lifted lambda functions
    nameCounter = 0;
    knownFunctions = new Set(); // top-level fn names
    hasMain = false;
    foreignIncludes = new Set();
    emit(module) {
        this.output = [];
        this.indent = 0;
        this.functionDecls = [];
        this.closureCount = 0;
        this.lambdaLifts = [];
        this.nameCounter = 0;
        this.knownFunctions = new Set();
        this.hasMain = false;
        this.foreignIncludes = new Set();
        // First pass: collect known function names and foreign includes
        for (const decl of module.decls) {
            if (decl.kind === 'fn') {
                this.knownFunctions.add(decl.name);
                if (decl.name === 'main') {
                    this.hasMain = true;
                }
            }
            if (decl.kind === 'foreign' && decl.module) {
                // For C backend, module is a header file (e.g., "stdio.h")
                this.foreignIncludes.add(decl.module);
                // Register the function name as known so it can be called
                this.knownFunctions.add(decl.name);
            }
        }
        // Emit includes
        this.emitIncludes();
        this.output.push('');
        // Collect all lifted lambdas and function bodies first (dry run for lambdas)
        // We need forward declarations for all functions
        for (const decl of module.decls) {
            if (decl.kind === 'fn') {
                this.functionDecls.push(`JaplValue japl_fn_${decl.name}(JaplValue* args, int argc, JaplValue* env, int envc);`);
            }
        }
        // Emit forward declarations
        this.emitForwardDecls();
        this.output.push('');
        // Emit declarations
        for (const decl of module.decls) {
            this.emitDecl(decl);
            this.output.push('');
        }
        // Emit lifted lambdas before main
        if (this.lambdaLifts.length > 0) {
            for (const lifted of this.lambdaLifts) {
                this.output.push(lifted);
                this.output.push('');
            }
        }
        // Emit C main() if JAPL has a main function
        if (this.hasMain) {
            this.line('int main(void) {');
            this.indented(() => {
                this.line('japl_runtime_init();');
                this.line('JaplValue result = japl_fn_main(NULL, 0, NULL, 0);');
                this.line('(void)result;');
                this.line('japl_runtime_shutdown();');
                this.line('return 0;');
            });
            this.line('}');
        }
        return this.output.join('\n').trimEnd() + '\n';
    }
    emitIncludes() {
        this.line('#include "japl_runtime.h"');
        this.line('#include <stdio.h>');
        this.line('#include <stdlib.h>');
        this.line('#include <string.h>');
        for (const header of this.foreignIncludes) {
            // If the module looks like a system header (e.g., "math.h"), use angle brackets
            // Otherwise use quotes
            if (header.endsWith('.h')) {
                this.line(`#include <${header}>`);
            }
            else {
                this.line(`/* foreign module: ${header} */`);
            }
        }
    }
    emitForwardDecls() {
        if (this.functionDecls.length === 0)
            return;
        this.line('/* Forward declarations */');
        for (const decl of this.functionDecls) {
            this.line(decl);
        }
    }
    emitDecl(decl) {
        switch (decl.kind) {
            case 'fn':
                this.emitFnDecl(decl);
                break;
            case 'type':
                this.emitTypeDecl(decl);
                break;
            case 'record_type':
                // Record types are structural in C runtime, no codegen needed
                this.line(`/* record type ${decl.name} — structural */`);
                break;
            case 'test':
                this.emitTestDecl(decl);
                break;
            case 'import':
                // Imports are resolved at link time in C
                this.line(`/* import ${decl.path.join('/')} */`);
                break;
            case 'foreign':
                // Foreign declarations are handled via includes
                this.line(`/* foreign: ${decl.module ? decl.module + '::' : ''}${decl.name} */`);
                break;
        }
    }
    emitFnDecl(decl) {
        this.line(`JaplValue japl_fn_${decl.name}(JaplValue* args, int argc, JaplValue* env, int envc) {`);
        this.indented(() => {
            this.line('(void)argc; (void)env; (void)envc;');
            // Bind parameters from args array
            for (let i = 0; i < decl.params.length; i++) {
                this.line(`JaplValue ${this.sanitizeName(decl.params[i])} = args[${i}];`);
            }
            // Emit body as return statement
            this.emitExprAsStatements(decl.body, true);
        });
        this.line('}');
    }
    emitTypeDecl(decl) {
        this.line(`/* type ${decl.name} — tagged union */`);
        // Type declarations don't produce C code; constructors are built with japl_tagged at use sites
    }
    emitTestDecl(decl) {
        const safeName = this.sanitizeName(decl.name.replace(/^"|"$/g, ''));
        this.line(`JaplValue japl_test_${safeName}(JaplValue* args, int argc, JaplValue* env, int envc) {`);
        this.indented(() => {
            this.line('(void)args; (void)argc; (void)env; (void)envc;');
            this.emitExprAsStatements(decl.body, true);
        });
        this.line('}');
    }
    emitExpr(expr) {
        switch (expr.kind) {
            case 'int':
                return `japl_int(${expr.value})`;
            case 'float':
                return `japl_float(${expr.value})`;
            case 'string': {
                // The lexer stores strings with quotes, e.g., "hello"
                const raw = expr.value;
                return `japl_string(${raw})`;
            }
            case 'bool':
                return `japl_bool(${expr.value ? 1 : 0})`;
            case 'unit':
                return 'japl_unit()';
            case 'var':
                return this.sanitizeName(expr.name);
            case 'let':
                return this.emitLetAsBlock(expr);
            case 'app':
                return this.emitApp(expr);
            case 'lambda':
                return this.emitLambda(expr);
            case 'if':
                return `(japl_to_bool(${this.emitExpr(expr.cond)}) ? ${this.emitExpr(expr.then)} : ${this.emitExpr(expr.else)})`;
            case 'match':
                return this.emitMatchAsBlock(expr);
            case 'binop':
                return this.emitBinop(expr);
            case 'unaryop':
                return this.emitUnaryop(expr);
            case 'concat':
                return `japl_string_concat(${this.emitExpr(expr.left)}, ${this.emitExpr(expr.right)})`;
            case 'record':
                return this.emitRecord(expr);
            case 'field_access':
                return `japl_field(${this.emitExpr(expr.expr)}, "${expr.field}")`;
            case 'record_update':
                return this.emitRecordUpdate(expr);
            case 'list':
                return this.emitList(expr);
            case 'construct':
                return this.emitConstruct(expr);
            case 'block':
                return this.emitBlockExpr(expr);
            case 'spawn':
                return this.emitSpawn(expr);
            case 'send':
                return `(japl_send(japl_to_int(${this.emitExpr(expr.pid)}), ${this.emitExpr(expr.msg)}), japl_unit())`;
            case 'receive':
                return this.emitReceive(expr);
            case 'try':
                return this.emitExpr(expr.expr);
            case 'return':
                return this.emitExpr(expr.expr);
            case 'tail_loop':
                return this.emitExpr(expr.body);
            case 'tail_continue':
                return 'japl_unit() /* tail_continue */';
        }
    }
    emitApp(expr) {
        const args = expr.args.map(a => this.emitExpr(a));
        // Check if calling a known top-level function
        if (expr.fn.kind === 'var') {
            const name = expr.fn.name;
            // Check builtins first
            if (name in BUILTIN_FNS) {
                const cName = BUILTIN_FNS[name];
                if (args.length === 0) {
                    return `${cName}(NULL, 0, NULL, 0)`;
                }
                return `${cName}((JaplValue[]){${args.join(', ')}}, ${args.length}, NULL, 0)`;
            }
            // Known JAPL function
            if (this.knownFunctions.has(name)) {
                if (args.length === 0) {
                    return `japl_fn_${name}(NULL, 0, NULL, 0)`;
                }
                const tmpArgs = this.freshName('_args');
                // Use compound literal for the args array
                return `japl_fn_${name}((JaplValue[]){${args.join(', ')}}, ${args.length}, NULL, 0)`;
            }
        }
        // General case: apply to a closure value
        if (args.length === 0) {
            return `japl_apply(${this.emitExpr(expr.fn)}, 0)`;
        }
        return `japl_apply(${this.emitExpr(expr.fn)}, ${args.length}, ${args.join(', ')})`;
    }
    emitBinop(expr) {
        const left = this.emitExpr(expr.left);
        const right = this.emitExpr(expr.right);
        const opMap = {
            '+': 'japl_add',
            '-': 'japl_sub',
            '*': 'japl_mul',
            '/': 'japl_div',
            '%': 'japl_mod',
            '==': 'japl_eq',
            '!=': 'japl_neq',
            '<': 'japl_lt',
            '>': 'japl_gt',
            '<=': 'japl_lte',
            '>=': 'japl_gte',
            '&&': 'japl_and',
            '||': 'japl_or',
        };
        const fn = opMap[expr.op];
        if (fn) {
            return `${fn}(${left}, ${right})`;
        }
        // Fallback for unknown ops
        return `/* unknown op ${expr.op} */ ${left}`;
    }
    emitUnaryop(expr) {
        const operand = this.emitExpr(expr.operand);
        switch (expr.op) {
            case '-': return `japl_negate(${operand})`;
            case '!': return `japl_not(${operand})`;
            default: return operand;
        }
    }
    emitRecord(expr) {
        if (expr.fields.length === 0) {
            return 'japl_record(0)';
        }
        const fieldParts = expr.fields.map(([k, v]) => `"${k}", ${this.emitExpr(v)}`);
        return `japl_record(${expr.fields.length}, ${fieldParts.join(', ')})`;
    }
    emitRecordUpdate(expr) {
        let result = this.emitExpr(expr.record);
        for (const [key, val] of expr.updates) {
            result = `japl_record_update(${result}, "${key}", ${this.emitExpr(val)})`;
        }
        return result;
    }
    emitList(expr) {
        let result = 'japl_nil()';
        for (let i = expr.elements.length - 1; i >= 0; i--) {
            result = `japl_cons(${this.emitExpr(expr.elements[i])}, ${result})`;
        }
        return result;
    }
    emitConstruct(expr) {
        if (expr.args.length === 0) {
            return `japl_tagged("${expr.tag}", 0)`;
        }
        const args = expr.args.map(a => this.emitExpr(a));
        return `japl_tagged("${expr.tag}", ${expr.args.length}, ${args.join(', ')})`;
    }
    emitLambda(expr) {
        const lambdaName = this.freshName('_lambda');
        const params = expr.params;
        // Collect free variables for closure capture
        const freeVars = this.collectFreeVars(expr.body, new Set(params));
        // Build the lifted function
        const lines = [];
        lines.push(`JaplValue ${lambdaName}(JaplValue* args, int argc, JaplValue* env, int envc) {`);
        lines.push('    (void)argc; (void)env; (void)envc;');
        for (let i = 0; i < params.length; i++) {
            lines.push(`    JaplValue ${this.sanitizeName(params[i])} = args[${i}];`);
        }
        for (let i = 0; i < freeVars.length; i++) {
            lines.push(`    JaplValue ${this.sanitizeName(freeVars[i])} = env[${i}];`);
        }
        // Emit body
        const savedOutput = this.output;
        const savedIndent = this.indent;
        this.output = [];
        this.indent = 1;
        this.emitExprAsStatements(expr.body, true);
        const bodyLines = this.output;
        this.output = savedOutput;
        this.indent = savedIndent;
        lines.push(...bodyLines);
        lines.push('}');
        this.lambdaLifts.push(lines.join('\n'));
        // At use site, create closure
        if (freeVars.length === 0) {
            return `japl_closure(&${lambdaName}, ${params.length}, 0)`;
        }
        const envArgs = freeVars.map(v => this.sanitizeName(v));
        return `japl_closure(&${lambdaName}, ${params.length}, ${freeVars.length}, ${envArgs.join(', ')})`;
    }
    emitSpawn(expr) {
        if (expr.fn.kind === 'var' && this.knownFunctions.has(expr.fn.name)) {
            return `japl_pid(japl_spawn(&japl_fn_${expr.fn.name}, japl_unit()))`;
        }
        // For closure spawns, we'd need the fn pointer from the closure
        return `japl_pid(japl_spawn(NULL, japl_unit())) /* TODO: closure spawn */`;
    }
    emitReceive(expr) {
        // This is complex enough we need statement-level emission
        // Use a GCC statement expression or helper
        const msgName = this.freshName('_msg');
        const resultName = this.freshName('_recv_result');
        // For expression context, we can't easily do this without statement expressions
        // Use a GCC extension ({...}) block
        const parts = [];
        parts.push(`({`);
        parts.push(`    JaplValue ${msgName} = japl_receive();`);
        parts.push(`    JaplValue ${resultName} = japl_unit();`);
        for (let i = 0; i < expr.arms.length; i++) {
            const arm = expr.arms[i];
            const { condition, bindings } = this.emitPattern(arm.pattern, msgName);
            const prefix = i === 0 ? 'if' : '} else if';
            parts.push(`    ${prefix} (${condition}) {`);
            for (const b of bindings) {
                parts.push(`        ${b}`);
            }
            parts.push(`        ${resultName} = ${this.emitExpr(arm.body)};`);
        }
        if (expr.arms.length > 0) {
            parts.push('    }');
        }
        parts.push(`    ${resultName};`);
        parts.push(`})`);
        return parts.join('\n');
    }
    emitMatchAsBlock(expr) {
        // Use GCC statement expression for match in expression position
        const scrName = this.freshName('_match');
        const resultName = this.freshName('_match_result');
        const parts = [];
        parts.push('({');
        parts.push(`    JaplValue ${scrName} = ${this.emitExpr(expr.scrutinee)};`);
        parts.push(`    JaplValue ${resultName} = japl_unit();`);
        for (let i = 0; i < expr.arms.length; i++) {
            const arm = expr.arms[i];
            const { condition, bindings } = this.emitPattern(arm.pattern, scrName);
            const prefix = i === 0 ? 'if' : '} else if';
            parts.push(`    ${prefix} (${condition}) {`);
            for (const b of bindings) {
                parts.push(`        ${b}`);
            }
            parts.push(`        ${resultName} = ${this.emitExpr(arm.body)};`);
        }
        if (expr.arms.length > 0) {
            parts.push('    }');
        }
        parts.push(`    ${resultName};`);
        parts.push('})');
        return parts.join('\n');
    }
    emitLetAsBlock(expr) {
        // Use GCC statement expression
        const parts = [];
        parts.push('({');
        parts.push(`    JaplValue ${this.sanitizeName(expr.name)} = ${this.emitExpr(expr.value)};`);
        parts.push(`    ${this.emitExpr(expr.body)};`);
        parts.push('})');
        return parts.join('\n');
    }
    emitBlockExpr(expr) {
        const parts = [];
        parts.push('({');
        for (let i = 0; i < expr.exprs.length; i++) {
            const e = expr.exprs[i];
            if (i === expr.exprs.length - 1) {
                parts.push(`    ${this.emitExpr(e)};`);
            }
            else if (e.kind === 'let') {
                parts.push(`    JaplValue ${this.sanitizeName(e.name)} = ${this.emitExpr(e.value)};`);
                // Flatten remaining let body into the block
            }
            else {
                parts.push(`    ${this.emitExpr(e)};`);
            }
        }
        parts.push('})');
        return parts.join('\n');
    }
    // ─── Statement-level emission ───
    emitExprAsStatements(expr, isReturn) {
        switch (expr.kind) {
            case 'let':
                this.emitLetChain(expr, isReturn);
                break;
            case 'block':
                for (let i = 0; i < expr.exprs.length; i++) {
                    const isLast = i === expr.exprs.length - 1;
                    this.emitExprAsStatements(expr.exprs[i], isLast && isReturn);
                }
                break;
            case 'if':
                this.emitIfStatements(expr, isReturn);
                break;
            case 'match':
                this.emitMatchStatements(expr.scrutinee, expr.arms, isReturn);
                break;
            case 'return':
                this.line(`return ${this.emitExpr(expr.expr)};`);
                break;
            default:
                if (isReturn) {
                    this.line(`return ${this.emitExpr(expr)};`);
                }
                else {
                    this.line(`${this.emitExpr(expr)};`);
                }
                break;
        }
    }
    emitLetChain(expr, isReturn) {
        this.line(`JaplValue ${this.sanitizeName(expr.name)} = ${this.emitExpr(expr.value)};`);
        this.emitExprAsStatements(expr.body, isReturn);
    }
    emitIfStatements(expr, isReturn) {
        if (isReturn && this.isSimpleExpr(expr.then) && this.isSimpleExpr(expr.else)) {
            this.line(`return japl_to_bool(${this.emitExpr(expr.cond)}) ? ${this.emitExpr(expr.then)} : ${this.emitExpr(expr.else)};`);
        }
        else {
            this.line(`if (japl_to_bool(${this.emitExpr(expr.cond)})) {`);
            this.indented(() => {
                this.emitExprAsStatements(expr.then, isReturn);
            });
            this.line('} else {');
            this.indented(() => {
                this.emitExprAsStatements(expr.else, isReturn);
            });
            this.line('}');
        }
    }
    emitMatchStatements(scrutinee, arms, isReturn) {
        const scrName = this.freshName('_match');
        this.line(`JaplValue ${scrName} = ${this.emitExpr(scrutinee)};`);
        for (let i = 0; i < arms.length; i++) {
            const arm = arms[i];
            const { condition, bindings } = this.emitPattern(arm.pattern, scrName);
            const prefix = i === 0 ? 'if' : '} else if';
            if (condition === 'true' && i === arms.length - 1 && i > 0) {
                this.line('} else {');
            }
            else if (i === 0) {
                this.line(`if (${condition}) {`);
            }
            else {
                this.line(`${prefix} (${condition}) {`);
            }
            this.indented(() => {
                for (const b of bindings) {
                    this.line(b);
                }
                if (arm.guard) {
                    this.line(`if (japl_to_bool(${this.emitExpr(arm.guard)})) {`);
                    this.indented(() => {
                        this.emitExprAsStatements(arm.body, isReturn);
                    });
                    this.line('}');
                }
                else {
                    this.emitExprAsStatements(arm.body, isReturn);
                }
            });
        }
        if (arms.length > 0) {
            this.line('}');
        }
    }
    emitPattern(pat, scrutinee) {
        switch (pat.kind) {
            case 'pvar':
                return {
                    condition: 'true',
                    bindings: [`JaplValue ${this.sanitizeName(pat.name)} = ${scrutinee};`],
                };
            case 'pwildcard':
                return { condition: 'true', bindings: [] };
            case 'pconstructor': {
                const cond = `strcmp(japl_get_tag(${scrutinee}), "${pat.tag}") == 0`;
                const bindings = [];
                for (let i = 0; i < pat.args.length; i++) {
                    const arg = pat.args[i];
                    if (arg.kind === 'pvar') {
                        bindings.push(`JaplValue ${this.sanitizeName(arg.name)} = japl_get_field(${scrutinee}, ${i});`);
                    }
                    else if (arg.kind === 'pwildcard') {
                        // No binding needed
                    }
                    // Nested patterns would need recursive handling
                }
                return { condition: cond, bindings };
            }
            case 'pliteral': {
                const litVal = this.emitExpr(pat.value);
                return {
                    condition: `japl_to_bool(japl_eq(${scrutinee}, ${litVal}))`,
                    bindings: [],
                };
            }
            case 'plist': {
                if (pat.elements.length === 0 && !pat.rest) {
                    return { condition: `${scrutinee}.kind == JAPL_NIL`, bindings: [] };
                }
                const bindings = [];
                let current = scrutinee;
                for (let i = 0; i < pat.elements.length; i++) {
                    const el = pat.elements[i];
                    if (el.kind === 'pvar') {
                        bindings.push(`JaplValue ${this.sanitizeName(el.name)} = japl_head(${current});`);
                    }
                    if (i < pat.elements.length - 1 || pat.rest) {
                        const tailName = this.freshName('_tail');
                        bindings.push(`JaplValue ${tailName} = japl_tail(${current});`);
                        current = tailName;
                    }
                }
                if (pat.rest) {
                    bindings.push(`JaplValue ${this.sanitizeName(pat.rest)} = ${current};`);
                }
                return {
                    condition: `japl_list_length(${scrutinee}) >= ${pat.elements.length}`,
                    bindings,
                };
            }
        }
    }
    // ─── Free variable collection for closure capture ───
    collectFreeVars(expr, bound) {
        const free = new Set();
        this.collectFreeVarsImpl(expr, bound, free);
        return [...free];
    }
    collectFreeVarsImpl(expr, bound, free) {
        switch (expr.kind) {
            case 'var':
                if (!bound.has(expr.name) && !this.knownFunctions.has(expr.name) && !(expr.name in BUILTIN_FNS)) {
                    free.add(expr.name);
                }
                break;
            case 'let': {
                this.collectFreeVarsImpl(expr.value, bound, free);
                const newBound = new Set(bound);
                newBound.add(expr.name);
                this.collectFreeVarsImpl(expr.body, newBound, free);
                break;
            }
            case 'lambda': {
                const newBound = new Set(bound);
                for (const p of expr.params)
                    newBound.add(p);
                this.collectFreeVarsImpl(expr.body, newBound, free);
                break;
            }
            case 'app':
                this.collectFreeVarsImpl(expr.fn, bound, free);
                for (const a of expr.args)
                    this.collectFreeVarsImpl(a, bound, free);
                break;
            case 'if':
                this.collectFreeVarsImpl(expr.cond, bound, free);
                this.collectFreeVarsImpl(expr.then, bound, free);
                this.collectFreeVarsImpl(expr.else, bound, free);
                break;
            case 'match':
                this.collectFreeVarsImpl(expr.scrutinee, bound, free);
                for (const arm of expr.arms) {
                    const armBound = new Set(bound);
                    this.collectPatternBindings(arm.pattern, armBound);
                    this.collectFreeVarsImpl(arm.body, armBound, free);
                }
                break;
            case 'binop':
                this.collectFreeVarsImpl(expr.left, bound, free);
                this.collectFreeVarsImpl(expr.right, bound, free);
                break;
            case 'concat':
                this.collectFreeVarsImpl(expr.left, bound, free);
                this.collectFreeVarsImpl(expr.right, bound, free);
                break;
            case 'unaryop':
                this.collectFreeVarsImpl(expr.operand, bound, free);
                break;
            case 'record':
                for (const [, v] of expr.fields)
                    this.collectFreeVarsImpl(v, bound, free);
                break;
            case 'field_access':
                this.collectFreeVarsImpl(expr.expr, bound, free);
                break;
            case 'record_update':
                this.collectFreeVarsImpl(expr.record, bound, free);
                for (const [, v] of expr.updates)
                    this.collectFreeVarsImpl(v, bound, free);
                break;
            case 'list':
                for (const e of expr.elements)
                    this.collectFreeVarsImpl(e, bound, free);
                break;
            case 'construct':
                for (const a of expr.args)
                    this.collectFreeVarsImpl(a, bound, free);
                break;
            case 'block':
                for (const e of expr.exprs)
                    this.collectFreeVarsImpl(e, bound, free);
                break;
            case 'spawn':
                this.collectFreeVarsImpl(expr.fn, bound, free);
                break;
            case 'send':
                this.collectFreeVarsImpl(expr.pid, bound, free);
                this.collectFreeVarsImpl(expr.msg, bound, free);
                break;
            case 'receive':
                for (const arm of expr.arms)
                    this.collectFreeVarsImpl(arm.body, bound, free);
                break;
            case 'try':
                this.collectFreeVarsImpl(expr.expr, bound, free);
                break;
            case 'return':
                this.collectFreeVarsImpl(expr.expr, bound, free);
                break;
            case 'int':
            case 'float':
            case 'string':
            case 'bool':
            case 'unit':
                break;
        }
    }
    collectPatternBindings(pat, bound) {
        switch (pat.kind) {
            case 'pvar':
                bound.add(pat.name);
                break;
            case 'pconstructor':
                for (const arg of pat.args)
                    this.collectPatternBindings(arg, bound);
                break;
            case 'plist':
                for (const el of pat.elements)
                    this.collectPatternBindings(el, bound);
                if (pat.rest)
                    bound.add(pat.rest);
                break;
            case 'pliteral':
            case 'pwildcard':
                break;
        }
    }
    // ─── Helpers ───
    isSimpleExpr(expr) {
        switch (expr.kind) {
            case 'int':
            case 'float':
            case 'string':
            case 'bool':
            case 'unit':
            case 'var':
                return true;
            case 'binop':
            case 'concat':
                return this.isSimpleExpr(expr.left) && this.isSimpleExpr(expr.right);
            case 'unaryop':
                return this.isSimpleExpr(expr.operand);
            case 'construct':
                return expr.args.every(a => this.isSimpleExpr(a));
            default:
                return false;
        }
    }
    sanitizeName(name) {
        return name.replace(/[^a-zA-Z0-9_]/g, '_');
    }
    line(s) {
        this.output.push(this.indentStr() + s);
    }
    indentStr() {
        return '    '.repeat(this.indent);
    }
    indented(fn) {
        this.indent++;
        fn();
        this.indent--;
    }
    freshName(prefix) {
        return `${prefix}_${this.nameCounter++}`;
    }
}
//# sourceMappingURL=emit_c.js.map
// ─── TypeScript Code Generator ───
// Takes JAPL IR and produces clean, readable TypeScript source.
export class TsEmitter {
    output = [];
    indent = 0;
    usedRuntimeImports = new Set();
    matchCounter = 0;
    foreignImports = new Map();
    foreignBuiltinNames = new Set();
    importRewrites = new Map();
    isModuleBuild = false;
    currentTcoParams = [];
    locallyDefinedConstructors = new Set();
    emit(module) {
        this.output = [];
        this.indent = 0;
        this.usedRuntimeImports.clear();
        this.foreignImports.clear();
        this.foreignBuiltinNames.clear();
        this.locallyDefinedConstructors.clear();
        // Pre-pass: collect locally defined constructor names from type decls
        for (const decl of module.decls) {
            if (decl.kind === "type") {
                for (const v of decl.variants) {
                    this.locallyDefinedConstructors.add(v.name);
                }
            }
        }
        // First pass: collect runtime imports and foreign imports
        for (const decl of module.decls) {
            this.scanImports(decl);
            if (decl.kind === "foreign") {
                if (decl.module) {
                    if (!this.foreignImports.has(decl.module)) {
                        this.foreignImports.set(decl.module, []);
                    }
                    this.foreignImports.get(decl.module).push({
                        localName: decl.name,
                        jsName: decl.jsName,
                    });
                }
                else {
                    // No module = builtin, track name so we don't redefine it
                    this.foreignBuiltinNames.add(decl.name);
                }
            }
        }
        // Emit runtime imports
        this.emitRuntimeImports();
        // Emit foreign module imports (grouped by module)
        this.emitForeignImports();
        // Emit builtin helpers
        this.line("const println = (...args: any[]) => console.log(...args);");
        this.line("const print = (...args: any[]) => process.stdout.write(args.join(''));");
        this.line("const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);");
        this.line("const int_to_string = (n: number): string => String(n);");
        this.line("const string_length = (s: string): number => s.length;");
        this.output.push("");
        // Emit declarations
        let hasMain = false;
        for (const decl of module.decls) {
            this.emitDecl(decl);
            this.output.push("");
            if (decl.kind === "fn" && decl.name === "main")
                hasMain = true;
        }
        // Auto-call main() if defined
        if (hasMain) {
            this.line("main();");
        }
        return this.output.join("\n").trimEnd() + "\n";
    }
    emitModule(module, options) {
        this.output = [];
        this.indent = 0;
        this.usedRuntimeImports.clear();
        this.foreignImports.clear();
        this.foreignBuiltinNames.clear();
        this.locallyDefinedConstructors.clear();
        this.importRewrites = options.importRewrites;
        this.isModuleBuild = true;
        // Pre-pass: collect locally defined constructor names from type decls
        for (const decl of module.decls) {
            if (decl.kind === "type") {
                for (const v of decl.variants) {
                    this.locallyDefinedConstructors.add(v.name);
                }
            }
        }
        // First pass: collect runtime imports and foreign imports
        for (const decl of module.decls) {
            this.scanImports(decl);
            if (decl.kind === "foreign") {
                if (decl.module) {
                    if (!this.foreignImports.has(decl.module)) {
                        this.foreignImports.set(decl.module, []);
                    }
                    this.foreignImports.get(decl.module).push({
                        localName: decl.name,
                        jsName: decl.jsName,
                    });
                }
                else {
                    this.foreignBuiltinNames.add(decl.name);
                }
            }
        }
        // Emit runtime imports
        this.emitRuntimeImports();
        // Emit foreign module imports
        this.emitForeignImports();
        // Emit builtin helpers
        this.line("const println = (...args: any[]) => console.log(...args);");
        this.line("const print = (...args: any[]) => process.stdout.write(args.join(''));");
        this.line("const show = (v: any): string => typeof v === 'string' ? v : JSON.stringify(v);");
        this.line("const int_to_string = (n: number): string => String(n);");
        this.line("const string_length = (s: string): number => s.length;");
        this.output.push("");
        // Emit declarations
        let hasMain = false;
        for (const decl of module.decls) {
            this.emitDecl(decl);
            this.output.push("");
            if (decl.kind === "fn" && decl.name === "main")
                hasMain = true;
        }
        // Auto-call main() only in entry file
        if (hasMain && options.isEntry) {
            this.line("main();");
        }
        this.isModuleBuild = false;
        return this.output.join("\n").trimEnd() + "\n";
    }
    scanImports(decl) {
        switch (decl.kind) {
            case "fn":
                this.scanExprImports(decl.body);
                break;
            case "test":
                this.scanExprImports(decl.body);
                break;
            case "type":
            case "record_type":
            case "import":
            case "foreign":
                break;
        }
    }
    scanExprImports(expr) {
        switch (expr.kind) {
            case "spawn":
                this.usedRuntimeImports.add("spawn");
                this.scanExprImports(expr.fn);
                break;
            case "send":
                this.usedRuntimeImports.add("send");
                this.scanExprImports(expr.pid);
                this.scanExprImports(expr.msg);
                break;
            case "receive":
                this.usedRuntimeImports.add("receive");
                for (const arm of expr.arms) {
                    this.scanExprImports(arm.body);
                }
                break;
            case "construct":
                if (!this.locallyDefinedConstructors.has(expr.tag)) {
                    if (expr.tag === "Ok" || expr.tag === "Err") {
                        this.usedRuntimeImports.add(expr.tag);
                    }
                    if (expr.tag === "Some" || expr.tag === "None") {
                        this.usedRuntimeImports.add(expr.tag);
                    }
                }
                for (const arg of expr.args) {
                    this.scanExprImports(arg);
                }
                break;
            case "app":
                this.scanExprImports(expr.fn);
                for (const arg of expr.args)
                    this.scanExprImports(arg);
                break;
            case "lambda":
                this.scanExprImports(expr.body);
                break;
            case "let":
                this.scanExprImports(expr.value);
                this.scanExprImports(expr.body);
                break;
            case "if":
                this.scanExprImports(expr.cond);
                this.scanExprImports(expr.then);
                this.scanExprImports(expr.else);
                break;
            case "match":
                this.scanExprImports(expr.scrutinee);
                for (const arm of expr.arms)
                    this.scanExprImports(arm.body);
                break;
            case "binop":
                this.scanExprImports(expr.left);
                this.scanExprImports(expr.right);
                break;
            case "concat":
                this.scanExprImports(expr.left);
                this.scanExprImports(expr.right);
                break;
            case "unaryop":
                this.scanExprImports(expr.operand);
                break;
            case "record":
                for (const [, v] of expr.fields)
                    this.scanExprImports(v);
                break;
            case "field_access":
                this.scanExprImports(expr.expr);
                break;
            case "record_update":
                this.scanExprImports(expr.record);
                for (const [, v] of expr.updates)
                    this.scanExprImports(v);
                break;
            case "list":
                for (const e of expr.elements)
                    this.scanExprImports(e);
                break;
            case "block":
                for (const e of expr.exprs)
                    this.scanExprImports(e);
                break;
            case "try":
                this.scanExprImports(expr.expr);
                break;
            case "return":
                this.scanExprImports(expr.expr);
                break;
            case "tail_loop":
                this.scanExprImports(expr.body);
                break;
            case "tail_continue":
                for (const arg of expr.args)
                    this.scanExprImports(arg);
                break;
            case "int":
            case "float":
            case "string":
            case "bool":
            case "unit":
            case "var":
                break;
        }
    }
    emitRuntimeImports() {
        if (this.usedRuntimeImports.size === 0)
            return;
        // Inline concurrency primitives instead of importing from @japl/runtime
        // This makes generated code self-contained — no npm install needed
        if (this.usedRuntimeImports.has("spawn") || this.usedRuntimeImports.has("send") || this.usedRuntimeImports.has("receive") || this.usedRuntimeImports.has("self")) {
            this.line("// JAPL Process Runtime (inlined)");
            this.line("const __japl_processes = new Map();");
            this.line("let __japl_pid_counter = 0;");
            this.line("let __japl_current_pid = 'main';");
        }
        if (this.usedRuntimeImports.has("spawn")) {
            this.line("function spawn(fn) {");
            this.line("  const pid = 'pid-' + (++__japl_pid_counter);");
            this.line("  const mailbox = [];");
            this.line("  const waiters = [];");
            this.line("  __japl_processes.set(pid, { mailbox, waiters });");
            this.line("  Promise.resolve().then(async () => {");
            this.line("    __japl_current_pid = pid;");
            this.line("    await fn();");
            this.line("  }).catch(e => console.error('[process ' + pid + ' crashed]', e));");
            this.line("  return pid;");
            this.line("}");
        }
        if (this.usedRuntimeImports.has("send")) {
            this.line("function send(pid, msg) {");
            this.line("  const proc = __japl_processes.get(pid);");
            this.line("  if (!proc) { console.error('send: unknown pid', pid); return; }");
            this.line("  if (proc.waiters.length > 0) { proc.waiters.shift()(msg); }");
            this.line("  else { proc.mailbox.push(msg); }");
            this.line("}");
        }
        if (this.usedRuntimeImports.has("receive")) {
            this.line("function receive() {");
            this.line("  const proc = __japl_processes.get(__japl_current_pid);");
            this.line("  if (!proc) return Promise.reject(new Error('receive: no process context'));");
            this.line("  if (proc.mailbox.length > 0) return Promise.resolve(proc.mailbox.shift());");
            this.line("  return new Promise(resolve => proc.waiters.push(resolve));");
            this.line("}");
        }
        if (this.usedRuntimeImports.has("self")) {
            this.line("function self() { return __japl_current_pid; }");
        }
        // Type constructors (Ok, Err, Some, None) are already handled by type declarations
        this.output.push("");
    }
    emitForeignImports() {
        if (this.foreignImports.size === 0)
            return;
        for (const [module, items] of this.foreignImports) {
            const importParts = items.map(item => {
                if (item.jsName) {
                    // foreign "node:fs" fn read_file as "readFileSync" → import { readFileSync as read_file }
                    return `${item.jsName} as ${item.localName}`;
                }
                return item.localName;
            });
            this.line(`import { ${importParts.join(", ")} } from '${module}';`);
        }
        this.output.push("");
    }
    emitDecl(decl) {
        switch (decl.kind) {
            case "fn":
                this.emitFnDecl(decl);
                break;
            case "type":
                this.emitTypeDecl(decl);
                break;
            case "record_type":
                this.emitRecordTypeDecl(decl);
                break;
            case "test":
                this.emitTestDecl(decl);
                break;
            case "import":
                this.emitImportDecl(decl);
                break;
            case "foreign":
                // Foreign declarations are handled via emitForeignImports()
                break;
        }
    }
    emitFnDecl(decl) {
        const exportKw = decl.exported ? "export " : "";
        const params = decl.params.join(", ");
        if (decl.body.kind === "tail_loop") {
            // TCO: emit as while(true) loop
            const tcoBody = decl.body;
            const savedParams = this.currentTcoParams;
            this.currentTcoParams = tcoBody.params;
            this.line(`${exportKw}function ${decl.name}(${params}) {`);
            this.indented(() => {
                this.line("while (true) {");
                this.indented(() => {
                    this.emitExprAsStatements(tcoBody.body, true);
                });
                this.line("}");
            });
            this.line("}");
            this.currentTcoParams = savedParams;
            return;
        }
        const bodyStr = this.emitExprAsReturn(decl.body);
        if (this.isSimpleExpr(decl.body)) {
            this.line(`${exportKw}function ${decl.name}(${params}) {`);
            this.indented(() => {
                this.line(`return ${bodyStr};`);
            });
            this.line("}");
        }
        else {
            this.line(`${exportKw}function ${decl.name}(${params}) {`);
            this.indented(() => {
                this.emitExprAsStatements(decl.body, true);
            });
            this.line("}");
        }
    }
    emitTypeDecl(decl) {
        // In module build mode, export types and constructors
        const exportKw = this.isModuleBuild ? "export " : "";
        // Emit discriminated union type
        const variants = decl.variants.map(v => {
            if (v.fields === 0) {
                return `{ _tag: "${v.name}" }`;
            }
            const fields = Array.from({ length: v.fields }, (_, i) => `_${i}: unknown`).join("; ");
            return `{ _tag: "${v.name}"; ${fields} }`;
        });
        this.line(`${exportKw}type ${decl.name} = ${variants.join(" | ")};`);
        // Emit constructor functions
        for (const v of decl.variants) {
            if (v.fields === 0) {
                this.line(`${exportKw}const ${v.name}: ${decl.name} = { _tag: "${v.name}" };`);
            }
            else {
                const params = Array.from({ length: v.fields }, (_, i) => `_${i}: unknown`);
                const fields = Array.from({ length: v.fields }, (_, i) => `_${i}`);
                this.line(`${exportKw}const ${v.name} = (${params.join(", ")}): ${decl.name} => ({ _tag: "${v.name}", ${fields.join(", ")} });`);
            }
        }
    }
    emitRecordTypeDecl(decl) {
        this.line(`type ${decl.name} = {`);
        this.indented(() => {
            for (const [name, type] of decl.fields) {
                this.line(`${name}: ${this.mapType(type)};`);
            }
        });
        this.line("};");
    }
    emitTestDecl(decl) {
        // Strip surrounding quotes from test name if present (lexer preserves them)
        const displayName = decl.name.replace(/^"|"$/g, "");
        this.line(`// test: ${displayName}`);
        this.line(`function test_${this.sanitizeName(displayName)}() {`);
        this.indented(() => {
            this.emitExprAsStatements(decl.body, false);
        });
        this.line("}");
    }
    emitImportDecl(decl) {
        // In module build mode, rewrite import paths to relative .js paths
        const moduleName = decl.path[0];
        let importPath;
        if (this.isModuleBuild && this.importRewrites.has(moduleName)) {
            importPath = this.importRewrites.get(moduleName);
        }
        else {
            importPath = decl.path.join("/");
        }
        if (decl.items.length > 0) {
            this.line(`import { ${decl.items.join(", ")} } from "${importPath}";`);
        }
        else {
            this.line(`import "${importPath}";`);
        }
    }
    // ─── Expression Emission ───
    emitExpr(expr) {
        switch (expr.kind) {
            case "int":
                return String(expr.value);
            case "float":
                return String(expr.value);
            case "string":
                // Lexer stores string values with quotes already included (e.g., "hello")
                return expr.value;
            case "bool":
                return String(expr.value);
            case "unit":
                return "undefined";
            case "var":
                return expr.name;
            case "let":
                // In expression context, use IIFE
                return this.emitLetAsIife(expr);
            case "app":
                return `${this.emitExpr(expr.fn)}(${expr.args.map(a => this.emitExpr(a)).join(", ")})`;
            case "lambda":
                return this.emitLambda(expr);
            case "if":
                if (this.isSimpleExpr(expr.then) && this.isSimpleExpr(expr.else)) {
                    return `(${this.emitExpr(expr.cond)} ? ${this.emitExpr(expr.then)} : ${this.emitExpr(expr.else)})`;
                }
                return this.emitIfAsIife(expr);
            case "match":
                return this.emitMatchAsIife(expr);
            case "binop":
                return this.emitBinop(expr);
            case "unaryop":
                return `${expr.op}${this.emitExpr(expr.operand)}`;
            case "concat":
                return `${this.emitExpr(expr.left)} + ${this.emitExpr(expr.right)}`;
            case "record":
                if (expr.fields.length === 0)
                    return "{}";
                return `{ ${expr.fields.map(([k, v]) => `${k}: ${this.emitExpr(v)}`).join(", ")} }`;
            case "field_access":
                return `${this.emitExpr(expr.expr)}.${expr.field}`;
            case "record_update":
                return `{ ...${this.emitExpr(expr.record)}, ${expr.updates.map(([k, v]) => `${k}: ${this.emitExpr(v)}`).join(", ")} }`;
            case "list":
                return `[${expr.elements.map(e => this.emitExpr(e)).join(", ")}]`;
            case "construct":
                return this.emitConstruct(expr);
            case "block":
                return this.emitBlockAsIife(expr);
            case "spawn": {
                const inner = this.emitExpr(expr.fn);
                // spawn(fn() { body }) → the fn is lowered to a lambda, pass directly
                // spawn(worker_fn) → wrap in () => worker_fn()
                if (expr.fn.kind === "lambda") {
                    return `spawn(async ${inner})`;
                }
                return `spawn(async () => ${inner})`;
            }
            case "send":
                return `send(${this.emitExpr(expr.pid)}, ${this.emitExpr(expr.msg)})`;
            case "receive":
                return this.emitReceiveAsIife(expr);
            case "try":
                return this.emitTry(expr);
            case "return":
                return `return ${this.emitExpr(expr.expr)}`;
            case "tail_loop":
                return this.emitExpr(expr.body);
            case "tail_continue":
                // In expression context, emit as IIFE (shouldn't normally happen)
                return "undefined /* tail_continue */";
        }
    }
    emitBinop(expr) {
        const left = this.emitExpr(expr.left);
        const right = this.emitExpr(expr.right);
        const op = this.mapOp(expr.op);
        return `${left} ${op} ${right}`;
    }
    mapOp(op) {
        switch (op) {
            case "==": return "===";
            case "!=": return "!==";
            default: return op;
        }
    }
    emitLambda(expr) {
        const params = expr.params.join(", ");
        if (this.isSimpleExpr(expr.body)) {
            return `(${params}) => ${this.emitExpr(expr.body)}`;
        }
        // Multi-statement lambda body
        const lines = [];
        lines.push(`(${params}) => {`);
        const saved = this.output;
        this.output = [];
        this.indent++;
        this.emitExprAsStatements(expr.body, true);
        this.indent--;
        const bodyLines = this.output;
        this.output = saved;
        lines.push(...bodyLines);
        lines.push(this.indentStr() + "}");
        return lines.join("\n");
    }
    emitConstruct(expr) {
        // Special-case Bool constructors → JavaScript true/false
        if (expr.tag === "True" && expr.args.length === 0)
            return "true";
        if (expr.tag === "False" && expr.args.length === 0)
            return "false";
        if (expr.args.length === 0) {
            return `{ _tag: "${expr.tag}" }`;
        }
        const fields = expr.args.map((a, i) => `_${i}: ${this.emitExpr(a)}`).join(", ");
        return `{ _tag: "${expr.tag}", ${fields} }`;
    }
    emitLetAsIife(expr) {
        const lines = [];
        lines.push("(() => {");
        const saved = this.output;
        this.output = [];
        this.indent++;
        this.emitLetChain(expr, true);
        this.indent--;
        lines.push(...this.output);
        lines.push(this.indentStr() + "})()");
        this.output = saved;
        return lines.join("\n");
    }
    emitIfAsIife(expr) {
        const lines = [];
        lines.push("(() => {");
        const saved = this.output;
        this.output = [];
        this.indent++;
        this.line(`if (${this.emitExpr(expr.cond)}) {`);
        this.indented(() => {
            this.emitExprAsStatements(expr.then, true);
        });
        this.line("} else {");
        this.indented(() => {
            this.emitExprAsStatements(expr.else, true);
        });
        this.line("}");
        this.indent--;
        lines.push(...this.output);
        lines.push(this.indentStr() + "})()");
        this.output = saved;
        return lines.join("\n");
    }
    emitMatchAsIife(expr) {
        const lines = [];
        lines.push("(() => {");
        const saved = this.output;
        this.output = [];
        this.indent++;
        this.emitMatchStatements(expr.scrutinee, expr.arms, true);
        this.indent--;
        lines.push(...this.output);
        lines.push(this.indentStr() + "})()");
        this.output = saved;
        return lines.join("\n");
    }
    emitBlockAsIife(expr) {
        const lines = [];
        lines.push("(() => {");
        const saved = this.output;
        this.output = [];
        this.indent++;
        for (let i = 0; i < expr.exprs.length; i++) {
            const isLast = i === expr.exprs.length - 1;
            this.emitExprAsStatements(expr.exprs[i], isLast);
        }
        this.indent--;
        lines.push(...this.output);
        lines.push(this.indentStr() + "})()");
        this.output = saved;
        return lines.join("\n");
    }
    emitReceiveAsIife(expr) {
        const lines = [];
        lines.push("(async () => {");
        const saved = this.output;
        this.output = [];
        this.indent++;
        this.line("const __msg = await receive();");
        this.emitMatchStatements({ kind: "var", name: "__msg" }, expr.arms, true);
        this.indent--;
        lines.push(...this.output);
        lines.push(this.indentStr() + "})()");
        this.output = saved;
        return lines.join("\n");
    }
    emitTry(expr) {
        // try expr unwraps a Result: if Ok, returns the value; if Err, early returns
        return `${this.emitExpr(expr.expr)}`;
    }
    // ─── Statement-level emission ───
    emitExprAsStatements(expr, isReturn) {
        switch (expr.kind) {
            case "let":
                this.emitLetChain(expr, isReturn);
                break;
            case "block":
                for (let i = 0; i < expr.exprs.length; i++) {
                    const isLast = i === expr.exprs.length - 1;
                    this.emitExprAsStatements(expr.exprs[i], isLast && isReturn);
                }
                break;
            case "if":
                if (isReturn && this.isSimpleExpr(expr.then) && this.isSimpleExpr(expr.else)) {
                    this.line(`return ${this.emitExpr(expr.cond)} ? ${this.emitExpr(expr.then)} : ${this.emitExpr(expr.else)};`);
                }
                else {
                    this.line(`if (${this.emitExpr(expr.cond)}) {`);
                    this.indented(() => {
                        this.emitExprAsStatements(expr.then, isReturn);
                    });
                    if (expr.else.kind !== "unit") {
                        this.line("} else {");
                        this.indented(() => {
                            this.emitExprAsStatements(expr.else, isReturn);
                        });
                    }
                    this.line("}");
                }
                break;
            case "match":
                this.emitMatchStatements(expr.scrutinee, expr.arms, isReturn);
                break;
            case "return":
                this.line(`return ${this.emitExpr(expr.expr)};`);
                break;
            case "tail_continue": {
                // Reassign parameters and continue the loop
                const tcoParams = this.currentTcoParams;
                if (tcoParams.length === expr.args.length) {
                    // Use temp variables to avoid order-dependent assignment issues
                    if (tcoParams.length === 1) {
                        this.line(`${tcoParams[0]} = ${this.emitExpr(expr.args[0])};`);
                    }
                    else {
                        // Multi-param: use temps to handle cases like f(b, a) where params are [a, b]
                        for (let i = 0; i < tcoParams.length; i++) {
                            this.line(`const __tco_${tcoParams[i]} = ${this.emitExpr(expr.args[i])};`);
                        }
                        for (let i = 0; i < tcoParams.length; i++) {
                            this.line(`${tcoParams[i]} = __tco_${tcoParams[i]};`);
                        }
                    }
                    this.line("continue;");
                }
                break;
            }
            case "tail_loop":
                // Should only appear at function level, handled by emitFnDecl
                this.line("while (true) {");
                this.indented(() => {
                    this.emitExprAsStatements(expr.body, isReturn);
                });
                this.line("}");
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
        this.line(`const ${expr.name} = ${this.emitExpr(expr.value)};`);
        this.emitExprAsStatements(expr.body, isReturn);
    }
    emitExprAsReturn(expr) {
        return this.emitExpr(expr);
    }
    // ─── Pattern Matching ───
    emitMatchStatements(scrutinee, arms, isReturn) {
        const scrExpr = this.emitExpr(scrutinee);
        // Assign scrutinee to temp var to avoid double evaluation
        const needsTemp = scrutinee.kind !== "var";
        const scrStr = needsTemp ? `__match_${this.matchCounter++}` : scrExpr;
        if (needsTemp) {
            this.line(`const ${scrStr} = ${scrExpr};`);
        }
        // Check if all arms are constructor patterns (tag-based switch)
        const allConstructors = arms.every(a => a.pattern.kind === "pconstructor");
        if (allConstructors) {
            this.line(`switch (${scrStr}._tag) {`);
            this.indented(() => {
                for (const arm of arms) {
                    const pat = arm.pattern;
                    this.line(`case "${pat.tag}": {`);
                    this.indented(() => {
                        // Bind fields
                        for (let i = 0; i < pat.args.length; i++) {
                            const arg = pat.args[i];
                            if (arg.kind === "pvar") {
                                this.line(`const ${arg.name} = ${scrStr}._${i};`);
                            }
                        }
                        if (arm.guard) {
                            this.line(`if (${this.emitExpr(arm.guard)}) {`);
                            this.indented(() => {
                                this.emitExprAsStatements(arm.body, isReturn);
                            });
                            this.line("}");
                        }
                        else {
                            this.emitExprAsStatements(arm.body, isReturn);
                        }
                    });
                    this.line("  break;");
                    this.line("}");
                }
            });
            this.line("}");
        }
        else {
            // General if/else chain
            for (let i = 0; i < arms.length; i++) {
                const arm = arms[i];
                const cond = this.emitPatternCondition(arm.pattern, scrStr);
                const isFirst = i === 0;
                const isLast = i === arms.length - 1;
                if (cond === "true" && isLast) {
                    // Wildcard/var as last arm
                    if (!isFirst) {
                        this.line("} else {");
                    }
                    else {
                        this.line("{");
                    }
                }
                else if (isFirst) {
                    this.line(`if (${cond}) {`);
                }
                else {
                    this.line(`} else if (${cond}) {`);
                }
                this.indented(() => {
                    this.emitPatternBindings(arm.pattern, scrStr);
                    this.emitExprAsStatements(arm.body, isReturn);
                });
                if (isLast) {
                    this.line("}");
                }
            }
        }
    }
    emitPatternCondition(pat, scrStr) {
        switch (pat.kind) {
            case "pvar":
                return "true";
            case "pwildcard":
                return "true";
            case "pconstructor":
                return `${scrStr}._tag === "${pat.tag}"`;
            case "pliteral":
                return `${scrStr} === ${this.emitExpr(pat.value)}`;
            case "plist":
                if (pat.elements.length === 0 && !pat.rest) {
                    return `${scrStr}.length === 0`;
                }
                return `${scrStr}.length >= ${pat.elements.length}`;
        }
    }
    emitPatternBindings(pat, scrStr) {
        switch (pat.kind) {
            case "pvar":
                this.line(`const ${pat.name} = ${scrStr};`);
                break;
            case "pconstructor":
                for (let i = 0; i < pat.args.length; i++) {
                    const arg = pat.args[i];
                    if (arg.kind === "pvar") {
                        this.line(`const ${arg.name} = ${scrStr}._${i};`);
                    }
                }
                break;
            case "plist":
                for (let i = 0; i < pat.elements.length; i++) {
                    const el = pat.elements[i];
                    if (el.kind === "pvar") {
                        this.line(`const ${el.name} = ${scrStr}[${i}];`);
                    }
                }
                if (pat.rest) {
                    this.line(`const ${pat.rest} = ${scrStr}.slice(${pat.elements.length});`);
                }
                break;
            case "pliteral":
            case "pwildcard":
                break;
        }
    }
    // ─── Helpers ───
    isSimpleExpr(expr) {
        switch (expr.kind) {
            case "int":
            case "float":
            case "string":
            case "bool":
            case "unit":
            case "var":
                return true;
            case "binop":
            case "concat":
                return this.isSimpleExpr(expr.left) && this.isSimpleExpr(expr.right);
            case "unaryop":
                return this.isSimpleExpr(expr.operand);
            case "app":
                return this.isSimpleExpr(expr.fn) && expr.args.every(a => this.isSimpleExpr(a));
            case "field_access":
                return this.isSimpleExpr(expr.expr);
            case "construct":
                return expr.args.every(a => this.isSimpleExpr(a));
            case "list":
                return expr.elements.every(e => this.isSimpleExpr(e));
            case "record":
                return expr.fields.every(([, v]) => this.isSimpleExpr(v));
            case "record_update":
                return this.isSimpleExpr(expr.record) && expr.updates.every(([, v]) => this.isSimpleExpr(v));
            case "tail_loop":
            case "tail_continue":
                return false;
            default:
                return false;
        }
    }
    mapType(type) {
        switch (type) {
            case "Int": return "number";
            case "Float": return "number";
            case "String": return "string";
            case "Bool": return "boolean";
            default: return type;
        }
    }
    sanitizeName(name) {
        return name.replace(/[^a-zA-Z0-9_]/g, "_");
    }
    line(s) {
        this.output.push(this.indentStr() + s);
    }
    indentStr() {
        return "  ".repeat(this.indent);
    }
    indented(fn) {
        this.indent++;
        fn();
        this.indent--;
    }
}
//# sourceMappingURL=emit.js.map
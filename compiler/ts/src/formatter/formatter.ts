import * as AST from '../parser/ast.js';

export class Formatter {
  private indent: number = 0;
  private output: string[] = [];

  format(module: AST.Module): string {
    this.indent = 0;
    this.output = [];
    for (let i = 0; i < module.decls.length; i++) {
      this.formatDecl(module.decls[i]);
      if (i < module.decls.length - 1) {
        this.output.push('');
      }
    }
    return this.output.join('\n') + '\n';
  }

  // ─── Declarations ───

  private formatDecl(decl: AST.Decl): void {
    switch (decl.kind) {
      case 'fn': this.formatFn(decl); break;
      case 'type': this.formatType(decl); break;
      case 'record_type': this.formatRecordType(decl); break;
      case 'trait': this.formatTrait(decl); break;
      case 'impl': this.formatImpl(decl); break;
      case 'module': this.formatModule(decl); break;
      case 'import': this.formatImport(decl); break;
      case 'test': this.formatTest(decl); break;
      case 'supervisor': this.formatSupervisor(decl); break;
      case 'foreign': this.formatForeign(decl); break;
    }
  }

  private formatFn(decl: Extract<AST.Decl, { kind: 'fn' }>): void {
    const pub = decl.pub ? 'pub ' : '';
    const params = decl.params.map(p => this.formatParam(p)).join(', ');
    const ret = decl.returnType ? ' -> ' + this.formatTypeExpr(decl.returnType) : '';
    const effects = decl.effects ? this.formatEffects(decl.effects) : '';

    if (this.isSimpleExpr(decl.body)) {
      this.line(`${pub}fn ${decl.name}(${params})${ret}${effects} {`);
      this.indent++;
      this.formatExpr(decl.body);
      this.indent--;
      this.line('}');
    } else {
      this.line(`${pub}fn ${decl.name}(${params})${ret}${effects} {`);
      this.indent++;
      this.formatBlockContents(decl.body);
      this.indent--;
      this.line('}');
    }
  }

  private formatType(decl: Extract<AST.Decl, { kind: 'type' }>): void {
    const typeParams = decl.typeParams.length > 0
      ? '(' + decl.typeParams.join(', ') + ')'
      : '';
    this.line(`type ${decl.name}${typeParams} =`);
    this.indent++;
    for (const v of decl.variants) {
      const fields = v.fields.length > 0
        ? '(' + v.fields.map(f => this.formatTypeExpr(f)).join(', ') + ')'
        : '';
      this.line(`| ${v.name}${fields}`);
    }
    this.indent--;
  }

  private formatRecordType(decl: Extract<AST.Decl, { kind: 'record_type' }>): void {
    const typeParams = decl.typeParams.length > 0
      ? '(' + decl.typeParams.join(', ') + ')'
      : '';
    if (decl.fields.length === 0) {
      this.line(`type ${decl.name}${typeParams} = {}`);
      return;
    }
    this.line(`type ${decl.name}${typeParams} = {`);
    this.indent++;
    for (let i = 0; i < decl.fields.length; i++) {
      const f = decl.fields[i];
      const comma = i < decl.fields.length - 1 ? ',' : '';
      this.line(`${f.name}: ${this.formatTypeExpr(f.type)}${comma}`);
    }
    this.indent--;
    this.line('}');
  }

  private formatTrait(decl: Extract<AST.Decl, { kind: 'trait' }>): void {
    const supers = decl.supertraits.length > 0
      ? ': ' + decl.supertraits.join(', ')
      : '';
    this.line(`trait ${decl.name}(${decl.typeParam})${supers} {`);
    this.indent++;
    for (const m of decl.methods) {
      const params = m.params.map(p => this.formatParam(p)).join(', ');
      const ret = m.returnType ? ' -> ' + this.formatTypeExpr(m.returnType) : '';
      this.line(`fn ${m.name}(${params})${ret}`);
    }
    this.indent--;
    this.line('}');
  }

  private formatImpl(decl: Extract<AST.Decl, { kind: 'impl' }>): void {
    this.line(`impl ${decl.traitName}(${decl.typeName}) {`);
    this.indent++;
    for (let i = 0; i < decl.methods.length; i++) {
      this.formatDecl(decl.methods[i]);
      if (i < decl.methods.length - 1) {
        this.output.push('');
      }
    }
    this.indent--;
    this.line('}');
  }

  private formatModule(decl: Extract<AST.Decl, { kind: 'module' }>): void {
    this.line(`module ${decl.name} {`);
    this.indent++;
    for (let i = 0; i < decl.decls.length; i++) {
      this.formatDecl(decl.decls[i]);
      if (i < decl.decls.length - 1) {
        this.output.push('');
      }
    }
    this.indent--;
    this.line('}');
  }

  private formatImport(decl: Extract<AST.Decl, { kind: 'import' }>): void {
    const path = decl.path.join('.');
    if (decl.items.length > 0) {
      this.line(`import ${path}.{${decl.items.join(', ')}}`);
    } else {
      this.line(`import ${path}`);
    }
  }

  private formatTest(decl: Extract<AST.Decl, { kind: 'test' }>): void {
    this.line(`test "${decl.name}" {`);
    this.indent++;
    this.formatBlockContents(decl.body);
    this.indent--;
    this.line('}');
  }

  private formatSupervisor(decl: Extract<AST.Decl, { kind: 'supervisor' }>): void {
    this.line(`supervisor ${decl.name} {`);
    this.indent++;
    this.line(`strategy = ${decl.strategy}`);
    for (const child of decl.children) {
      this.formatExpr(child);
    }
    this.indent--;
    this.line('}');
  }

  private formatForeign(decl: Extract<AST.Decl, { kind: 'foreign' }>): void {
    const mod = decl.module ? ` "${decl.module}"` : '';
    const alias = decl.jsName ? ` as "${decl.jsName}"` : '';
    const params = decl.params.map(p => this.formatParam(p)).join(', ');
    const ret = ' -> ' + this.formatTypeExpr(decl.returnType);
    this.line(`foreign${mod} fn ${decl.name}${alias}(${params})${ret}`);
  }

  // ─── Expressions ───

  private formatExpr(expr: AST.Expr): void {
    this.line(this.exprToString(expr));
  }

  private formatBlockContents(expr: AST.Expr): void {
    if (expr.kind === 'block') {
      for (const e of expr.exprs) {
        this.formatBlockExprItem(e);
      }
    } else if (expr.kind === 'let') {
      this.formatLetChain(expr);
    } else {
      this.formatExpr(expr);
    }
  }

  private formatBlockExprItem(expr: AST.Expr): void {
    if (expr.kind === 'let') {
      this.formatLetChain(expr);
    } else {
      this.formatExpr(expr);
    }
  }

  private formatLetChain(expr: AST.Expr): void {
    if (expr.kind !== 'let') {
      this.formatExpr(expr);
      return;
    }
    const typeAnnotation = expr.type ? ': ' + this.formatTypeExpr(expr.type) : '';
    this.line(`let ${expr.name}${typeAnnotation} = ${this.exprToString(expr.value)}`);
    if (expr.body.kind === 'unit') {
      // No continuation
    } else {
      this.formatLetChain(expr.body);
    }
  }

  private exprToString(expr: AST.Expr): string {
    switch (expr.kind) {
      case 'int':
        return String(expr.value);
      case 'float':
        return String(expr.value);
      case 'string':
        return expr.value;
      case 'bool':
        return String(expr.value);
      case 'unit':
        return '()';
      case 'var':
        return expr.name;
      case 'constructor':
        if (expr.args.length === 0) return expr.name;
        return `${expr.name}(${expr.args.map(a => this.exprToString(a)).join(', ')})`;
      case 'app':
        return `${this.exprToString(expr.fn)}(${expr.args.map(a => this.exprToString(a)).join(', ')})`;
      case 'lambda': {
        const params = expr.params.map(p => this.formatParam(p)).join(', ');
        return `fn(${params}) { ${this.exprToString(expr.body)} }`;
      }
      case 'let': {
        // Inline let for simple contexts — but normally handled by formatLetChain
        const typeAnnotation = expr.type ? ': ' + this.formatTypeExpr(expr.type) : '';
        return `let ${expr.name}${typeAnnotation} = ${this.exprToString(expr.value)}`;
      }
      case 'match':
        return this.formatMatchInline(expr);
      case 'if':
        return this.formatIfInline(expr);
      case 'pipe':
        return `${this.exprToString(expr.left)} |> ${this.exprToString(expr.right)}`;
      case 'binop':
        return `${this.exprToString(expr.left)} ${expr.op} ${this.exprToString(expr.right)}`;
      case 'unaryop':
        return `${expr.op}${this.exprToString(expr.operand)}`;
      case 'record': {
        if (expr.fields.length === 0) return '{}';
        const fields = expr.fields.map(([k, v]) => `${k}: ${this.exprToString(v)}`).join(', ');
        return `{ ${fields} }`;
      }
      case 'field_access':
        return `${this.exprToString(expr.expr)}.${expr.field}`;
      case 'record_update': {
        const fields = expr.fields.map(([k, v]) => `${k}: ${this.exprToString(v)}`).join(', ');
        return `{ ${this.exprToString(expr.record)} | ${fields} }`;
      }
      case 'list': {
        if (expr.elements.length === 0) return '[]';
        return `[${expr.elements.map(e => this.exprToString(e)).join(', ')}]`;
      }
      case 'block': {
        // Multi-line block — handled specially in formatBlockContents
        // For inline use, join with semicolons
        return expr.exprs.map(e => this.exprToString(e)).join('; ');
      }
      case 'spawn':
        return `spawn(${this.exprToString(expr.expr)})`;
      case 'send':
        return `send(${this.exprToString(expr.target)}, ${this.exprToString(expr.message)})`;
      case 'receive':
        return this.formatReceiveInline(expr);
      case 'try':
        return `${this.exprToString(expr.expr)}?`;
      case 'return':
        return expr.expr ? `return ${this.exprToString(expr.expr)}` : 'return';
    }
  }

  private formatMatchInline(expr: Extract<AST.Expr, { kind: 'match' }>): string {
    const lines: string[] = [];
    lines.push(`match ${this.exprToString(expr.scrutinee)} {`);
    for (const arm of expr.arms) {
      const pat = this.patternToString(arm.pattern);
      const guard = arm.guard ? ` if ${this.exprToString(arm.guard)}` : '';
      lines.push(`${this.indentStr(this.indent + 1)}${pat}${guard} => ${this.exprToString(arm.body)}`);
    }
    lines.push(`${this.indentStr(this.indent)}}`);
    return lines.join('\n');
  }

  private formatIfInline(expr: Extract<AST.Expr, { kind: 'if' }>): string {
    const lines: string[] = [];
    lines.push(`if ${this.exprToString(expr.condition)} {`);
    this.indent++;
    const thenLines = this.captureLines(() => this.formatBlockContents(expr.then));
    lines.push(...thenLines);
    this.indent--;
    if (expr.else) {
      lines.push(`${this.indentStr(this.indent)}} else {`);
      this.indent++;
      const elseLines = this.captureLines(() => this.formatBlockContents(expr.else!));
      lines.push(...elseLines);
      this.indent--;
      lines.push(`${this.indentStr(this.indent)}}`);
    } else {
      lines.push(`${this.indentStr(this.indent)}}`);
    }
    return lines.join('\n');
  }

  private formatReceiveInline(expr: Extract<AST.Expr, { kind: 'receive' }>): string {
    const lines: string[] = [];
    lines.push('receive {');
    for (const arm of expr.arms) {
      const pat = this.patternToString(arm.pattern);
      const guard = arm.guard ? ` if ${this.exprToString(arm.guard)}` : '';
      lines.push(`${this.indentStr(this.indent + 1)}${pat}${guard} => ${this.exprToString(arm.body)}`);
    }
    lines.push(`${this.indentStr(this.indent)}}`);
    return lines.join('\n');
  }

  // ─── Patterns ───

  private patternToString(pat: AST.Pattern): string {
    switch (pat.kind) {
      case 'pvar':
        return pat.name;
      case 'pconstructor':
        if (pat.args.length === 0) return pat.name;
        return `${pat.name}(${pat.args.map(a => this.patternToString(a)).join(', ')})`;
      case 'pliteral':
        return this.exprToString(pat.value);
      case 'pwildcard':
        return '_';
      case 'precord': {
        const fields = pat.fields.map(([k, v]) => `${k}: ${this.patternToString(v)}`).join(', ');
        return `{ ${fields} }`;
      }
      case 'plist': {
        const elems = pat.elements.map(e => this.patternToString(e)).join(', ');
        if (pat.rest) return `[${elems}, ...${pat.rest}]`;
        return `[${elems}]`;
      }
      case 'ptuple': {
        return `(${pat.elements.map(e => this.patternToString(e)).join(', ')})`;
      }
    }
  }

  // ─── Type Expressions ───

  private formatTypeExpr(t: AST.TypeExpr): string {
    switch (t.kind) {
      case 'tnamed':
        if (t.args.length === 0) return t.name;
        return `${t.name}(${t.args.map(a => this.formatTypeExpr(a)).join(', ')})`;
      case 'tfn':
        return `(${t.params.map(p => this.formatTypeExpr(p)).join(', ')}) -> ${this.formatTypeExpr(t.ret)}`;
      case 'trecord': {
        const fields = t.fields.map(([k, v]) => `${k}: ${this.formatTypeExpr(v)}`).join(', ');
        if (t.row) return `{ ${fields} | ${t.row} }`;
        return `{ ${fields} }`;
      }
      case 'ttuple':
        return `(${t.elements.map(e => this.formatTypeExpr(e)).join(', ')})`;
      case 'tunit':
        return '()';
      case 'tvar':
        return t.name;
    }
  }

  // ─── Helpers ───

  private formatParam(p: AST.Param): string {
    if (p.type) {
      return `${p.name}: ${this.formatTypeExpr(p.type)}`;
    }
    return p.name;
  }

  private formatEffects(e: AST.EffectExpr): string {
    return ' !' + e.effects.join(', ');
  }

  private isSimpleExpr(expr: AST.Expr): boolean {
    return expr.kind !== 'block' && expr.kind !== 'let' && expr.kind !== 'match' && expr.kind !== 'if';
  }

  private line(text: string): void {
    this.output.push(this.indentStr(this.indent) + text);
  }

  private indentStr(level: number): string {
    return '  '.repeat(level);
  }

  private captureLines(fn: () => void): string[] {
    const prev = this.output;
    this.output = [];
    fn();
    const captured = this.output;
    this.output = prev;
    return captured;
  }
}

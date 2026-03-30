import { TokenKind, tokenKindName } from '../lexer/token.js';
import { Lexer } from '../lexer/lexer.js';
export class ParseError extends Error {
    message;
    span;
    constructor(message, span) {
        super(`${message} at line ${span.line}, col ${span.col}`);
        this.message = message;
        this.span = span;
    }
}
function infixBp(kind) {
    switch (kind) {
        case TokenKind.Pipe: return [2, 3]; // |> left-assoc
        case TokenKind.Compose: return [5, 4]; // >> right-assoc
        case TokenKind.Or: return [6, 7]; // || left-assoc
        case TokenKind.And: return [8, 9]; // && left-assoc
        case TokenKind.Eq:
        case TokenKind.NotEq: return [10, 11]; // == != left-assoc
        case TokenKind.Lt:
        case TokenKind.Gt:
        case TokenKind.LtEq:
        case TokenKind.GtEq: return [12, 13]; // comparisons left-assoc
        case TokenKind.Concat: return [15, 14]; // <> right-assoc
        case TokenKind.Plus:
        case TokenKind.Minus: return [16, 17]; // + - left-assoc
        case TokenKind.Star:
        case TokenKind.Slash:
        case TokenKind.Percent: return [18, 19]; // * / % left-assoc
        default: return null;
    }
}
function prefixBp(_kind) {
    switch (_kind) {
        case TokenKind.Minus:
        case TokenKind.Not: return 20; // unary
        default: return null;
    }
}
// Postfix binding power for ? (try), . (field access), ( (application)
const POSTFIX_BP = 22;
function opString(kind) {
    switch (kind) {
        case TokenKind.Plus: return "+";
        case TokenKind.Minus: return "-";
        case TokenKind.Star: return "*";
        case TokenKind.Slash: return "/";
        case TokenKind.Percent: return "%";
        case TokenKind.Eq: return "==";
        case TokenKind.NotEq: return "!=";
        case TokenKind.Lt: return "<";
        case TokenKind.Gt: return ">";
        case TokenKind.LtEq: return "<=";
        case TokenKind.GtEq: return ">=";
        case TokenKind.And: return "&&";
        case TokenKind.Or: return "||";
        case TokenKind.Concat: return "<>";
        case TokenKind.Compose: return ">>";
        case TokenKind.Not: return "!";
        default: return "?";
    }
}
export class Parser {
    tokens;
    pos;
    errors;
    constructor(tokens) {
        // Filter out comments and newlines for simpler parsing
        this.tokens = tokens.filter((t) => t.kind !== TokenKind.Comment && t.kind !== TokenKind.Newline);
        this.pos = 0;
        this.errors = [];
    }
    parse() {
        const startPos = this.pos;
        const decls = [];
        while (!this.atEnd()) {
            try {
                decls.push(this.parseDecl());
            }
            catch (e) {
                if (e instanceof ParseError) {
                    this.errors.push(e);
                    // Skip to next potential declaration start
                    this.advance();
                }
                else {
                    throw e;
                }
            }
        }
        return { decls, span: this.spanFrom(startPos) };
    }
    getErrors() {
        return this.errors;
    }
    // ─── Declarations ───
    parseDecl() {
        const tok = this.current();
        switch (tok.kind) {
            case TokenKind.Pub: {
                this.advance();
                if (this.current().kind === TokenKind.Fn || this.current().kind === TokenKind.Tool) {
                    return this.parseFnDecl(true);
                }
                throw this.error(`Expected 'fn' or 'tool' after 'pub'`);
            }
            case TokenKind.Fn: return this.parseFnDecl(false);
            case TokenKind.Tool: return this.parseFnDecl(false);
            case TokenKind.Type: return this.parseTypeDecl();
            case TokenKind.Trait: return this.parseTraitDecl();
            case TokenKind.Impl: return this.parseImplDecl();
            case TokenKind.Module: return this.parseModuleDecl();
            case TokenKind.Import: return this.parseImportDecl();
            case TokenKind.Test: return this.parseTestDecl();
            case TokenKind.Supervisor: return this.parseSupervisorDecl();
            case TokenKind.Foreign: return this.parseForeignDecl();
            default:
                throw this.error(`Expected declaration, got ${tokenKindName(tok.kind)}`);
        }
    }
    parseFnDecl(pub) {
        const startPos = this.pos;
        // Accept both 'fn' and 'tool' keywords ('tool' desugars to 'fn')
        if (this.current().kind === TokenKind.Tool) {
            this.advance();
        }
        else {
            this.expect(TokenKind.Fn);
        }
        const name = this.expect(TokenKind.Ident).value;
        this.expect(TokenKind.LParen);
        const params = this.parseParamList();
        this.expect(TokenKind.RParen);
        let returnType;
        if (this.match(TokenKind.Arrow)) {
            returnType = this.parseTypeExpr();
        }
        let effects;
        if (this.check(TokenKind.Not)) {
            effects = this.parseEffectExpr();
        }
        let body;
        if (this.match(TokenKind.LBrace)) {
            body = this.parseBlockBody();
        }
        else if (this.match(TokenKind.Assign)) {
            body = this.parseExpr();
        }
        else {
            throw this.error(`Expected '{' or '=' in function body`);
        }
        return { kind: "fn", name, params, returnType, effects, body, pub, span: this.spanFrom(startPos) };
    }
    parseTypeDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Type);
        const name = this.expect(TokenKind.UpperIdent).value;
        // Optional type params: (a) or (a, b)
        const typeParams = [];
        if (this.match(TokenKind.LParen)) {
            do {
                typeParams.push(this.expect(TokenKind.Ident).value);
            } while (this.match(TokenKind.Comma));
            this.expect(TokenKind.RParen);
        }
        this.expect(TokenKind.Assign);
        // Sum type: | Variant | Variant
        if (this.check(TokenKind.Bar)) {
            const variants = [];
            while (this.match(TokenKind.Bar)) {
                const vStart = this.pos;
                const vName = this.expect(TokenKind.UpperIdent).value;
                const fields = [];
                if (this.match(TokenKind.LParen)) {
                    if (!this.check(TokenKind.RParen)) {
                        do {
                            fields.push(this.parseTypeExpr());
                        } while (this.match(TokenKind.Comma));
                    }
                    this.expect(TokenKind.RParen);
                }
                variants.push({ name: vName, fields, span: this.spanFrom(vStart) });
            }
            return { kind: "type", name, typeParams, variants, span: this.spanFrom(startPos) };
        }
        // Record type: { field: T, ... }
        if (this.check(TokenKind.LBrace)) {
            this.advance();
            const fields = [];
            if (!this.check(TokenKind.RBrace)) {
                do {
                    const fStart = this.pos;
                    const fName = this.expect(TokenKind.Ident).value;
                    this.expect(TokenKind.Colon);
                    const fType = this.parseTypeExpr();
                    fields.push({ name: fName, type: fType, span: this.spanFrom(fStart) });
                } while (this.match(TokenKind.Comma));
            }
            this.expect(TokenKind.RBrace);
            return { kind: "record_type", name, typeParams, fields, span: this.spanFrom(startPos) };
        }
        throw this.error(`Expected '|' or '{' in type declaration`);
    }
    parseTraitDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Trait);
        const name = this.expect(TokenKind.UpperIdent).value;
        this.expect(TokenKind.LParen);
        const typeParam = this.expect(TokenKind.Ident).value;
        this.expect(TokenKind.RParen);
        const supertraits = [];
        // Optional: where constraints (simplified as supertraits)
        if (this.match(TokenKind.Colon)) {
            do {
                supertraits.push(this.expect(TokenKind.UpperIdent).value);
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.LBrace);
        const methods = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            methods.push(this.parseFnSig());
        }
        this.expect(TokenKind.RBrace);
        return { kind: "trait", name, typeParam, supertraits, methods, span: this.spanFrom(startPos) };
    }
    parseImplDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Impl);
        const traitName = this.expect(TokenKind.UpperIdent).value;
        // impl Trait for Type { ... } or impl Trait(Type) { ... }
        let typeName;
        if (this.match(TokenKind.LParen)) {
            typeName = this.expect(TokenKind.UpperIdent).value;
            this.expect(TokenKind.RParen);
        }
        else {
            // Simple: impl TraitName TypeName { ... }
            typeName = this.expect(TokenKind.UpperIdent).value;
        }
        this.expect(TokenKind.LBrace);
        const methods = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            methods.push(this.parseFnDecl(false));
        }
        this.expect(TokenKind.RBrace);
        return { kind: "impl", traitName, typeName, methods, span: this.spanFrom(startPos) };
    }
    parseModuleDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Module);
        const name = this.expect(TokenKind.UpperIdent).value;
        this.expect(TokenKind.LBrace);
        const decls = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            decls.push(this.parseDecl());
        }
        this.expect(TokenKind.RBrace);
        return { kind: "module", name, decls, span: this.spanFrom(startPos) };
    }
    parseImportDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Import);
        const path = [];
        path.push(this.expect(TokenKind.UpperIdent).value);
        while (this.match(TokenKind.Dot)) {
            if (this.check(TokenKind.LBrace)) {
                break;
            }
            if (this.check(TokenKind.UpperIdent)) {
                path.push(this.advance().value);
            }
            else if (this.check(TokenKind.Ident)) {
                // Single item import: import List.map
                path.push(this.advance().value);
            }
            else {
                throw this.error(`Expected identifier in import path`);
            }
        }
        const items = [];
        if (this.check(TokenKind.LBrace)) {
            // The dot before { was already consumed by the while loop above
            this.advance(); // consume {
            if (!this.check(TokenKind.RBrace)) {
                do {
                    if (this.check(TokenKind.Ident)) {
                        items.push(this.advance().value);
                    }
                    else if (this.check(TokenKind.UpperIdent)) {
                        items.push(this.advance().value);
                    }
                    else {
                        throw this.error(`Expected identifier in import list`);
                    }
                } while (this.match(TokenKind.Comma));
            }
            this.expect(TokenKind.RBrace);
        }
        return { kind: "import", path, items, span: this.spanFrom(startPos) };
    }
    parseTestDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Test);
        const name = this.expect(TokenKind.String).value;
        this.expect(TokenKind.LBrace);
        const body = this.parseBlockBody();
        return { kind: "test", name, body, span: this.spanFrom(startPos) };
    }
    parseSupervisorDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Supervisor);
        const name = this.expect(TokenKind.Ident).value;
        this.expect(TokenKind.LBrace);
        // strategy = one_for_one
        this.expect(TokenKind.Strategy);
        this.expect(TokenKind.Assign);
        const strategy = this.expect(TokenKind.Ident).value;
        // children are expressions
        const children = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            children.push(this.parseExpr());
            this.match(TokenKind.Comma); // optional comma
        }
        this.expect(TokenKind.RBrace);
        return { kind: "supervisor", name, strategy, children, span: this.spanFrom(startPos) };
    }
    parseForeignDecl() {
        const startPos = this.pos;
        this.expect(TokenKind.Foreign);
        // Optional module string: foreign "node:fs" fn ...
        let module;
        if (this.check(TokenKind.String)) {
            const raw = this.advance().value;
            // Strip surrounding quotes from the lexer token value
            module = raw.replace(/^"|"$/g, "");
        }
        this.expect(TokenKind.Fn);
        const name = this.expect(TokenKind.Ident).value;
        // Optional JS name alias: foreign "node:fs" fn read_file as "readFileSync"(...)
        let jsName;
        if (this.check(TokenKind.As)) {
            this.advance();
            const raw = this.expect(TokenKind.String).value;
            jsName = raw.replace(/^"|"$/g, "");
        }
        this.expect(TokenKind.LParen);
        const params = this.parseParamList();
        this.expect(TokenKind.RParen);
        let returnType;
        if (this.match(TokenKind.Arrow)) {
            returnType = this.parseTypeExpr();
        }
        else {
            // No return type → Unit
            returnType = { kind: "tnamed", name: "Unit", args: [], span: this.spanFrom(startPos) };
        }
        return { kind: "foreign", module, name, jsName, params, returnType, span: this.spanFrom(startPos) };
    }
    // ─── Helper: function signature (for traits) ───
    parseFnSig() {
        const startPos = this.pos;
        this.expect(TokenKind.Fn);
        const name = this.expect(TokenKind.Ident).value;
        this.expect(TokenKind.LParen);
        const params = this.parseParamList();
        this.expect(TokenKind.RParen);
        let returnType;
        if (this.match(TokenKind.Arrow)) {
            returnType = this.parseTypeExpr();
        }
        return { name, params, returnType, span: this.spanFrom(startPos) };
    }
    parseParamList() {
        const params = [];
        if (this.check(TokenKind.RParen))
            return params;
        do {
            const pStart = this.pos;
            const name = this.expect(TokenKind.Ident).value;
            let type;
            if (this.match(TokenKind.Colon)) {
                type = this.parseTypeExpr();
            }
            params.push({ name, type, span: this.spanFrom(pStart) });
        } while (this.match(TokenKind.Comma));
        return params;
    }
    // ─── Expressions (Pratt parser) ───
    parseExpr(minBp = 0) {
        let left = this.parsePrefixExpr();
        for (;;) {
            // Postfix: ?, .field, (args)
            if (this.check(TokenKind.Question) && POSTFIX_BP >= minBp) {
                const startPos = this.pos;
                this.advance();
                left = { kind: "try", expr: left, span: this.spanFromExpr(left, startPos) };
                continue;
            }
            if (this.check(TokenKind.Dot) && POSTFIX_BP >= minBp) {
                this.advance();
                const field = this.expect(TokenKind.Ident).value;
                left = { kind: "field_access", expr: left, field, span: { ...left.span, end: this.prevSpan().end } };
                continue;
            }
            if (this.check(TokenKind.LParen) && POSTFIX_BP >= minBp) {
                this.advance();
                const args = [];
                if (!this.check(TokenKind.RParen)) {
                    do {
                        args.push(this.parseExpr());
                    } while (this.match(TokenKind.Comma));
                }
                this.expect(TokenKind.RParen);
                left = { kind: "app", fn: left, args, span: { ...left.span, end: this.prevSpan().end } };
                continue;
            }
            // Infix
            const tok = this.current();
            const bp = infixBp(tok.kind);
            if (!bp)
                break;
            const [leftBp, rightBp] = bp;
            if (leftBp < minBp)
                break;
            this.advance();
            if (tok.kind === TokenKind.Pipe) {
                const right = this.parseExpr(rightBp);
                left = { kind: "pipe", left, right, span: { ...left.span, end: right.span.end } };
            }
            else {
                const right = this.parseExpr(rightBp);
                left = { kind: "binop", op: opString(tok.kind), left, right, span: { ...left.span, end: right.span.end } };
            }
        }
        return left;
    }
    parsePrefixExpr() {
        const tok = this.current();
        const bp = prefixBp(tok.kind);
        if (bp !== null) {
            const startPos = this.pos;
            this.advance();
            const operand = this.parseExpr(bp);
            return { kind: "unaryop", op: opString(tok.kind), operand, span: this.spanFrom(startPos) };
        }
        return this.parsePrimary();
    }
    parsePrimary() {
        const tok = this.current();
        switch (tok.kind) {
            case TokenKind.Int: {
                this.advance();
                return { kind: "int", value: Number(tok.value.replace(/_/g, '')), span: tok.span };
            }
            case TokenKind.Float: {
                this.advance();
                return { kind: "float", value: Number(tok.value.replace(/_/g, '')), span: tok.span };
            }
            case TokenKind.String: {
                this.advance();
                // Check for string interpolation: "...${expr}..."
                if (tok.value.includes('${')) {
                    return this.desugarInterpolation(tok.value, tok.span);
                }
                return { kind: "string", value: tok.value, span: tok.span };
            }
            case TokenKind.True: {
                this.advance();
                return { kind: "bool", value: true, span: tok.span };
            }
            case TokenKind.False: {
                this.advance();
                return { kind: "bool", value: false, span: tok.span };
            }
            case TokenKind.Ident: {
                this.advance();
                return { kind: "var", name: tok.value, span: tok.span };
            }
            case TokenKind.UpperIdent: {
                return this.parseConstructorExpr();
            }
            case TokenKind.LParen: {
                return this.parseParenExpr();
            }
            case TokenKind.LBrace: {
                return this.parseRecordOrBlock();
            }
            case TokenKind.LBracket: {
                return this.parseListExpr();
            }
            case TokenKind.Let: {
                return this.parseLetExpr();
            }
            case TokenKind.Match: {
                return this.parseMatchExpr();
            }
            case TokenKind.If: {
                return this.parseIfExpr();
            }
            case TokenKind.Fn: {
                return this.parseLambdaExpr();
            }
            case TokenKind.Receive: {
                return this.parseReceiveExpr();
            }
            case TokenKind.Spawn: {
                // spawn(expr) → { kind: "spawn", expr }
                this.advance(); // consume 'spawn'
                this.expect(TokenKind.LParen);
                const spawnExpr = this.parseExpr();
                this.expect(TokenKind.RParen);
                return { kind: "spawn", expr: spawnExpr, span: this.spanFrom(tok.span.start) };
            }
            case TokenKind.Send: {
                // send(target, message) → { kind: "send", target, message }
                this.advance(); // consume 'send'
                this.expect(TokenKind.LParen);
                const target = this.parseExpr();
                this.expect(TokenKind.Comma);
                const message = this.parseExpr();
                this.expect(TokenKind.RParen);
                return { kind: "send", target, message, span: this.spanFrom(tok.span.start) };
            }
            case TokenKind.Return: {
                return this.parseReturnExpr();
            }
            default:
                throw this.error(`Unexpected token ${tokenKindName(tok.kind)} '${tok.value}'`);
        }
    }
    desugarInterpolation(raw, span) {
        // raw is e.g. "hello ${name}, age ${show(version)}!"
        // (includes surrounding quotes from lexer)
        const inner = raw.slice(1, -1); // strip surrounding quotes
        const parts = [];
        let i = 0;
        let textStart = 0;
        while (i < inner.length) {
            if (inner[i] === '\\') {
                // skip escaped characters
                i += 2;
                continue;
            }
            if (inner[i] === '$' && i + 1 < inner.length && inner[i + 1] === '{') {
                // Emit text before this interpolation
                if (i > textStart) {
                    const text = inner.slice(textStart, i);
                    parts.push({ kind: "string", value: `"${text}"`, span });
                }
                // Find matching closing brace (handle nested braces)
                let depth = 1;
                let j = i + 2;
                while (j < inner.length && depth > 0) {
                    if (inner[j] === '{')
                        depth++;
                    else if (inner[j] === '}')
                        depth--;
                    if (depth > 0)
                        j++;
                }
                // inner[i+2..j] is the expression source
                const exprSource = inner.slice(i + 2, j);
                // Parse the expression using a sub-parser
                const subLexer = new Lexer(exprSource);
                const subTokens = subLexer.tokenize();
                const subParser = new Parser(subTokens);
                const exprAst = subParser.parseExpr();
                parts.push(exprAst);
                i = j + 1; // skip past closing }
                textStart = i;
            }
            else {
                i++;
            }
        }
        // Remaining text after last interpolation
        if (textStart < inner.length) {
            const text = inner.slice(textStart);
            parts.push({ kind: "string", value: `"${text}"`, span });
        }
        if (parts.length === 0) {
            return { kind: "string", value: '""', span };
        }
        if (parts.length === 1) {
            return parts[0];
        }
        // Build left-associative concat chain
        let result = parts[0];
        for (let k = 1; k < parts.length; k++) {
            result = {
                kind: "binop",
                op: "<>",
                left: result,
                right: parts[k],
                span,
            };
        }
        return result;
    }
    parseConstructorExpr() {
        const startPos = this.pos;
        const name = this.expect(TokenKind.UpperIdent).value;
        const args = [];
        if (this.match(TokenKind.LParen)) {
            if (!this.check(TokenKind.RParen)) {
                do {
                    args.push(this.parseExpr());
                } while (this.match(TokenKind.Comma));
            }
            this.expect(TokenKind.RParen);
        }
        return { kind: "constructor", name, args, span: this.spanFrom(startPos) };
    }
    parseParenExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.LParen);
        // Unit: ()
        if (this.match(TokenKind.RParen)) {
            return { kind: "unit", span: this.spanFrom(startPos) };
        }
        const expr = this.parseExpr();
        this.expect(TokenKind.RParen);
        return expr;
    }
    parseLetExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Let);
        const name = this.expect(TokenKind.Ident).value;
        let type;
        if (this.match(TokenKind.Colon)) {
            type = this.parseTypeExpr();
        }
        this.expect(TokenKind.Assign);
        const value = this.parseExpr();
        // The body is the next expression in context (let is an expression).
        // If we're inside a block and there are more expressions, the body is the next expr.
        // If there's nothing else, body is unit.
        let body;
        if (this.check(TokenKind.Semicolon)) {
            this.advance();
            body = this.parseExpr();
        }
        else if (!this.atEnd() &&
            !this.check(TokenKind.RBrace) &&
            !this.check(TokenKind.RParen) &&
            !this.check(TokenKind.Comma) &&
            !this.check(TokenKind.EOF)) {
            body = this.parseExpr();
        }
        else {
            body = { kind: "unit", span: this.spanFrom(this.pos) };
        }
        return { kind: "let", name, type, value, body, span: this.spanFrom(startPos) };
    }
    parseMatchExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Match);
        const scrutinee = this.parseExpr(0);
        this.expect(TokenKind.LBrace);
        const arms = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            arms.push(this.parseMatchArm());
            this.match(TokenKind.Comma); // optional trailing comma
        }
        this.expect(TokenKind.RBrace);
        return { kind: "match", scrutinee, arms, span: this.spanFrom(startPos) };
    }
    parseMatchArm() {
        const startPos = this.pos;
        const pattern = this.parsePattern();
        let guard;
        if (this.match(TokenKind.If)) {
            guard = this.parseExpr();
        }
        this.expect(TokenKind.FatArrow);
        const body = this.parseExpr();
        return { pattern, guard, body, span: this.spanFrom(startPos) };
    }
    parseIfExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.If);
        const condition = this.parseExpr();
        this.expect(TokenKind.LBrace);
        const thenBranch = this.parseBlockBody();
        let elseBranch;
        if (this.match(TokenKind.Else)) {
            if (this.check(TokenKind.If)) {
                elseBranch = this.parseIfExpr();
            }
            else {
                this.expect(TokenKind.LBrace);
                elseBranch = this.parseBlockBody();
            }
        }
        return { kind: "if", condition, then: thenBranch, else: elseBranch, span: this.spanFrom(startPos) };
    }
    parseLambdaExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Fn);
        this.expect(TokenKind.LParen);
        const params = this.parseParamList();
        this.expect(TokenKind.RParen);
        // Optional return type annotation: -> Type
        if (this.match(TokenKind.Arrow)) {
            this.parseTypeExpr(); // consume and discard for now (codegen doesn't use it)
        }
        this.expect(TokenKind.LBrace);
        const body = this.parseBlockBody();
        return { kind: "lambda", params, body, span: this.spanFrom(startPos) };
    }
    parseBlockBody() {
        const startPos = this.pos;
        const exprs = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            exprs.push(this.parseExpr());
            this.match(TokenKind.Semicolon); // optional semicolons
        }
        this.expect(TokenKind.RBrace);
        if (exprs.length === 0) {
            return { kind: "unit", span: this.spanFrom(startPos) };
        }
        if (exprs.length === 1) {
            return exprs[0];
        }
        return { kind: "block", exprs, span: this.spanFrom(startPos) };
    }
    parseReceiveExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Receive);
        this.expect(TokenKind.LBrace);
        const arms = [];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            arms.push(this.parseMatchArm());
            this.match(TokenKind.Comma);
        }
        this.expect(TokenKind.RBrace);
        return { kind: "receive", arms, span: this.spanFrom(startPos) };
    }
    parseReturnExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Return);
        let expr;
        if (!this.atEnd() &&
            !this.check(TokenKind.RBrace) &&
            !this.check(TokenKind.Semicolon) &&
            !this.check(TokenKind.EOF)) {
            expr = this.parseExpr();
        }
        return { kind: "return", expr, span: this.spanFrom(startPos) };
    }
    parseRecordOrBlock() {
        const startPos = this.pos;
        this.advance(); // consume {
        // Empty block: {}
        if (this.check(TokenKind.RBrace)) {
            this.advance();
            return { kind: "unit", span: this.spanFrom(startPos) };
        }
        // Look ahead to distinguish record from block:
        // record: { ident : expr, ... }
        // record update: { expr | field: expr, ... }
        // block: { expr; expr }
        // Check for record literal: identifier followed by colon
        if (this.check(TokenKind.Ident) && this.peekAt(1)?.kind === TokenKind.Colon) {
            return this.parseRecordLiteral(startPos);
        }
        // Could be record update: { expr | field: val }
        // Or a block
        const firstExpr = this.parseExpr();
        if (this.match(TokenKind.Bar)) {
            // Record update: { record | field: val, ... }
            const fields = [];
            do {
                const fName = this.expect(TokenKind.Ident).value;
                this.expect(TokenKind.Colon);
                const fVal = this.parseExpr();
                fields.push([fName, fVal]);
            } while (this.match(TokenKind.Comma));
            this.expect(TokenKind.RBrace);
            return { kind: "record_update", record: firstExpr, fields, span: this.spanFrom(startPos) };
        }
        // Block: collect remaining expressions
        const exprs = [firstExpr];
        while (!this.check(TokenKind.RBrace) && !this.atEnd()) {
            this.match(TokenKind.Semicolon);
            if (this.check(TokenKind.RBrace))
                break;
            exprs.push(this.parseExpr());
        }
        this.expect(TokenKind.RBrace);
        if (exprs.length === 1)
            return exprs[0];
        return { kind: "block", exprs, span: this.spanFrom(startPos) };
    }
    parseRecordLiteral(startPos) {
        const fields = [];
        do {
            const fName = this.expect(TokenKind.Ident).value;
            this.expect(TokenKind.Colon);
            const fVal = this.parseExpr();
            fields.push([fName, fVal]);
        } while (this.match(TokenKind.Comma));
        this.expect(TokenKind.RBrace);
        return { kind: "record", fields, span: this.spanFrom(startPos) };
    }
    parseListExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.LBracket);
        const elements = [];
        if (!this.check(TokenKind.RBracket)) {
            do {
                elements.push(this.parseExpr());
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.RBracket);
        return { kind: "list", elements, span: this.spanFrom(startPos) };
    }
    // ─── Patterns ───
    parsePattern() {
        const tok = this.current();
        switch (tok.kind) {
            case TokenKind.UpperIdent: {
                const startPos = this.pos;
                const name = this.advance().value;
                const args = [];
                if (this.match(TokenKind.LParen)) {
                    if (!this.check(TokenKind.RParen)) {
                        do {
                            args.push(this.parsePattern());
                        } while (this.match(TokenKind.Comma));
                    }
                    this.expect(TokenKind.RParen);
                }
                if (args.length === 0 && !this.check(TokenKind.LParen)) {
                    // Could be a nullary constructor
                    return { kind: "pconstructor", name, args: [], span: this.spanFrom(startPos) };
                }
                return { kind: "pconstructor", name, args, span: this.spanFrom(startPos) };
            }
            case TokenKind.Ident: {
                if (tok.value === "_") {
                    this.advance();
                    return { kind: "pwildcard", span: tok.span };
                }
                this.advance();
                return { kind: "pvar", name: tok.value, span: tok.span };
            }
            case TokenKind.Int: {
                this.advance();
                return { kind: "pliteral", value: { kind: "int", value: Number(tok.value.replace(/_/g, '')), span: tok.span }, span: tok.span };
            }
            case TokenKind.Float: {
                this.advance();
                return { kind: "pliteral", value: { kind: "float", value: Number(tok.value.replace(/_/g, '')), span: tok.span }, span: tok.span };
            }
            case TokenKind.String: {
                this.advance();
                return { kind: "pliteral", value: { kind: "string", value: tok.value, span: tok.span }, span: tok.span };
            }
            case TokenKind.True: {
                this.advance();
                return { kind: "pliteral", value: { kind: "bool", value: true, span: tok.span }, span: tok.span };
            }
            case TokenKind.False: {
                this.advance();
                return { kind: "pliteral", value: { kind: "bool", value: false, span: tok.span }, span: tok.span };
            }
            case TokenKind.LBrace: {
                return this.parseRecordPattern();
            }
            case TokenKind.LBracket: {
                return this.parseListPattern();
            }
            case TokenKind.LParen: {
                return this.parseTuplePattern();
            }
            default:
                throw this.error(`Unexpected token in pattern: ${tokenKindName(tok.kind)}`);
        }
    }
    parseRecordPattern() {
        const startPos = this.pos;
        this.expect(TokenKind.LBrace);
        const fields = [];
        if (!this.check(TokenKind.RBrace)) {
            do {
                const fName = this.expect(TokenKind.Ident).value;
                let pat;
                if (this.match(TokenKind.Colon)) {
                    pat = this.parsePattern();
                }
                else {
                    // Shorthand: { name } is equivalent to { name: name }
                    pat = { kind: "pvar", name: fName, span: this.prevSpan() };
                }
                fields.push([fName, pat]);
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.RBrace);
        return { kind: "precord", fields, span: this.spanFrom(startPos) };
    }
    parseListPattern() {
        const startPos = this.pos;
        this.expect(TokenKind.LBracket);
        const elements = [];
        let rest;
        if (!this.check(TokenKind.RBracket)) {
            do {
                if (this.match(TokenKind.DotDot)) {
                    rest = this.expect(TokenKind.Ident).value;
                    break;
                }
                elements.push(this.parsePattern());
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.RBracket);
        return { kind: "plist", elements, rest, span: this.spanFrom(startPos) };
    }
    parseTuplePattern() {
        const startPos = this.pos;
        this.expect(TokenKind.LParen);
        // Could be () for unit
        if (this.match(TokenKind.RParen)) {
            return { kind: "pliteral", value: { kind: "unit", span: this.spanFrom(startPos) }, span: this.spanFrom(startPos) };
        }
        const elements = [];
        do {
            elements.push(this.parsePattern());
        } while (this.match(TokenKind.Comma));
        this.expect(TokenKind.RParen);
        if (elements.length === 1) {
            return elements[0]; // Not a tuple, just parenthesized
        }
        return { kind: "ptuple", elements, span: this.spanFrom(startPos) };
    }
    // ─── Type Expressions ───
    parseTypeExpr() {
        const tok = this.current();
        // Function type: fn(T, T) -> T
        if (tok.kind === TokenKind.Fn) {
            return this.parseFnTypeExpr();
        }
        // Named type: SomeType or SomeType(arg, arg)
        if (tok.kind === TokenKind.UpperIdent) {
            const startPos = this.pos;
            const name = this.advance().value;
            const args = [];
            if (this.match(TokenKind.LParen)) {
                if (!this.check(TokenKind.RParen)) {
                    do {
                        args.push(this.parseTypeExpr());
                    } while (this.match(TokenKind.Comma));
                }
                this.expect(TokenKind.RParen);
            }
            return { kind: "tnamed", name, args, span: this.spanFrom(startPos) };
        }
        // Type variable: lowercase
        if (tok.kind === TokenKind.Ident) {
            this.advance();
            return { kind: "tvar", name: tok.value, span: tok.span };
        }
        // Record type: { field: T, ... }
        if (tok.kind === TokenKind.LBrace) {
            return this.parseRecordTypeExpr();
        }
        // Parenthesized / tuple / unit
        if (tok.kind === TokenKind.LParen) {
            const startPos = this.pos;
            this.advance();
            if (this.match(TokenKind.RParen)) {
                return { kind: "tunit", span: this.spanFrom(startPos) };
            }
            const first = this.parseTypeExpr();
            if (this.match(TokenKind.Comma)) {
                // Tuple type
                const elements = [first];
                do {
                    elements.push(this.parseTypeExpr());
                } while (this.match(TokenKind.Comma));
                this.expect(TokenKind.RParen);
                return { kind: "ttuple", elements, span: this.spanFrom(startPos) };
            }
            this.expect(TokenKind.RParen);
            return first;
        }
        throw this.error(`Expected type expression, got ${tokenKindName(tok.kind)}`);
    }
    parseFnTypeExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Fn);
        this.expect(TokenKind.LParen);
        const params = [];
        if (!this.check(TokenKind.RParen)) {
            do {
                params.push(this.parseTypeExpr());
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.RParen);
        this.expect(TokenKind.Arrow);
        const ret = this.parseTypeExpr();
        return { kind: "tfn", params, ret, span: this.spanFrom(startPos) };
    }
    parseRecordTypeExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.LBrace);
        const fields = [];
        let row;
        if (!this.check(TokenKind.RBrace)) {
            do {
                const fName = this.expect(TokenKind.Ident).value;
                this.expect(TokenKind.Colon);
                const fType = this.parseTypeExpr();
                fields.push([fName, fType]);
            } while (this.match(TokenKind.Comma));
            if (this.match(TokenKind.Bar)) {
                row = this.expect(TokenKind.Ident).value;
            }
        }
        this.expect(TokenKind.RBrace);
        return { kind: "trecord", fields, row, span: this.spanFrom(startPos) };
    }
    parseEffectExpr() {
        const startPos = this.pos;
        this.expect(TokenKind.Not);
        this.expect(TokenKind.LBracket);
        const effects = [];
        if (!this.check(TokenKind.RBracket)) {
            do {
                effects.push(this.expect(TokenKind.UpperIdent).value);
            } while (this.match(TokenKind.Comma));
        }
        this.expect(TokenKind.RBracket);
        return { effects, span: this.spanFrom(startPos) };
    }
    // ─── Token Helpers ───
    current() {
        if (this.pos >= this.tokens.length) {
            // Return EOF token
            const lastSpan = this.tokens.length > 0
                ? this.tokens[this.tokens.length - 1].span
                : { start: 0, end: 0, line: 1, col: 1 };
            return { kind: TokenKind.EOF, value: "", span: lastSpan };
        }
        return this.tokens[this.pos];
    }
    advance() {
        const tok = this.current();
        if (this.pos < this.tokens.length)
            this.pos++;
        return tok;
    }
    expect(kind) {
        const tok = this.current();
        if (tok.kind !== kind) {
            throw this.error(`Expected ${tokenKindName(kind)}, got ${tokenKindName(tok.kind)} '${tok.value}'`);
        }
        return this.advance();
    }
    match(kind) {
        if (this.check(kind)) {
            this.advance();
            return true;
        }
        return false;
    }
    check(kind) {
        return this.current().kind === kind;
    }
    peek() {
        return this.peekAt(1) ?? this.current();
    }
    peekAt(offset) {
        const idx = this.pos + offset;
        if (idx >= this.tokens.length)
            return undefined;
        // Need to account for filtered tokens - we filter in constructor so indices are direct
        return this.tokens[idx];
    }
    atEnd() {
        return this.pos >= this.tokens.length || this.current().kind === TokenKind.EOF;
    }
    prevSpan() {
        if (this.pos > 0 && this.pos - 1 < this.tokens.length) {
            return this.tokens[this.pos - 1].span;
        }
        return this.current().span;
    }
    spanFrom(startPos) {
        const start = startPos < this.tokens.length
            ? this.tokens[startPos].span
            : this.current().span;
        const end = this.prevSpan();
        return { start: start.start, end: end.end, line: start.line, col: start.col };
    }
    spanFromExpr(expr, endPos) {
        const end = endPos < this.tokens.length
            ? this.tokens[endPos].span
            : this.prevSpan();
        return { start: expr.span.start, end: end.end, line: expr.span.line, col: expr.span.col };
    }
    error(message) {
        return new ParseError(message, this.current().span);
    }
}
//# sourceMappingURL=parser.js.map
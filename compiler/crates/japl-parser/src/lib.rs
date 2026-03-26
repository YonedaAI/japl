//! japl-parser: Pratt + recursive descent parser for the JAPL programming language.
//!
//! Produces an untyped AST from a token stream.

#![allow(dead_code)]

use japl_ast::*;
use japl_common::{Diagnostic, DiagnosticSink, FileId, Span};
use japl_lexer::{Lexer, SpannedToken, Token};
use smol_str::SmolStr;

/// The JAPL parser.
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    file_id: FileId,
    diagnostics: DiagnosticSink,
    node_id_counter: u32,
}

impl Parser {
    pub fn new(source: &str, file_id: FileId) -> Self {
        let lexer = Lexer::new(source, file_id);
        let tokens: Vec<SpannedToken> = lexer.collect();

        Parser {
            tokens,
            pos: 0,
            file_id,
            diagnostics: DiagnosticSink::new(),
            node_id_counter: 0,
        }
    }

    fn next_node_id(&mut self) -> NodeId {
        let id = self.node_id_counter;
        self.node_id_counter += 1;
        NodeId(id)
    }

    // ── Token access ────────────────────────────────────────

    fn current(&self) -> &SpannedToken {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream should have at least EOF")
        })
    }

    fn peek(&self) -> &Token {
        &self.current().token
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn advance(&mut self) -> SpannedToken {
        let tok = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: Token) -> Result<SpannedToken, ()> {
        if *self.peek() == expected {
            Ok(self.advance())
        } else {
            let span = self.current_span();
            self.diagnostics.emit(
                Diagnostic::error(format!(
                    "expected '{}', found '{}'",
                    expected,
                    self.peek()
                ))
                .with_label(span, "here"),
            );
            Err(())
        }
    }

    fn eat(&mut self, tok: Token) -> bool {
        if *self.peek() == tok {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while *self.peek() == Token::Newline {
            self.advance();
        }
    }

    fn at_end(&self) -> bool {
        *self.peek() == Token::Eof
    }

    // ── Error recovery ──────────────────────────────────────

    fn synchronize(&mut self) {
        loop {
            match self.peek() {
                Token::Fn
                | Token::Type
                | Token::Let
                | Token::Module
                | Token::Import
                | Token::Test
                | Token::Trait
                | Token::Impl
                | Token::Supervisor
                | Token::Foreign
                | Token::Eof => break,
                Token::Newline => {
                    self.advance();
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ── Top-level parsing ───────────────────────────────────

    pub fn parse_file(mut self) -> (SourceFile, Vec<Diagnostic>) {
        self.skip_newlines();

        let start_span = self.current_span();

        // Optional module declaration
        let module_decl = if *self.peek() == Token::Module {
            match self.parse_module_decl() {
                Ok(m) => Some(m),
                Err(()) => None,
            }
        } else {
            None
        };

        self.skip_newlines();

        // Import declarations
        let mut imports = Vec::new();
        while *self.peek() == Token::Import {
            match self.parse_import_decl() {
                Ok(i) => imports.push(i),
                Err(()) => self.synchronize(),
            }
            self.skip_newlines();
        }

        // Top-level items
        let mut items = Vec::new();
        while !self.at_end() {
            self.skip_newlines();
            if self.at_end() {
                break;
            }
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(()) => self.synchronize(),
            }
            self.skip_newlines();
        }

        let end_span = self.current_span();
        let span = start_span.merge(end_span);

        let file = SourceFile {
            module_decl,
            imports,
            items,
            span,
        };

        (file, self.diagnostics.into_diagnostics())
    }

    fn parse_module_decl(&mut self) -> Result<ModuleDecl, ()> {
        let start = self.current_span();
        self.expect(Token::Module)?;
        let name = self.parse_qualified_name()?;
        let span = start.merge(name.span);
        self.skip_newlines();
        Ok(ModuleDecl { name, span })
    }

    fn parse_import_decl(&mut self) -> Result<ImportDecl, ()> {
        let start = self.current_span();
        self.expect(Token::Import)?;
        let path = self.parse_qualified_name()?;

        let items = if self.eat(Token::Dot) {
            self.expect(Token::LBrace)?;
            let mut import_items = Vec::new();
            loop {
                if *self.peek() == Token::RBrace {
                    break;
                }
                let item = match self.peek() {
                    Token::UpperIdent => {
                        let tok = self.advance();
                        ImportItem::Type(tok.text.clone())
                    }
                    Token::LowerIdent => {
                        let tok = self.advance();
                        ImportItem::Name(tok.text.clone())
                    }
                    _ => {
                        let span = self.current_span();
                        self.diagnostics.emit(
                            Diagnostic::error("expected identifier in import list")
                                .with_label(span, "here"),
                        );
                        return Err(());
                    }
                };
                import_items.push(item);
                if !self.eat(Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RBrace)?;
            Some(import_items)
        } else {
            None
        };

        let end = self.current_span();
        self.skip_newlines();

        Ok(ImportDecl {
            path,
            items,
            span: start.merge(end),
        })
    }

    fn parse_qualified_name(&mut self) -> Result<QualifiedName, ()> {
        let start = self.current_span();
        let mut segments = Vec::new();

        match self.peek() {
            Token::UpperIdent | Token::LowerIdent => {
                let tok = self.advance();
                segments.push(tok.text.clone());
            }
            _ => {
                self.diagnostics.emit(
                    Diagnostic::error("expected identifier").with_label(start, "here"),
                );
                return Err(());
            }
        }

        while self.eat(Token::Dot) {
            match self.peek() {
                Token::UpperIdent | Token::LowerIdent => {
                    let tok = self.advance();
                    segments.push(tok.text.clone());
                }
                _ => break,
            }
        }

        let end = self.current_span();
        Ok(QualifiedName {
            segments,
            span: start.merge(end),
        })
    }

    // ── Item parsing ────────────────────────────────────────

    fn parse_item(&mut self) -> Result<Item, ()> {
        match self.peek() {
            Token::Fn => self.parse_fn_def().map(Item::FnDef),
            Token::Type => self.parse_type_decl(),
            Token::Opaque => self.parse_type_decl(),
            Token::Trait => self.parse_trait_def().map(Item::TraitDef),
            Token::Impl => self.parse_impl_block().map(Item::ImplBlock),
            Token::Test => self.parse_test_def().map(Item::TestDef),
            Token::Bench => self.parse_bench_def().map(Item::BenchDef),
            Token::Supervisor => self.parse_supervisor_def().map(Item::SupervisorDef),
            Token::Foreign => self.parse_foreign_block().map(Item::ForeignBlock),
            _ => {
                let span = self.current_span();
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "unexpected token '{}' at top level",
                        self.peek()
                    ))
                    .with_label(span, "here"),
                );
                Err(())
            }
        }
    }

    // ── Function definition ─────────────────────────────────

    fn parse_fn_def(&mut self) -> Result<FnDef, ()> {
        let start = self.current_span();
        self.expect(Token::Fn)?;

        let name_tok = self.expect(Token::LowerIdent)?;
        let name = name_tok.text.clone();

        // Optional type parameters
        let type_params = if *self.peek() == Token::LBracket {
            self.parse_type_params()?
        } else {
            vec![]
        };

        // Parameters
        self.expect(Token::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(Token::RParen)?;

        // Return type
        let return_type = if self.eat(Token::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // Effects
        let effects = if *self.peek() == Token::With {
            self.advance();
            self.parse_effect_list()?
        } else {
            vec![]
        };

        // Where clause
        let where_clause = if *self.peek() == Token::Where {
            self.advance();
            self.parse_where_clause()?
        } else {
            vec![]
        };

        // Body
        self.expect(Token::Eq)?;
        self.skip_newlines();

        let body = if self.eat(Token::Indent) {
            self.parse_block()?
        } else {
            self.parse_expr()?
        };

        let end_span = self.current_span();

        Ok(FnDef {
            id: self.next_node_id(),
            name,
            type_params,
            params,
            return_type,
            effects,
            where_clause,
            body,
            span: start.merge(end_span),
        })
    }

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, ()> {
        self.expect(Token::LBracket)?;
        let mut params = Vec::new();
        loop {
            if *self.peek() == Token::RBracket {
                break;
            }
            let start = self.current_span();
            let name_tok = self.expect(Token::LowerIdent)?;
            let end = self.current_span();
            params.push(TypeParam {
                name: name_tok.text.clone(),
                bounds: vec![],
                span: start.merge(end),
            });
            if !self.eat(Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBracket)?;
        Ok(params)
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ()> {
        let mut params = Vec::new();
        if *self.peek() == Token::RParen {
            return Ok(params);
        }
        loop {
            let param = self.parse_param()?;
            params.push(param);
            if !self.eat(Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ()> {
        let start = self.current_span();
        let id = self.next_node_id();

        let ownership = match self.peek() {
            Token::Own => {
                self.advance();
                Ownership::Own
            }
            Token::Ref => {
                self.advance();
                Ownership::Ref
            }
            _ => Ownership::Value,
        };

        let pattern = self.parse_pattern()?;

        let ty = if self.eat(Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let end = self.current_span();

        Ok(Param {
            id,
            pattern,
            ty,
            ownership,
            span: start.merge(end),
        })
    }

    fn parse_effect_list(&mut self) -> Result<Vec<EffectExpr>, ()> {
        let mut effects = Vec::new();
        loop {
            let effect = self.parse_effect_expr()?;
            effects.push(effect);
            if !self.eat(Token::Comma) {
                break;
            }
        }
        Ok(effects)
    }

    fn parse_effect_expr(&mut self) -> Result<EffectExpr, ()> {
        let start = self.current_span();
        match self.peek() {
            Token::UpperIdent => {
                let name = self.parse_qualified_name()?;
                let args = if *self.peek() == Token::LBracket {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == Token::RBracket {
                            break;
                        }
                        args.push(self.parse_type_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    self.expect(Token::RBracket)?;
                    args
                } else {
                    vec![]
                };
                let end = self.current_span();
                Ok(EffectExpr::Named {
                    name,
                    args,
                    span: start.merge(end),
                })
            }
            Token::LowerIdent => {
                let tok = self.advance();
                Ok(EffectExpr::Var {
                    name: tok.text.clone(),
                    span: tok.span,
                })
            }
            _ => {
                self.diagnostics
                    .emit(Diagnostic::error("expected effect").with_label(start, "here"));
                Err(())
            }
        }
    }

    fn parse_where_clause(&mut self) -> Result<Vec<Constraint>, ()> {
        let mut constraints = Vec::new();
        loop {
            let start = self.current_span();
            let trait_name = self.parse_qualified_name()?;
            self.expect(Token::LBracket)?;
            let mut type_args = Vec::new();
            loop {
                if *self.peek() == Token::RBracket {
                    break;
                }
                type_args.push(self.parse_type_expr()?);
                if !self.eat(Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RBracket)?;
            let end = self.current_span();
            constraints.push(Constraint {
                trait_name,
                type_args,
                span: start.merge(end),
            });
            if !self.eat(Token::Comma) {
                break;
            }
        }
        Ok(constraints)
    }

    // ── Type declarations ───────────────────────────────────

    fn parse_type_decl(&mut self) -> Result<Item, ()> {
        let start = self.current_span();

        // opaque type
        if self.eat(Token::Opaque) {
            self.expect(Token::Type)?;
            let name_tok = self.expect(Token::UpperIdent)?;
            let type_params = if *self.peek() == Token::LBracket {
                self.parse_type_params()?
            } else {
                vec![]
            };
            let end = self.current_span();
            return Ok(Item::TypeDef(TypeDef {
                id: self.next_node_id(),
                name: name_tok.text.clone(),
                type_params,
                deriving: vec![],
                is_packed: false,
                body: TypeBody::Record(vec![]),
                span: start.merge(end),
            }));
        }

        self.expect(Token::Type)?;

        // type alias
        if *self.peek() == Token::Alias {
            self.advance();
            let name_tok = self.expect(Token::UpperIdent)?;
            let type_params = if *self.peek() == Token::LBracket {
                self.parse_type_params()?
            } else {
                vec![]
            };
            self.expect(Token::Eq)?;
            self.skip_newlines();
            let target = self.parse_type_expr()?;
            let end = self.current_span();
            return Ok(Item::TypeAlias(TypeAlias {
                id: self.next_node_id(),
                name: name_tok.text.clone(),
                type_params,
                target,
                span: start.merge(end),
            }));
        }

        let name_tok = self.expect(Token::UpperIdent)?;
        let name = name_tok.text.clone();

        let type_params = if *self.peek() == Token::LBracket {
            self.parse_type_params()?
        } else {
            vec![]
        };

        let deriving = if *self.peek() == Token::Deriving {
            self.advance();
            self.expect(Token::LParen)?;
            let mut names = Vec::new();
            loop {
                if *self.peek() == Token::RParen {
                    break;
                }
                let tok = self.expect(Token::UpperIdent)?;
                names.push(tok.text.clone());
                if !self.eat(Token::Comma) {
                    break;
                }
            }
            self.expect(Token::RParen)?;
            names
        } else {
            vec![]
        };

        self.expect(Token::Eq)?;
        self.skip_newlines();

        // Handle indented type body
        let _had_indent = self.eat(Token::Indent);
        self.skip_newlines();

        let body = if *self.peek() == Token::Pipe {
            // Sum type
            let mut variants = Vec::new();
            while self.eat(Token::Pipe) {
                self.skip_newlines();
                let vstart = self.current_span();
                let vname_tok = self.expect(Token::UpperIdent)?;
                let fields = if self.eat(Token::LParen) {
                    let mut fs = Vec::new();
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        fs.push(self.parse_type_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    self.expect(Token::RParen)?;
                    fs
                } else {
                    vec![]
                };
                let vend = self.current_span();
                variants.push(Variant {
                    name: vname_tok.text.clone(),
                    fields,
                    span: vstart.merge(vend),
                });
                self.skip_newlines();
            }
            TypeBody::Sum(variants)
        } else if *self.peek() == Token::LBrace {
            // Record type
            self.expect(Token::LBrace)?;
            let mut fields = Vec::new();
            loop {
                self.skip_newlines();
                if *self.peek() == Token::RBrace {
                    break;
                }
                let fstart = self.current_span();
                let fname_tok = self.expect(Token::LowerIdent)?;
                self.expect(Token::Colon)?;
                let fty = self.parse_type_expr()?;
                let fend = self.current_span();
                fields.push(FieldDef {
                    name: fname_tok.text.clone(),
                    ty: fty,
                    span: fstart.merge(fend),
                });
                self.eat(Token::Comma);
                self.skip_newlines();
            }
            self.expect(Token::RBrace)?;
            TypeBody::Record(fields)
        } else if *self.peek() == Token::UpperIdent {
            // Inline sum type
            let mut variants = Vec::new();
            loop {
                let vstart = self.current_span();
                let vname_tok = self.expect(Token::UpperIdent)?;
                let fields = if self.eat(Token::LParen) {
                    let mut fs = Vec::new();
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        fs.push(self.parse_type_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    self.expect(Token::RParen)?;
                    fs
                } else {
                    vec![]
                };
                let vend = self.current_span();
                variants.push(Variant {
                    name: vname_tok.text.clone(),
                    fields,
                    span: vstart.merge(vend),
                });
                self.skip_newlines();
                if !self.eat(Token::Pipe) {
                    break;
                }
                self.skip_newlines();
            }
            TypeBody::Sum(variants)
        } else {
            self.diagnostics.emit(
                Diagnostic::error("expected type body").with_label(self.current_span(), "here"),
            );
            return Err(());
        };

        // Consume a trailing Dedent if present
        self.eat(Token::Dedent);

        let end = self.current_span();
        Ok(Item::TypeDef(TypeDef {
            id: self.next_node_id(),
            name,
            type_params,
            deriving,
            is_packed: false,
            body,
            span: start.merge(end),
        }))
    }

    // ── Trait definition ────────────────────────────────────

    fn parse_trait_def(&mut self) -> Result<TraitDef, ()> {
        let start = self.current_span();
        self.expect(Token::Trait)?;
        let name_tok = self.expect(Token::UpperIdent)?;

        let type_params = if *self.peek() == Token::LBracket {
            self.parse_type_params()?
        } else {
            vec![]
        };

        let supertraits = if *self.peek() == Token::Where {
            self.advance();
            self.parse_where_clause()?
        } else {
            vec![]
        };

        self.expect(Token::Eq)?;
        self.skip_newlines();

        let mut methods = Vec::new();
        if self.eat(Token::Indent) {
            loop {
                self.skip_newlines();
                if *self.peek() == Token::Dedent || self.at_end() {
                    break;
                }
                if *self.peek() == Token::Fn {
                    let method = self.parse_fn_def()?;
                    methods.push(method);
                } else {
                    self.synchronize();
                }
                self.skip_newlines();
            }
            self.eat(Token::Dedent);
        }

        let end = self.current_span();
        Ok(TraitDef {
            id: self.next_node_id(),
            name: name_tok.text.clone(),
            type_params,
            supertraits,
            methods,
            span: start.merge(end),
        })
    }

    // ── Impl block ──────────────────────────────────────────

    fn parse_impl_block(&mut self) -> Result<ImplBlock, ()> {
        let start = self.current_span();
        self.expect(Token::Impl)?;
        let trait_name = self.parse_qualified_name()?;

        self.expect(Token::LBracket)?;
        let mut type_args = Vec::new();
        loop {
            if *self.peek() == Token::RBracket {
                break;
            }
            type_args.push(self.parse_type_expr()?);
            if !self.eat(Token::Comma) {
                break;
            }
        }
        self.expect(Token::RBracket)?;

        self.expect(Token::Eq)?;
        self.skip_newlines();

        let mut methods = Vec::new();
        if self.eat(Token::Indent) {
            loop {
                self.skip_newlines();
                if *self.peek() == Token::Dedent || self.at_end() {
                    break;
                }
                let method = self.parse_fn_def()?;
                methods.push(method);
                self.skip_newlines();
            }
            self.eat(Token::Dedent);
        }

        let end = self.current_span();
        Ok(ImplBlock {
            id: self.next_node_id(),
            trait_name,
            type_args,
            methods,
            span: start.merge(end),
        })
    }

    // ── Test / Bench / Supervisor ───────────────────────────

    fn parse_test_def(&mut self) -> Result<TestDef, ()> {
        let start = self.current_span();
        self.expect(Token::Test)?;
        let name_tok = self.expect(Token::StringLiteral)?;
        let name = strip_quotes(&name_tok.text);
        self.expect(Token::Eq)?;
        self.skip_newlines();
        let body = if self.eat(Token::Indent) {
            self.parse_block()?
        } else {
            self.parse_expr()?
        };
        let end = self.current_span();
        Ok(TestDef {
            name,
            body,
            span: start.merge(end),
        })
    }

    fn parse_bench_def(&mut self) -> Result<BenchDef, ()> {
        let start = self.current_span();
        self.expect(Token::Bench)?;
        let name_tok = self.expect(Token::StringLiteral)?;
        let name = strip_quotes(&name_tok.text);
        self.expect(Token::Eq)?;
        self.skip_newlines();
        let body = if self.eat(Token::Indent) {
            self.parse_block()?
        } else {
            self.parse_expr()?
        };
        let end = self.current_span();
        Ok(BenchDef {
            name,
            body,
            span: start.merge(end),
        })
    }

    fn parse_supervisor_def(&mut self) -> Result<SupervisorDef, ()> {
        let start = self.current_span();
        self.expect(Token::Supervisor)?;
        let name_tok = self.expect(Token::LowerIdent)?;
        self.expect(Token::Eq)?;
        self.skip_newlines();

        // Parse supervisor body in simplified form
        let strategy = Expr::Var {
            name: SmolStr::new("OneForOne"),
            id: self.next_node_id(),
            span: self.current_span(),
        };
        let children = Vec::new();

        // Skip body
        if self.eat(Token::Indent) {
            let mut depth = 1;
            while depth > 0 && !self.at_end() {
                match self.peek() {
                    Token::Indent => {
                        depth += 1;
                        self.advance();
                    }
                    Token::Dedent => {
                        depth -= 1;
                        self.advance();
                    }
                    _ => {
                        self.advance();
                    }
                }
            }
        }

        let end = self.current_span();
        Ok(SupervisorDef {
            id: self.next_node_id(),
            name: name_tok.text.clone(),
            strategy,
            children,
            span: start.merge(end),
        })
    }

    fn parse_foreign_block(&mut self) -> Result<ForeignBlock, ()> {
        let start = self.current_span();
        self.expect(Token::Foreign)?;
        let abi_tok = self.expect(Token::StringLiteral)?;
        let abi = strip_quotes(&abi_tok.text);

        let mut items = Vec::new();

        if *self.peek() == Token::Fn {
            let sig = self.parse_fn_signature()?;
            items.push(ForeignItem::Fn(sig));
        } else if self.eat(Token::Indent) {
            loop {
                self.skip_newlines();
                if *self.peek() == Token::Dedent || self.at_end() {
                    break;
                }
                if *self.peek() == Token::Fn {
                    let sig = self.parse_fn_signature()?;
                    items.push(ForeignItem::Fn(sig));
                } else {
                    self.synchronize();
                }
                self.skip_newlines();
            }
            self.eat(Token::Dedent);
        }

        let end = self.current_span();
        Ok(ForeignBlock {
            abi,
            items,
            span: start.merge(end),
        })
    }

    fn parse_fn_signature(&mut self) -> Result<FnSignature, ()> {
        let start = self.current_span();
        self.expect(Token::Fn)?;
        let name_tok = self.expect(Token::LowerIdent)?;

        let type_params = if *self.peek() == Token::LBracket {
            self.parse_type_params()?
        } else {
            vec![]
        };

        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        loop {
            if *self.peek() == Token::RParen {
                break;
            }
            params.push(self.parse_type_expr()?);
            if !self.eat(Token::Comma) {
                break;
            }
        }
        self.expect(Token::RParen)?;

        self.expect(Token::Arrow)?;
        let return_type = self.parse_type_expr()?;

        let effects = if *self.peek() == Token::With {
            self.advance();
            self.parse_effect_list()?
        } else {
            vec![]
        };

        let end = self.current_span();
        Ok(FnSignature {
            name: name_tok.text.clone(),
            type_params,
            params,
            return_type,
            effects,
            span: start.merge(end),
        })
    }

    // ── Type expressions ────────────────────────────────────

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ()> {
        let start = self.current_span();

        match self.peek() {
            Token::Fn => {
                self.advance();
                self.expect(Token::LParen)?;
                let mut params = Vec::new();
                loop {
                    if *self.peek() == Token::RParen {
                        break;
                    }
                    params.push(self.parse_type_expr()?);
                    if !self.eat(Token::Comma) {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                self.expect(Token::Arrow)?;
                let return_type = Box::new(self.parse_type_expr()?);
                let effects = if *self.peek() == Token::With {
                    self.advance();
                    self.parse_effect_list()?
                } else {
                    vec![]
                };
                let end = self.current_span();
                Ok(TypeExpr::Fn {
                    params,
                    return_type,
                    effects,
                    span: start.merge(end),
                })
            }
            Token::Own => {
                self.advance();
                let inner = Box::new(self.parse_type_expr()?);
                let end = self.current_span();
                Ok(TypeExpr::Owned {
                    inner,
                    span: start.merge(end),
                })
            }
            Token::Ref => {
                self.advance();
                let inner = Box::new(self.parse_type_expr()?);
                let end = self.current_span();
                Ok(TypeExpr::Borrowed {
                    inner,
                    span: start.merge(end),
                })
            }
            Token::LParen => {
                self.advance();
                if self.eat(Token::RParen) {
                    Ok(TypeExpr::Unit { span: start })
                } else {
                    let ty = self.parse_type_expr()?;
                    if self.eat(Token::Comma) {
                        // Tuple type
                        let mut elements = vec![ty];
                        loop {
                            if *self.peek() == Token::RParen {
                                break;
                            }
                            elements.push(self.parse_type_expr()?);
                            if !self.eat(Token::Comma) {
                                break;
                            }
                        }
                        self.expect(Token::RParen)?;
                        let end = self.current_span();
                        Ok(TypeExpr::Tuple {
                            elements,
                            span: start.merge(end),
                        })
                    } else {
                        self.expect(Token::RParen)?;
                        Ok(ty)
                    }
                }
            }
            Token::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                let mut row_var = None;
                loop {
                    self.skip_newlines();
                    if *self.peek() == Token::RBrace {
                        break;
                    }
                    if *self.peek() == Token::Pipe {
                        self.advance();
                        if let Token::LowerIdent = self.peek() {
                            let tok = self.advance();
                            row_var = Some(tok.text.clone());
                        }
                        break;
                    }
                    let fstart = self.current_span();
                    let fname_tok = self.expect(Token::LowerIdent)?;
                    self.expect(Token::Colon)?;
                    let fty = self.parse_type_expr()?;
                    let fend = self.current_span();
                    fields.push(FieldDef {
                        name: fname_tok.text.clone(),
                        ty: fty,
                        span: fstart.merge(fend),
                    });
                    self.eat(Token::Comma);
                }
                self.expect(Token::RBrace)?;
                let end = self.current_span();
                Ok(TypeExpr::Record {
                    fields,
                    row_var,
                    span: start.merge(end),
                })
            }
            Token::UpperIdent => {
                let name = self.parse_qualified_name()?;
                let args = if *self.peek() == Token::LBracket {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == Token::RBracket {
                            break;
                        }
                        args.push(self.parse_type_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    self.expect(Token::RBracket)?;
                    args
                } else {
                    vec![]
                };
                let end = self.current_span();
                Ok(TypeExpr::Named {
                    name,
                    args,
                    span: start.merge(end),
                })
            }
            Token::LowerIdent => {
                let tok = self.advance();
                Ok(TypeExpr::Var {
                    name: tok.text.clone(),
                    span: tok.span,
                })
            }
            _ => {
                self.diagnostics.emit(
                    Diagnostic::error(format!("expected type, found '{}'", self.peek()))
                        .with_label(start, "here"),
                );
                Err(())
            }
        }
    }

    // ── Expression parsing (Pratt) ──────────────────────────

    fn parse_expr(&mut self) -> Result<Expr, ()> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ()> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Postfix operators
            match self.peek() {
                Token::Question => {
                    let ((), r_bp) = postfix_bp(&Token::Question);
                    if r_bp < min_bp {
                        break;
                    }
                    let op_tok = self.advance();
                    let span = lhs.span().merge(op_tok.span);
                    lhs = Expr::Try {
                        expr: Box::new(lhs),
                        span,
                    };
                    continue;
                }
                Token::Dot => {
                    let ((), r_bp) = postfix_bp(&Token::Dot);
                    if r_bp < min_bp {
                        break;
                    }
                    self.advance();
                    if *self.peek() == Token::LowerIdent {
                        let field_tok = self.advance();
                        let span = lhs.span().merge(field_tok.span);
                        lhs = Expr::FieldAccess {
                            expr: Box::new(lhs),
                            field: field_tok.text.clone(),
                            span,
                        };
                        continue;
                    }
                }
                Token::LParen => {
                    let ((), r_bp) = postfix_bp(&Token::LParen);
                    if r_bp < min_bp {
                        break;
                    }
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        args.push(self.parse_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(Token::RParen)?;
                    let span = lhs.span().merge(end.span);
                    lhs = Expr::App {
                        func: Box::new(lhs),
                        args,
                        span,
                    };
                    continue;
                }
                _ => {}
            }

            // Infix operators
            if let Some((l_bp, r_bp)) = infix_bp(self.peek()) {
                if l_bp < min_bp {
                    break;
                }

                let op_tok = self.advance();
                self.skip_newlines();

                match &op_tok.token {
                    Token::PipeRight => {
                        let rhs = self.parse_expr_bp(r_bp)?;
                        let span = lhs.span().merge(rhs.span());
                        lhs = Expr::Pipeline {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        };
                    }
                    Token::ComposeRight => {
                        let rhs = self.parse_expr_bp(r_bp)?;
                        let span = lhs.span().merge(rhs.span());
                        lhs = Expr::Compose {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        };
                    }
                    _ => {
                        if let Some(op) = token_to_binop(&op_tok.token) {
                            let rhs = self.parse_expr_bp(r_bp)?;
                            let span = lhs.span().merge(rhs.span());
                            lhs = Expr::BinOp {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                                span,
                            };
                        }
                    }
                }
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ()> {
        match self.peek().clone() {
            Token::IntLiteral
            | Token::HexIntLiteral
            | Token::BinIntLiteral
            | Token::OctIntLiteral => {
                let tok = self.advance();
                Ok(Expr::IntLit {
                    value: tok.text.clone(),
                    span: tok.span,
                })
            }
            Token::FloatLiteral => {
                let tok = self.advance();
                Ok(Expr::FloatLit {
                    value: tok.text.clone(),
                    span: tok.span,
                })
            }
            Token::StringLiteral => {
                let tok = self.advance();
                let raw = strip_quotes(&tok.text);
                Ok(Expr::StringLit {
                    segments: vec![StringSegment::Literal(raw)],
                    span: tok.span,
                })
            }
            Token::CharLiteral => {
                let tok = self.advance();
                let ch = parse_char_literal(&tok.text);
                Ok(Expr::CharLit {
                    value: ch,
                    span: tok.span,
                })
            }
            Token::True => {
                let tok = self.advance();
                Ok(Expr::BoolLit {
                    value: true,
                    span: tok.span,
                })
            }
            Token::False => {
                let tok = self.advance();
                Ok(Expr::BoolLit {
                    value: false,
                    span: tok.span,
                })
            }
            Token::LowerIdent => {
                let tok = self.advance();
                let id = self.next_node_id();
                Ok(Expr::Var {
                    name: tok.text.clone(),
                    id,
                    span: tok.span,
                })
            }
            Token::UpperIdent => {
                let name = self.parse_qualified_name()?;
                let id = self.next_node_id();
                let span = name.span;

                // Check for constructor call
                if *self.peek() == Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        args.push(self.parse_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(Token::RParen)?;
                    return Ok(Expr::App {
                        func: Box::new(Expr::Constructor { name, id, span }),
                        args,
                        span: span.merge(end.span),
                    });
                }

                Ok(Expr::Constructor { name, id, span })
            }
            Token::LParen => {
                let start = self.advance();
                if self.eat(Token::RParen) {
                    return Ok(Expr::UnitLit { span: start.span });
                }
                let expr = self.parse_expr()?;
                // Check for tuple
                if self.eat(Token::Comma) {
                    let mut elements = vec![expr];
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(Token::RParen)?;
                    return Ok(Expr::TupleLit {
                        elements,
                        span: start.span.merge(end.span),
                    });
                }
                // Check for type annotation
                if self.eat(Token::Colon) {
                    let ty = self.parse_type_expr()?;
                    let end = self.expect(Token::RParen)?;
                    return Ok(Expr::Annotation {
                        expr: Box::new(expr),
                        ty,
                        span: start.span.merge(end.span),
                    });
                }
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => {
                let start = self.advance();
                let mut elements = Vec::new();
                loop {
                    if *self.peek() == Token::RBracket {
                        break;
                    }
                    elements.push(self.parse_expr()?);
                    if !self.eat(Token::Comma) {
                        break;
                    }
                }
                let end = self.expect(Token::RBracket)?;
                Ok(Expr::ListLit {
                    elements,
                    span: start.span.merge(end.span),
                })
            }
            Token::LBrace => self.parse_record_or_update(),
            Token::Minus => {
                let tok = self.advance();
                let expr = self.parse_expr_bp(10)?;
                let span = tok.span.merge(expr.span());
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    span,
                })
            }
            Token::Bang => {
                let tok = self.advance();
                let expr = self.parse_expr_bp(10)?;
                let span = tok.span.merge(expr.span());
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                })
            }
            Token::Let => self.parse_let_expr(),
            Token::Use => self.parse_use_expr(),
            Token::If => self.parse_if_expr(),
            Token::Match => self.parse_match_expr(),
            Token::Fn => self.parse_lambda_expr(),
            Token::Receive => self.parse_receive_expr(),
            _ => {
                let span = self.current_span();
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "unexpected token '{}' in expression",
                        self.peek()
                    ))
                    .with_label(span, "here"),
                );
                Err(())
            }
        }
    }

    fn parse_record_or_update(&mut self) -> Result<Expr, ()> {
        let start = self.advance(); // consume {
        self.skip_newlines();

        // Empty record
        if *self.peek() == Token::RBrace {
            let end = self.advance();
            return Ok(Expr::RecordLit {
                fields: vec![],
                span: start.span.merge(end.span),
            });
        }

        // Check if this is `ident = expr` (record lit) or `expr | ...` (record update)
        if *self.peek() == Token::LowerIdent {
            let saved_pos = self.pos;
            let ident_tok = self.advance();

            if self.eat(Token::Eq) {
                // Record literal
                let val = self.parse_expr()?;
                let mut fields = vec![(ident_tok.text.clone(), val)];
                while self.eat(Token::Comma) {
                    self.skip_newlines();
                    if *self.peek() == Token::RBrace {
                        break;
                    }
                    let fname_tok = self.expect(Token::LowerIdent)?;
                    self.expect(Token::Eq)?;
                    let fval = self.parse_expr()?;
                    fields.push((fname_tok.text.clone(), fval));
                }
                self.skip_newlines();
                let end = self.expect(Token::RBrace)?;
                return Ok(Expr::RecordLit {
                    fields,
                    span: start.span.merge(end.span),
                });
            } else if *self.peek() == Token::Pipe {
                // Record update: { base | field = val }
                self.advance();
                let base = Expr::Var {
                    name: ident_tok.text.clone(),
                    id: self.next_node_id(),
                    span: ident_tok.span,
                };
                let mut updates = Vec::new();
                loop {
                    self.skip_newlines();
                    if *self.peek() == Token::RBrace {
                        break;
                    }
                    let fname_tok = self.expect(Token::LowerIdent)?;
                    self.expect(Token::Eq)?;
                    let fval = self.parse_expr()?;
                    updates.push((fname_tok.text.clone(), fval));
                    if !self.eat(Token::Comma) {
                        break;
                    }
                }
                self.skip_newlines();
                let end = self.expect(Token::RBrace)?;
                return Ok(Expr::RecordUpdate {
                    base: Box::new(base),
                    updates,
                    span: start.span.merge(end.span),
                });
            } else {
                // Restore and try as general expression
                self.pos = saved_pos;
            }
        }

        // General: parse expression, expect | for update
        let base = self.parse_expr()?;
        if self.eat(Token::Pipe) {
            let mut updates = Vec::new();
            loop {
                self.skip_newlines();
                if *self.peek() == Token::RBrace {
                    break;
                }
                let fname_tok = self.expect(Token::LowerIdent)?;
                self.expect(Token::Eq)?;
                let fval = self.parse_expr()?;
                updates.push((fname_tok.text.clone(), fval));
                if !self.eat(Token::Comma) {
                    break;
                }
            }
            self.skip_newlines();
            let end = self.expect(Token::RBrace)?;
            Ok(Expr::RecordUpdate {
                base: Box::new(base),
                updates,
                span: start.span.merge(end.span),
            })
        } else {
            let span = self.current_span();
            self.diagnostics.emit(
                Diagnostic::error("expected '|' or '=' in record expression")
                    .with_label(span, "here"),
            );
            Err(())
        }
    }

    fn parse_let_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::Let)?;
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(Token::Eq)?;
        self.skip_newlines();
        let value = self.parse_expr()?;
        self.skip_newlines();

        let body = if self.at_end() || *self.peek() == Token::Dedent {
            Expr::UnitLit {
                span: self.current_span(),
            }
        } else {
            self.parse_expr()?
        };

        let end_span = body.span();
        Ok(Expr::Let {
            pattern,
            ty,
            value: Box::new(value),
            body: Box::new(body),
            span: start.merge(end_span),
        })
    }

    fn parse_use_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::Use)?;
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(Token::Eq)?;
        self.skip_newlines();
        let value = self.parse_expr()?;
        self.skip_newlines();

        let body = if self.at_end() || *self.peek() == Token::Dedent {
            Expr::UnitLit {
                span: self.current_span(),
            }
        } else {
            self.parse_expr()?
        };

        let end_span = body.span();
        Ok(Expr::Use {
            pattern,
            ty,
            value: Box::new(value),
            body: Box::new(body),
            span: start.merge(end_span),
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::If)?;
        let condition = self.parse_expr()?;
        self.expect(Token::Then)?;
        self.skip_newlines();

        let then_branch = if self.eat(Token::Indent) {
            self.parse_block()?
        } else {
            self.parse_expr()?
        };

        self.skip_newlines();
        self.expect(Token::Else)?;
        self.skip_newlines();

        let else_branch = if self.eat(Token::Indent) {
            self.parse_block()?
        } else {
            self.parse_expr()?
        };

        let end_span = else_branch.span();
        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span: start.merge(end_span),
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(Token::With)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        let indent = self.eat(Token::Indent);

        while self.eat(Token::Pipe) {
            self.skip_newlines();
            let arm_start = self.current_span();
            let pattern = self.parse_pattern()?;
            let guard = if *self.peek() == Token::If {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(Token::Arrow)?;
            self.skip_newlines();
            let body = if self.eat(Token::Indent) {
                self.parse_block()?
            } else {
                self.parse_expr()?
            };
            let arm_end = body.span();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_start.merge(arm_end),
            });
            self.skip_newlines();
        }

        if indent {
            self.eat(Token::Dedent);
        }

        let end = self.current_span();
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.merge(end),
        })
    }

    fn parse_lambda_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::Fn)?;

        let params = if *self.peek() == Token::LParen {
            self.advance();
            let ps = self.parse_param_list()?;
            self.expect(Token::RParen)?;
            ps
        } else if *self.peek() == Token::LowerIdent {
            let param = self.parse_param()?;
            vec![param]
        } else {
            vec![]
        };

        self.expect(Token::Arrow)?;
        self.skip_newlines();
        let body = self.parse_expr()?;
        let end_span = body.span();

        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
            span: start.merge(end_span),
        })
    }

    fn parse_receive_expr(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        self.expect(Token::Receive)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        let indent = self.eat(Token::Indent);

        while self.eat(Token::Pipe) {
            self.skip_newlines();
            let arm_start = self.current_span();
            let pattern = self.parse_pattern()?;
            self.expect(Token::Arrow)?;
            self.skip_newlines();
            let body = self.parse_expr()?;
            let arm_end = body.span();
            arms.push(MatchArm {
                pattern,
                guard: None,
                body,
                span: arm_start.merge(arm_end),
            });
            self.skip_newlines();
        }

        if indent {
            self.eat(Token::Dedent);
        }

        let end = self.current_span();
        Ok(Expr::Receive {
            arms,
            timeout: None,
            span: start.merge(end),
        })
    }

    fn parse_block(&mut self) -> Result<Expr, ()> {
        let start = self.current_span();
        let mut exprs = Vec::new();

        loop {
            self.skip_newlines();
            if *self.peek() == Token::Dedent || self.at_end() {
                break;
            }
            let expr = self.parse_expr()?;
            exprs.push(expr);
            self.skip_newlines();
        }

        self.eat(Token::Dedent);

        if exprs.len() == 1 {
            Ok(exprs.into_iter().next().unwrap())
        } else if exprs.is_empty() {
            Ok(Expr::UnitLit { span: start })
        } else {
            let end = exprs.last().unwrap().span();
            Ok(Expr::Block {
                exprs,
                span: start.merge(end),
            })
        }
    }

    // ── Pattern parsing ─────────────────────────────────────

    fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        let start = self.current_span();

        match self.peek().clone() {
            Token::LowerIdent => {
                let tok = self.advance();
                if tok.text.as_str() == "_" {
                    Ok(Pattern::Wildcard { span: tok.span })
                } else {
                    Ok(Pattern::Var {
                        name: tok.text.clone(),
                        id: self.next_node_id(),
                        span: tok.span,
                    })
                }
            }
            Token::UpperIdent => {
                let name = self.parse_qualified_name()?;
                if self.eat(Token::LParen) {
                    let mut fields = Vec::new();
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        fields.push(self.parse_pattern()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(Token::RParen)?;
                    Ok(Pattern::Constructor {
                        name,
                        fields,
                        span: start.merge(end.span),
                    })
                } else {
                    Ok(Pattern::Constructor {
                        name,
                        fields: vec![],
                        span: start,
                    })
                }
            }
            Token::True => {
                let tok = self.advance();
                Ok(Pattern::Literal {
                    expr: Box::new(Expr::BoolLit {
                        value: true,
                        span: tok.span,
                    }),
                    span: tok.span,
                })
            }
            Token::False => {
                let tok = self.advance();
                Ok(Pattern::Literal {
                    expr: Box::new(Expr::BoolLit {
                        value: false,
                        span: tok.span,
                    }),
                    span: tok.span,
                })
            }
            Token::IntLiteral
            | Token::HexIntLiteral
            | Token::BinIntLiteral
            | Token::OctIntLiteral => {
                let tok = self.advance();
                Ok(Pattern::Literal {
                    expr: Box::new(Expr::IntLit {
                        value: tok.text.clone(),
                        span: tok.span,
                    }),
                    span: tok.span,
                })
            }
            Token::StringLiteral => {
                let tok = self.advance();
                let raw = strip_quotes(&tok.text);
                Ok(Pattern::Literal {
                    expr: Box::new(Expr::StringLit {
                        segments: vec![StringSegment::Literal(raw)],
                        span: tok.span,
                    }),
                    span: tok.span,
                })
            }
            Token::LParen => {
                self.advance();
                if self.eat(Token::RParen) {
                    return Ok(Pattern::Literal {
                        expr: Box::new(Expr::UnitLit { span: start }),
                        span: start,
                    });
                }
                let first = self.parse_pattern()?;
                if self.eat(Token::Comma) {
                    let mut elements = vec![first];
                    loop {
                        if *self.peek() == Token::RParen {
                            break;
                        }
                        elements.push(self.parse_pattern()?);
                        if !self.eat(Token::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(Token::RParen)?;
                    Ok(Pattern::Tuple {
                        elements,
                        span: start.merge(end.span),
                    })
                } else {
                    self.expect(Token::RParen)?;
                    Ok(first)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                let mut rest = None;
                loop {
                    if *self.peek() == Token::RBracket {
                        break;
                    }
                    if self.eat(Token::DotDot) {
                        rest = Some(Box::new(self.parse_pattern()?));
                        break;
                    }
                    elements.push(self.parse_pattern()?);
                    if !self.eat(Token::Comma) {
                        if self.eat(Token::DotDot) {
                            rest = Some(Box::new(self.parse_pattern()?));
                        }
                        break;
                    }
                }
                let end = self.expect(Token::RBracket)?;
                Ok(Pattern::List {
                    elements,
                    rest,
                    span: start.merge(end.span),
                })
            }
            Token::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                let mut has_rest = false;
                loop {
                    self.skip_newlines();
                    if *self.peek() == Token::RBrace {
                        break;
                    }
                    if self.eat(Token::DotDot) {
                        has_rest = true;
                        break;
                    }
                    let fname_tok = self.expect(Token::LowerIdent)?;
                    let pat = if self.eat(Token::Eq) {
                        self.parse_pattern()?
                    } else {
                        Pattern::Var {
                            name: fname_tok.text.clone(),
                            id: self.next_node_id(),
                            span: fname_tok.span,
                        }
                    };
                    fields.push((fname_tok.text.clone(), pat));
                    if !self.eat(Token::Comma) {
                        break;
                    }
                }
                let end = self.expect(Token::RBrace)?;
                Ok(Pattern::Record {
                    fields,
                    rest: has_rest,
                    span: start.merge(end.span),
                })
            }
            Token::Caret => {
                self.advance();
                let name_tok = self.expect(Token::LowerIdent)?;
                Ok(Pattern::Pin {
                    name: name_tok.text.clone(),
                    id: self.next_node_id(),
                    span: start.merge(name_tok.span),
                })
            }
            _ => {
                self.diagnostics.emit(
                    Diagnostic::error(format!(
                        "unexpected token '{}' in pattern",
                        self.peek()
                    ))
                    .with_label(start, "here"),
                );
                Err(())
            }
        }
    }
}

// ── Operator precedence tables ──────────────────────────────

fn infix_bp(token: &Token) -> Option<(u8, u8)> {
    match token {
        Token::PipeRight => Some((1, 2)),
        Token::ComposeRight => Some((4, 3)),
        Token::PipePipe => Some((5, 6)),
        Token::AmpAmp => Some((7, 8)),
        Token::EqEq | Token::BangEq => Some((9, 10)),
        Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => Some((11, 12)),
        Token::PlusPlus | Token::Diamond => Some((14, 13)),
        Token::Plus | Token::Minus => Some((15, 16)),
        Token::Star | Token::Slash | Token::Percent => Some((17, 18)),
        _ => None,
    }
}

fn postfix_bp(token: &Token) -> ((), u8) {
    match token {
        Token::Question => ((), 21),
        Token::Dot => ((), 23),
        Token::LParen => ((), 23),
        _ => ((), 0),
    }
}

fn token_to_binop(token: &Token) -> Option<BinOp> {
    match token {
        Token::Plus => Some(BinOp::Add),
        Token::Minus => Some(BinOp::Sub),
        Token::Star => Some(BinOp::Mul),
        Token::Slash => Some(BinOp::Div),
        Token::Percent => Some(BinOp::Mod),
        Token::EqEq => Some(BinOp::Eq),
        Token::BangEq => Some(BinOp::Neq),
        Token::Lt => Some(BinOp::Lt),
        Token::Gt => Some(BinOp::Gt),
        Token::LtEq => Some(BinOp::LtEq),
        Token::GtEq => Some(BinOp::GtEq),
        Token::AmpAmp => Some(BinOp::And),
        Token::PipePipe => Some(BinOp::Or),
        Token::PlusPlus => Some(BinOp::Concat),
        Token::Diamond => Some(BinOp::Append),
        _ => None,
    }
}

fn strip_quotes(s: &str) -> SmolStr {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        SmolStr::new(&s[1..s.len() - 1])
    } else {
        SmolStr::new(s)
    }
}

fn parse_char_literal(s: &str) -> char {
    let inner = &s[1..s.len() - 1];
    if inner.starts_with('\\') {
        match inner.chars().nth(1) {
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('\\') => '\\',
            Some('0') => '\0',
            Some('\'') => '\'',
            _ => '?',
        }
    } else {
        inner.chars().next().unwrap_or('?')
    }
}

/// Parse a source string into a SourceFile AST.
pub fn parse(source: &str, file_id: FileId) -> (SourceFile, Vec<Diagnostic>) {
    let parser = Parser::new(source, file_id);
    parser.parse_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let src = "fn add(x: Int, y: Int) -> Int = x + y";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        assert_eq!(file.items.len(), 1);
        match &file.items[0] {
            Item::FnDef(f) => {
                assert_eq!(f.name.as_str(), "add");
                assert_eq!(f.params.len(), 2);
            }
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn test_parse_type_decl() {
        let src = "type Option[a] =\n  | Some(a)\n  | None\n";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        assert_eq!(file.items.len(), 1);
        match &file.items[0] {
            Item::TypeDef(t) => {
                assert_eq!(t.name.as_str(), "Option");
                assert_eq!(t.type_params.len(), 1);
                match &t.body {
                    TypeBody::Sum(variants) => {
                        assert_eq!(variants.len(), 2);
                        assert_eq!(variants[0].name.as_str(), "Some");
                        assert_eq!(variants[1].name.as_str(), "None");
                    }
                    _ => panic!("expected Sum type"),
                }
            }
            _ => panic!("expected TypeDef"),
        }
    }

    #[test]
    fn test_parse_expression_precedence() {
        let src = "fn main() -> Int = 1 + 2 * 3";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        match &file.items[0] {
            Item::FnDef(f) => match &f.body {
                Expr::BinOp {
                    op: BinOp::Add,
                    rhs,
                    ..
                } => match rhs.as_ref() {
                    Expr::BinOp {
                        op: BinOp::Mul, ..
                    } => {}
                    other => panic!("expected Mul, got {:?}", other),
                },
                other => panic!("expected Add, got {:?}", other),
            },
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn test_parse_if_expr() {
        let src = "fn f(x: Int) -> Int = if x > 0 then x else 0 - x";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        match &file.items[0] {
            Item::FnDef(f) => match &f.body {
                Expr::If { .. } => {}
                other => panic!("expected If, got {:?}", other),
            },
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let src = "fn f() -> List[Int] = [1, 2, 3]";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        match &file.items[0] {
            Item::FnDef(f) => match &f.body {
                Expr::ListLit { elements, .. } => {
                    assert_eq!(elements.len(), 3);
                }
                other => panic!("expected ListLit, got {:?}", other),
            },
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn test_parse_import() {
        let src = "import Std.IO\n\nfn main() -> Unit = ()";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        assert_eq!(file.imports.len(), 1);
        assert_eq!(file.imports[0].path.segments.len(), 2);
    }

    #[test]
    fn test_parse_pipe() {
        let src = "fn f(x: Int) -> Int = x |> add_one |> double";
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        match &file.items[0] {
            Item::FnDef(f) => match &f.body {
                Expr::Pipeline { .. } => {}
                other => panic!("expected Pipeline, got {:?}", other),
            },
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn test_parse_pattern_matching() {
        let src = r#"fn describe(shape: Shape) -> String = match shape with
  | Circle(r) -> "circle"
  | Rectangle(w, h) -> "rect"
"#;
        let (file, diags) = parse(src, FileId(0));
        assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
        match &file.items[0] {
            Item::FnDef(f) => match &f.body {
                Expr::Match { arms, .. } => {
                    assert_eq!(arms.len(), 2);
                }
                other => panic!("expected Match, got {:?}", other),
            },
            _ => panic!("expected FnDef"),
        }
    }
}

use super::ast::*;
use super::error::{self, Span};
use super::token::{Token, SpannedToken};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
    file: String,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>, file: &str) -> Self {
        Parser {
            tokens,
            pos: 0,
            file: file.to_string(),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span.clone()
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].token;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> error::Result<()> {
        let span = self.span();
        let tok = self.advance().clone();
        if tok != *expected {
            error::err(format!("expected {:?}, got {:?}", expected, tok), &self.file, Some(span))
        } else {
            Ok(())
        }
    }

    fn eat_ident(&mut self) -> error::Result<String> {
        let span = self.span();
        let tok = self.advance().clone();
        if let Token::Ident(s) = tok {
            Ok(s)
        } else {
            error::err(format!("expected identifier, got {:?}", tok), &self.file, Some(span))
        }
    }

    fn eat_string(&mut self) -> error::Result<String> {
        let span = self.span();
        let tok = self.advance().clone();
        if let Token::StringLit(s) = tok {
            Ok(s)
        } else {
            error::err(format!("expected string literal, got {:?}", tok), &self.file, Some(span))
        }
    }

    pub fn parse_program(&mut self) -> error::Result<Program> {
        let mut items = Vec::new();
        while *self.peek() != Token::Eof {
            items.push(self.parse_top_level()?);
        }
        Ok(Program { items })
    }

    fn parse_top_level(&mut self) -> error::Result<TopLevel> {
        // Collect doc comments
        let mut doc = None;
        while let Token::DocComment(ref s) = self.peek().clone() {
            let s = s.clone();
            self.advance();
            doc = Some(match doc {
                Some(prev) => format!("{}\n{}", prev, s),
                None => s,
            });
        }

        match self.peek().clone() {
            Token::Fn => {
                let fdef = self.parse_fn_def_inner(false, doc)?;
                Ok(TopLevel::FnDef(fdef))
            }
            Token::Pub => {
                self.advance(); // eat pub
                match self.peek().clone() {
                    Token::Fn => {
                        let fdef = self.parse_fn_def_inner(true, doc)?;
                        Ok(TopLevel::FnDef(fdef))
                    }
                    _ => {
                        let span = self.span();
                        error::err(format!("expected fn after pub, got {:?}", self.peek()), &self.file, Some(span))
                    }
                }
            }
            Token::Type => {
                let tdef = self.parse_type_def()?;
                Ok(TopLevel::TypeDef(tdef))
            }
            Token::Foreign => {
                let fdef = self.parse_foreign_fn()?;
                Ok(TopLevel::ForeignFn(fdef))
            }
            Token::Import => {
                let idef = self.parse_import()?;
                Ok(TopLevel::Import(idef))
            }
            Token::Const => {
                let cdef = self.parse_const()?;
                Ok(TopLevel::Const(cdef))
            }
            Token::Trait => {
                let tdef = self.parse_trait_def()?;
                Ok(TopLevel::TraitDef(tdef))
            }
            Token::Opaque => {
                let odef = self.parse_opaque_type()?;
                Ok(TopLevel::OpaqueType(odef))
            }
            _ => {
                let span = self.span();
                error::err(format!("expected fn, type, foreign, import, const, trait, or opaque, got {:?}", self.peek()), &self.file, Some(span))
            }
        }
    }

    fn parse_fn_def(&mut self) -> error::Result<FnDef> {
        self.parse_fn_def_inner(false, None)
    }

    fn parse_fn_def_inner(&mut self, is_pub: bool, doc_comment: Option<String>) -> error::Result<FnDef> {
        self.expect(&Token::Fn)?;
        let name = self.eat_ident()?;
        // Parse optional type params: fn foo<T, U>(...)
        let type_params = self.parse_optional_type_params()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let (ret_ty, effect) = if *self.peek() == Token::Arrow {
            self.advance();
            // Check for effect annotation: -> IO Type, -> Process Type
            let eff = match self.peek() {
                Token::Ident(name) if name == "IO" || name == "io"
                    || name == "Process" || name == "process" => {
                    let e = name.clone();
                    // Only treat as effect if followed by a type or {
                    // Peek ahead: if next-next is a type name or {, it's an effect annotation
                    let next_pos = self.pos + 1;
                    if next_pos < self.tokens.len() {
                        match &self.tokens[next_pos].token {
                            Token::Ident(_) | Token::LBrace | Token::LParen => {
                                self.advance(); // consume the effect keyword
                                Some(e)
                            }
                            _ => None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let ty = Some(self.parse_type()?);
            (ty, eff)
        } else {
            (None, None)
        };
        let body = self.parse_block_expr()?;
        Ok(FnDef { name, params, ret_ty, body, is_pub, doc_comment, effect, type_params })
    }

    fn parse_optional_type_params(&mut self) -> error::Result<Vec<String>> {
        if *self.peek() == Token::Lt {
            self.advance(); // <
            let mut params = Vec::new();
            params.push(self.eat_ident()?);
            while *self.peek() == Token::Comma {
                self.advance();
                params.push(self.eat_ident()?);
            }
            self.expect(&Token::Gt)?;
            Ok(params)
        } else {
            Ok(vec![])
        }
    }

    fn parse_type_def(&mut self) -> error::Result<TypeDef> {
        self.expect(&Token::Type)?;
        let name = self.eat_ident()?;
        let type_params = self.parse_optional_type_params()?;
        self.expect(&Token::Eq)?;
        let mut variants = Vec::new();
        // Expect | Variant | Variant ...
        while *self.peek() == Token::Pipe {
            self.advance(); // eat |
            let vname = self.eat_ident()?;
            let mut fields = Vec::new();
            if *self.peek() == Token::LParen {
                self.advance();
                if *self.peek() != Token::RParen {
                    fields.push(self.parse_type()?);
                    while *self.peek() == Token::Comma {
                        self.advance();
                        fields.push(self.parse_type()?);
                    }
                }
                self.expect(&Token::RParen)?;
            }
            variants.push(Variant { name: vname, fields });
        }
        Ok(TypeDef { name, variants, type_params })
    }

    fn parse_foreign_fn(&mut self) -> error::Result<ForeignFnDef> {
        self.expect(&Token::Foreign)?;
        let module = self.eat_string()?;
        self.expect(&Token::Fn)?;
        let name = self.eat_ident()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret_ty = if *self.peek() == Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        Ok(ForeignFnDef { module, name, params, ret_ty })
    }

    fn parse_params(&mut self) -> error::Result<Vec<Param>> {
        let mut params = Vec::new();
        if *self.peek() == Token::RParen {
            return Ok(params);
        }
        params.push(self.parse_param()?);
        while *self.peek() == Token::Comma {
            self.advance();
            params.push(self.parse_param()?);
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> error::Result<Param> {
        let name = self.eat_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        Ok(Param { name, ty })
    }

    fn parse_type(&mut self) -> error::Result<Type> {
        if *self.peek() == Token::Fn {
            self.advance();
            self.expect(&Token::LParen)?;
            let mut params = Vec::new();
            if *self.peek() != Token::RParen {
                params.push(self.parse_type()?);
                while *self.peek() == Token::Comma {
                    self.advance();
                    params.push(self.parse_type()?);
                }
            }
            self.expect(&Token::RParen)?;
            self.expect(&Token::Arrow)?;
            let ret = self.parse_type()?;
            Ok(Type::FnType(params, Box::new(ret)))
        } else {
            let name = self.eat_ident()?;
            Ok(Type::Named(name))
        }
    }

    fn parse_block_expr(&mut self) -> error::Result<Expr> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        let mut final_expr: Option<Expr> = None;

        while *self.peek() != Token::RBrace {
            if *self.peek() == Token::Let {
                stmts.push(self.parse_let_stmt()?);
            } else {
                let expr = self.parse_expr()?;
                // If next is RBrace, this is the final expression
                if *self.peek() == Token::RBrace {
                    final_expr = Some(expr);
                } else {
                    stmts.push(Stmt::Expr(expr));
                }
            }
        }
        self.expect(&Token::RBrace)?;

        if stmts.is_empty() && final_expr.is_some() {
            Ok(final_expr.unwrap())
        } else {
            Ok(Expr::Block(stmts, final_expr.map(Box::new)))
        }
    }

    fn parse_let_stmt(&mut self) -> error::Result<Stmt> {
        self.expect(&Token::Let)?;
        let name = self.eat_ident()?;
        // Optional type annotation: let x: Int = ...
        if *self.peek() == Token::Colon {
            self.advance();
            let ty = self.parse_type()?;
            self.expect(&Token::Eq)?;
            let expr = self.parse_expr()?;
            Ok(Stmt::LetTyped(name, ty, expr))
        } else {
            self.expect(&Token::Eq)?;
            let expr = self.parse_expr()?;
            Ok(Stmt::Let(name, expr))
        }
    }

    // Pratt parser for expressions
    fn parse_expr(&mut self) -> error::Result<Expr> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_comparison()?;
        while *self.peek() == Token::PipeOp {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Pipe(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_concat()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::Neq,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat()?;
            left = Expr::BinOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_add()?;
        while *self.peek() == Token::Concat {
            self.advance();
            let right = self.parse_add()?;
            left = Expr::BinOp(BinOp::Concat, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::BinOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_call()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_call()?;
            left = Expr::BinOp(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_call(&mut self) -> error::Result<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if *self.peek() == Token::LParen {
                self.advance();
                let mut args = Vec::new();
                if *self.peek() != Token::RParen {
                    args.push(self.parse_expr()?);
                    while *self.peek() == Token::Comma {
                        self.advance();
                        args.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RParen)?;
                expr = Expr::Call(Box::new(expr), args);
            } else if *self.peek() == Token::Dot {
                self.advance();
                let field = self.eat_ident()?;
                expr = Expr::FieldAccess(Box::new(expr), field);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> error::Result<Expr> {
        match self.peek().clone() {
            Token::Int(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Token::True => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBrace => {
                // Could be block or record literal
                self.parse_brace_expr()
            }
            Token::If => {
                self.parse_if()
            }
            Token::Match => {
                self.parse_match()
            }
            Token::Receive => {
                self.parse_receive_expr()
            }
            Token::Fn => {
                self.parse_lambda()
            }
            _ => {
                let span = self.span();
                error::err(format!("unexpected token {:?}", self.peek()), &self.file, Some(span))
            }
        }
    }

    fn parse_brace_expr(&mut self) -> error::Result<Expr> {
        // Look ahead to decide: record literal or block
        // Record: { name: expr, ... } or { expr | name: expr }
        // Block: { stmt; stmt; expr }
        // Heuristic: if we see Ident Colon, it's a record
        let saved = self.pos;
        self.expect(&Token::LBrace)?;

        if *self.peek() == Token::RBrace {
            self.expect(&Token::RBrace)?;
            return Ok(Expr::Block(vec![], None));
        }

        // Check for record literal: ident ':'
        if let Token::Ident(_) = self.peek().clone() {
            let next_pos = self.pos + 1;
            if next_pos < self.tokens.len() && self.tokens[next_pos].token == Token::Colon {
                // Record literal
                return self.parse_record_fields();
            }
            // Check for record update: ident '|' field: val
            if next_pos < self.tokens.len() && self.tokens[next_pos].token == Token::Pipe {
                let base_name = self.eat_ident()?;
                self.expect(&Token::Pipe)?;
                let mut fields = Vec::new();
                loop {
                    let name = self.eat_ident()?;
                    self.expect(&Token::Colon)?;
                    let val = self.parse_expr()?;
                    fields.push((name, val));
                    if *self.peek() == Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
                return Ok(Expr::RecordUpdate(Box::new(Expr::Ident(base_name)), fields));
            }
        }

        // Restore and parse as block
        self.pos = saved;
        self.parse_block_expr()
    }

    fn parse_record_fields(&mut self) -> error::Result<Expr> {
        let mut fields = Vec::new();
        loop {
            let name = self.eat_ident()?;
            self.expect(&Token::Colon)?;
            let val = self.parse_expr()?;
            fields.push((name, val));
            if *self.peek() == Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::Record(fields))
    }

    fn parse_if(&mut self) -> error::Result<Expr> {
        self.expect(&Token::If)?;
        let cond = self.parse_expr()?;
        let then = self.parse_block_expr()?;
        let else_ = if *self.peek() == Token::Else {
            self.advance();
            Some(Box::new(self.parse_block_expr()?))
        } else {
            None
        };
        Ok(Expr::If(Box::new(cond), Box::new(then), else_))
    }

    fn parse_match(&mut self) -> error::Result<Expr> {
        self.expect(&Token::Match)?;
        let scrutinee = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let arms = self.parse_match_arms()?;
        self.expect(&Token::RBrace)?;
        Ok(Expr::Match(Box::new(scrutinee), arms))
    }

    fn parse_receive_expr(&mut self) -> error::Result<Expr> {
        self.expect(&Token::Receive)?;
        self.expect(&Token::LBrace)?;
        let arms = self.parse_match_arms()?;
        self.expect(&Token::RBrace)?;
        Ok(Expr::Receive(arms))
    }

    fn parse_match_arms(&mut self) -> error::Result<Vec<MatchArm>> {
        let mut arms = Vec::new();
        while *self.peek() != Token::RBrace {
            let pattern = self.parse_pattern()?;
            // Optional guard: if <expr>
            let guard = if *self.peek() == Token::If {
                self.advance();
                Some(self.parse_comparison()?)
            } else {
                None
            };
            self.expect(&Token::FatArrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pattern, guard, body });
            // optional comma
            if *self.peek() == Token::Comma {
                self.advance();
            }
        }
        Ok(arms)
    }

    fn parse_pattern(&mut self) -> error::Result<Pattern> {
        match self.peek().clone() {
            Token::Ident(ref name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Ident(_) => {
                let name = self.eat_ident()?;
                let mut bindings = Vec::new();
                if *self.peek() == Token::LParen {
                    self.advance();
                    if *self.peek() != Token::RParen {
                        bindings.push(self.eat_ident()?);
                        while *self.peek() == Token::Comma {
                            self.advance();
                            bindings.push(self.eat_ident()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                Ok(Pattern::Variant(name, bindings))
            }
            Token::Int(n) => {
                self.advance();
                Ok(Pattern::IntLit(n))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Pattern::StringLit(s))
            }
            Token::True => {
                self.advance();
                Ok(Pattern::BoolLit(true))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::BoolLit(false))
            }
            _ => {
                let span = self.span();
                error::err(format!("expected pattern, got {:?}", self.peek()), &self.file, Some(span))
            }
        }
    }

    fn parse_lambda(&mut self) -> error::Result<Expr> {
        self.expect(&Token::Fn)?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret_ty = if *self.peek() == Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block_expr()?;
        Ok(Expr::Lambda(params, ret_ty, Box::new(body)))
    }

    // import Math.{add, sub}
    fn parse_import(&mut self) -> error::Result<ImportDef> {
        self.expect(&Token::Import)?;
        let mut module_path = Vec::new();
        module_path.push(self.eat_ident()?);
        while *self.peek() == Token::Dot {
            self.advance();
            // Check if next is { for import list
            if *self.peek() == Token::LBrace {
                break;
            }
            module_path.push(self.eat_ident()?);
        }
        // Parse import names: .{name1, name2}
        let mut names = Vec::new();
        if *self.peek() == Token::LBrace {
            self.advance();
            if *self.peek() != Token::RBrace {
                names.push(self.eat_ident()?);
                while *self.peek() == Token::Comma {
                    self.advance();
                    names.push(self.eat_ident()?);
                }
            }
            self.expect(&Token::RBrace)?;
        }
        Ok(ImportDef { module_path, names })
    }

    // const MAX = 100
    fn parse_const(&mut self) -> error::Result<ConstDef> {
        self.expect(&Token::Const)?;
        let name = self.eat_ident()?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(ConstDef { name, value })
    }

    // trait Show(a) { fn show(a) -> String }
    fn parse_trait_def(&mut self) -> error::Result<TraitDef> {
        self.expect(&Token::Trait)?;
        let name = self.eat_ident()?;
        self.expect(&Token::LParen)?;
        let type_param = self.eat_ident()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let mut methods = Vec::new();
        while *self.peek() != Token::RBrace {
            self.expect(&Token::Fn)?;
            let mname = self.eat_ident()?;
            self.expect(&Token::LParen)?;
            let params = self.parse_params()?;
            self.expect(&Token::RParen)?;
            self.expect(&Token::Arrow)?;
            let ret_ty = self.parse_type()?;
            methods.push(TraitMethod { name: mname, params, ret_ty });
        }
        self.expect(&Token::RBrace)?;
        Ok(TraitDef { name, type_param, methods })
    }

    // opaque type UserId = Int
    fn parse_opaque_type(&mut self) -> error::Result<OpaqueTypeDef> {
        self.expect(&Token::Opaque)?;
        self.expect(&Token::Type)?;
        let name = self.eat_ident()?;
        self.expect(&Token::Eq)?;
        let inner = self.parse_type()?;
        Ok(OpaqueTypeDef { name, inner })
    }
}

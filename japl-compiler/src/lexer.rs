use crate::error::{self, Span};
use crate::token::{Token, SpannedToken};

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    file: String,
}

impl Lexer {
    pub fn new(input: &str, file: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            file: file.to_string(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn span(&self) -> Span {
        Span { line: self.line, col: self.col }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek2() == Some('/') {
                // Check for doc comment (///)
                if self.input.get(self.pos + 2) == Some(&'/') {
                    break; // Don't skip doc comments - let tokenize handle them
                }
                // Regular line comment
                while let Some(c) = self.peek() {
                    if c == '\n' { break; }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> error::Result<String> {
        let span = self.span();
        self.advance(); // skip opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(s),
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        _ => return error::err("invalid escape", &self.file, Some(span)),
                    }
                }
                Some(ch) => s.push(ch),
                None => return error::err("unterminated string", &self.file, Some(span)),
            }
        }
    }

    fn read_number(&mut self) -> i64 {
        let mut n: i64 = 0;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                n = n * 10 + (ch as i64 - '0' as i64);
                self.advance();
            } else {
                break;
            }
        }
        n
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    pub fn tokenize(&mut self) -> error::Result<Vec<SpannedToken>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            let span = self.span();
            match self.peek() {
                None => {
                    tokens.push(SpannedToken { token: Token::Eof, span });
                    return Ok(tokens);
                }
                Some(ch) => {
                    let token = match ch {
                        '"' => {
                            let s = self.read_string()?;
                            Token::StringLit(s)
                        }
                        '(' => { self.advance(); Token::LParen }
                        ')' => { self.advance(); Token::RParen }
                        '{' => { self.advance(); Token::LBrace }
                        '}' => { self.advance(); Token::RBrace }
                        ':' => { self.advance(); Token::Colon }
                        ',' => { self.advance(); Token::Comma }
                        '.' => { self.advance(); Token::Dot }
                        '+' => { self.advance(); Token::Plus }
                        '*' => { self.advance(); Token::Star }
                        '/' => {
                            self.advance();
                            // Check for doc comment: ///
                            if self.peek() == Some('/') && self.peek2() == Some('/') {
                                self.advance(); // second /
                                self.advance(); // third /
                                // Skip optional leading space
                                if self.peek() == Some(' ') {
                                    self.advance();
                                }
                                let mut doc = String::new();
                                while let Some(c) = self.peek() {
                                    if c == '\n' { break; }
                                    doc.push(c);
                                    self.advance();
                                }
                                Token::DocComment(doc)
                            } else {
                                Token::Slash
                            }
                        }
                        '%' => { self.advance(); Token::Percent }
                        '-' => {
                            self.advance();
                            if self.peek() == Some('>') {
                                self.advance();
                                Token::Arrow
                            } else {
                                Token::Minus
                            }
                        }
                        '=' => {
                            self.advance();
                            if self.peek() == Some('=') {
                                self.advance();
                                Token::EqEq
                            } else if self.peek() == Some('>') {
                                self.advance();
                                Token::FatArrow
                            } else {
                                Token::Eq
                            }
                        }
                        '!' => {
                            self.advance();
                            if self.peek() == Some('=') {
                                self.advance();
                                Token::BangEq
                            } else {
                                return error::err("unexpected '!'", &self.file, Some(span));
                            }
                        }
                        '<' => {
                            self.advance();
                            if self.peek() == Some('=') {
                                self.advance();
                                Token::LtEq
                            } else if self.peek() == Some('>') {
                                self.advance();
                                Token::Concat
                            } else {
                                Token::Lt
                            }
                        }
                        '>' => {
                            self.advance();
                            if self.peek() == Some('=') {
                                self.advance();
                                Token::GtEq
                            } else {
                                Token::Gt
                            }
                        }
                        '|' => {
                            self.advance();
                            if self.peek() == Some('>') {
                                self.advance();
                                Token::PipeOp
                            } else {
                                Token::Pipe
                            }
                        }
                        _ if ch.is_ascii_digit() => {
                            let n = self.read_number();
                            Token::Int(n)
                        }
                        _ if ch.is_alphabetic() || ch == '_' => {
                            let ident = self.read_ident();
                            match ident.as_str() {
                                "fn" => Token::Fn,
                                "let" => Token::Let,
                                "if" => Token::If,
                                "else" => Token::Else,
                                "match" => Token::Match,
                                "type" => Token::Type,
                                "foreign" => Token::Foreign,
                                "True" => Token::True,
                                "False" => Token::False,
                                "receive" => Token::Receive,
                                "import" => Token::Import,
                                "pub" => Token::Pub,
                                "const" => Token::Const,
                                "trait" => Token::Trait,
                                "opaque" => Token::Opaque,
                                "use" => Token::Use,
                                _ => Token::Ident(ident),
                            }
                        }
                        _ => {
                            return error::err(
                                format!("unexpected character '{}'", ch),
                                &self.file,
                                Some(span),
                            );
                        }
                    };
                    tokens.push(SpannedToken { token, span });
                }
            }
        }
    }
}

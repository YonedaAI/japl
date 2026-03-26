//! japl-lexer: Tokenizer for the JAPL programming language.
//!
//! Uses the `logos` crate for DFA-based lexing, with a custom post-processing
//! layer that handles indentation tracking and emits synthetic Indent/Dedent tokens.

mod token;

pub use token::Token;

use japl_common::{Diagnostic, DiagnosticSink, FileId, Span};
use smol_str::SmolStr;
use std::collections::VecDeque;

/// A token with its source span and original text.
#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
    pub text: SmolStr,
}

/// The JAPL lexer. Wraps the logos DFA lexer and adds indentation tracking.
pub struct Lexer<'src> {
    source: &'src str,
    file_id: FileId,
    raw_tokens: Vec<(Token, std::ops::Range<usize>)>,
    pos: usize,
    indent_stack: Vec<u32>,
    pending: VecDeque<SpannedToken>,
    paren_depth: u32,
    diagnostics: DiagnosticSink,
    done: bool,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str, file_id: FileId) -> Self {
        use logos::Logos;

        // Collect all raw tokens from logos
        let mut raw_tokens = Vec::new();
        let mut lex = Token::lexer(source);
        while let Some(result) = lex.next() {
            let range = lex.span();
            match result {
                Ok(tok) => raw_tokens.push((tok, range)),
                Err(()) => raw_tokens.push((Token::Error, range)),
            }
        }

        Lexer {
            source,
            file_id,
            raw_tokens,
            pos: 0,
            indent_stack: vec![0],
            pending: VecDeque::new(),
            paren_depth: 0,
            diagnostics: DiagnosticSink::new(),
            done: false,
        }
    }

    fn make_span(&self, start: usize, end: usize) -> Span {
        Span::new(self.file_id, start as u32, end as u32)
    }

    fn text_at(&self, range: &std::ops::Range<usize>) -> SmolStr {
        SmolStr::new(&self.source[range.clone()])
    }

    /// Produce the next token, handling indentation and nesting.
    pub fn next_token(&mut self) -> Option<SpannedToken> {
        // Drain pending tokens first
        if let Some(tok) = self.pending.pop_front() {
            return Some(tok);
        }

        if self.done {
            return None;
        }

        if self.pos >= self.raw_tokens.len() {
            // Emit remaining dedents at EOF
            self.done = true;
            let eof_offset = self.source.len();
            while self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                self.pending.push_back(SpannedToken {
                    token: Token::Dedent,
                    span: self.make_span(eof_offset, eof_offset),
                    text: SmolStr::new(""),
                });
            }
            self.pending.push_back(SpannedToken {
                token: Token::Eof,
                span: self.make_span(eof_offset, eof_offset),
                text: SmolStr::new(""),
            });
            return self.pending.pop_front();
        }

        let (tok, range) = self.raw_tokens[self.pos].clone();
        self.pos += 1;

        match tok {
            Token::Newline => {
                // Inside parens/brackets/braces, newlines are insignificant
                if self.paren_depth > 0 {
                    return self.next_token();
                }

                // Look ahead to compute indentation of the next non-blank line
                let next_indent = self.measure_next_indent();

                let current_indent = *self.indent_stack.last().unwrap();

                // Emit the Newline token
                let newline_tok = SpannedToken {
                    token: Token::Newline,
                    span: self.make_span(range.start, range.end),
                    text: SmolStr::new("\n"),
                };

                if next_indent > current_indent {
                    self.indent_stack.push(next_indent);
                    self.pending.push_back(SpannedToken {
                        token: Token::Indent,
                        span: self.make_span(range.end, range.end),
                        text: SmolStr::new(""),
                    });
                } else if next_indent < current_indent {
                    while self.indent_stack.len() > 1
                        && *self.indent_stack.last().unwrap() > next_indent
                    {
                        self.indent_stack.pop();
                        self.pending.push_back(SpannedToken {
                            token: Token::Dedent,
                            span: self.make_span(range.end, range.end),
                            text: SmolStr::new(""),
                        });
                    }
                }

                Some(newline_tok)
            }

            Token::LParen | Token::LBracket | Token::LBrace => {
                self.paren_depth += 1;
                Some(SpannedToken {
                    token: tok,
                    span: self.make_span(range.start, range.end),
                    text: self.text_at(&range),
                })
            }

            Token::RParen | Token::RBracket | Token::RBrace => {
                if self.paren_depth > 0 {
                    self.paren_depth -= 1;
                }
                Some(SpannedToken {
                    token: tok,
                    span: self.make_span(range.start, range.end),
                    text: self.text_at(&range),
                })
            }

            Token::LineComment | Token::DocComment => {
                // Skip comments
                self.next_token()
            }

            Token::Error => {
                let text = self.text_at(&range);
                let span = self.make_span(range.start, range.end);
                self.diagnostics.emit(
                    Diagnostic::error(format!("unexpected character: '{}'", text))
                        .with_label(span, "here"),
                );
                self.next_token()
            }

            _ => Some(SpannedToken {
                token: tok,
                span: self.make_span(range.start, range.end),
                text: self.text_at(&range),
            }),
        }
    }

    /// Measure the indentation level of the next non-blank line.
    fn measure_next_indent(&mut self) -> u32 {
        // Skip over blank lines (consecutive Newlines)
        while self.pos < self.raw_tokens.len() {
            let (ref tok, _) = self.raw_tokens[self.pos];
            if *tok == Token::Newline {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.raw_tokens.len() {
            return 0;
        }

        // The indentation of the next token is its column (byte offset from start of its line)
        let (_, ref next_range) = self.raw_tokens[self.pos];
        let offset = next_range.start;

        // Walk backwards from offset to find the start of the line
        let line_start = self.source[..offset]
            .rfind('\n')
            .map(|p| p + 1)
            .unwrap_or(0);

        // Count spaces/tabs from line_start to offset
        let indent_str = &self.source[line_start..offset];
        let mut col = 0u32;
        for ch in indent_str.chars() {
            match ch {
                ' ' => col += 1,
                '\t' => col += 4, // tabs are 4 spaces
                _ => break,
            }
        }
        col
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics.into_diagnostics()
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

/// Lex an entire source file into a Vec<SpannedToken> plus diagnostics.
pub fn lex_all(source: &str, file_id: FileId) -> (Vec<SpannedToken>, Vec<Diagnostic>) {
    let mut lexer = Lexer::new(source, file_id);
    let mut tokens = Vec::new();
    while let Some(tok) = lexer.next_token() {
        tokens.push(tok);
    }
    let diagnostics = lexer.into_diagnostics();
    (tokens, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<Token> {
        let (tokens, _) = lex_all(src, FileId(0));
        tokens.into_iter().map(|t| t.token).collect()
    }

    #[test]
    fn test_integer_literals() {
        let tokens = lex("42 0xFF 0b1010 0o77");
        assert_eq!(
            tokens,
            vec![
                Token::IntLiteral,
                Token::HexIntLiteral,
                Token::BinIntLiteral,
                Token::OctIntLiteral,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_float_literal() {
        let tokens = lex("3.14 1.0e10");
        assert_eq!(
            tokens,
            vec![Token::FloatLiteral, Token::FloatLiteral, Token::Eof]
        );
    }

    #[test]
    fn test_string_literal() {
        let tokens = lex("\"hello world\"");
        assert_eq!(tokens, vec![Token::StringLiteral, Token::Eof]);
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("fn let match if else type module import");
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Let,
                Token::Match,
                Token::If,
                Token::Else,
                Token::Type,
                Token::Module,
                Token::Import,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_operators() {
        let tokens = lex("+ - * / == != |> >> -> =>");
        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::EqEq,
                Token::BangEq,
                Token::PipeRight,
                Token::ComposeRight,
                Token::Arrow,
                Token::FatArrow,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex("foo Bar _private Type123");
        assert_eq!(
            tokens,
            vec![
                Token::LowerIdent,
                Token::UpperIdent,
                Token::LowerIdent,
                Token::UpperIdent,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_booleans() {
        let tokens = lex("True False");
        assert_eq!(tokens, vec![Token::True, Token::False, Token::Eof]);
    }

    #[test]
    fn test_delimiters() {
        let tokens = lex("( ) [ ] { }");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::RParen,
                Token::LBracket,
                Token::RBracket,
                Token::LBrace,
                Token::RBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_simple_expression() {
        let tokens = lex("1 + 2 * 3");
        assert_eq!(
            tokens,
            vec![
                Token::IntLiteral,
                Token::Plus,
                Token::IntLiteral,
                Token::Star,
                Token::IntLiteral,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_function_decl_tokens() {
        let tokens = lex("fn add(x: Int, y: Int) -> Int = x + y");
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::LowerIdent,
                Token::LParen,
                Token::LowerIdent,
                Token::Colon,
                Token::UpperIdent,
                Token::Comma,
                Token::LowerIdent,
                Token::Colon,
                Token::UpperIdent,
                Token::RParen,
                Token::Arrow,
                Token::UpperIdent,
                Token::Eq,
                Token::LowerIdent,
                Token::Plus,
                Token::LowerIdent,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_comments_skipped() {
        let tokens = lex("x -- this is a comment\ny");
        // Should skip the comment, but x, newline, y remain
        assert!(tokens.contains(&Token::LowerIdent));
        assert!(tokens.contains(&Token::Eof));
    }

    #[test]
    fn test_indentation() {
        let src = "fn foo() -> Int =\n  let x = 1\n  x\n";
        let tokens = lex(src);
        assert!(tokens.contains(&Token::Indent));
        assert!(tokens.contains(&Token::Dedent));
    }

    #[test]
    fn test_pipe_operator() {
        let tokens = lex("x |> f |> g");
        assert_eq!(
            tokens,
            vec![
                Token::LowerIdent,
                Token::PipeRight,
                Token::LowerIdent,
                Token::PipeRight,
                Token::LowerIdent,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_nested_parens_suppress_indent() {
        // Newlines inside parens should not produce Indent/Dedent
        let src = "f(\n  x,\n  y\n)";
        let tokens = lex(src);
        assert!(!tokens.contains(&Token::Indent));
        assert!(!tokens.contains(&Token::Dedent));
    }
}

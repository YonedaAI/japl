use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    If,
    Else,
    Match,
    Type,
    Foreign,
    True,
    False,
    Receive,
    Import,
    Pub,
    Const,
    Trait,
    Opaque,
    Use,

    // Literals
    Int(i64),
    StringLit(String),
    Ident(String),

    // Symbols
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Dot,
    Arrow,      // ->
    FatArrow,   // =>
    Pipe,       // |
    PipeOp,     // |>
    Eq,         // =
    EqEq,       // ==
    BangEq,     // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Concat,     // <>

    // Special
    DocComment(String),

    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

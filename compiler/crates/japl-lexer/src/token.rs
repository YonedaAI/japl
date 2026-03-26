//! Token definitions for the JAPL lexer.

use logos::Logos;

/// Every token produced by the JAPL lexer.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t]+")] // skip horizontal whitespace
pub enum Token {
    // ── Keywords ────────────────────────────────────────────
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("match")]
    Match,
    #[token("with")]
    With,
    #[token("type")]
    Type,
    #[token("module")]
    Module,
    #[token("import")]
    Import,
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("else")]
    Else,
    #[token("use")]
    Use,
    #[token("own")]
    Own,
    #[token("ref")]
    Ref,
    #[token("opaque")]
    Opaque,
    #[token("trait")]
    Trait,
    #[token("impl")]
    Impl,
    #[token("deriving")]
    Deriving,
    #[token("test")]
    Test,
    #[token("property")]
    Property,
    #[token("forall")]
    Forall,
    #[token("assert")]
    Assert,
    #[token("where")]
    Where,
    #[token("signature")]
    Signature,
    #[token("foreign")]
    Foreign,
    #[token("unsafe")]
    Unsafe,
    #[token("loop")]
    Loop,
    #[token("while")]
    While,
    #[token("do")]
    Do,
    #[token("continue")]
    Continue,
    #[token("packed")]
    Packed,
    #[token("bench")]
    Bench,
    #[token("receive")]
    Receive,
    #[token("supervisor")]
    Supervisor,
    #[token("strategy")]
    Strategy,
    #[token("child")]
    Child,
    #[token("pub")]
    Pub,
    #[token("resource")]
    Resource,
    #[token("process")]
    Process,
    #[token("spawn")]
    Spawn,
    #[token("send")]
    Send,
    #[token("return")]
    Return,
    #[token("done")]
    Done,
    #[token("fail")]
    Fail,
    #[token("panic")]
    Panic,
    #[token("as")]
    As,
    #[token("in")]
    In,
    #[token("on")]
    On,
    #[token("alias")]
    Alias,

    // ── Literals ────────────────────────────────────────────
    #[regex(r"[0-9][0-9_]*", priority = 3)]
    IntLiteral,

    #[regex(r"0x[0-9a-fA-F][0-9a-fA-F_]*")]
    HexIntLiteral,

    #[regex(r"0b[01][01_]*")]
    BinIntLiteral,

    #[regex(r"0o[0-7][0-7_]*")]
    OctIntLiteral,

    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?", priority = 4)]
    FloatLiteral,

    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,

    #[regex(r"'[^'\\]'|'\\[nrt\\0']'|'\\u\{[0-9a-fA-F]+\}'")]
    CharLiteral,

    #[token("True")]
    True,
    #[token("False")]
    False,

    // ── Identifiers ─────────────────────────────────────────
    #[regex(r"[a-z_][a-zA-Z0-9_]*")]
    LowerIdent,

    #[regex(r"[A-Z][a-zA-Z0-9_]*", priority = 2)]
    UpperIdent,

    // ── Operators ───────────────────────────────────────────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("==")]
    EqEq,
    #[token("!=")]
    BangEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("!")]
    Bang,
    #[token("++")]
    PlusPlus,
    #[token("<>")]
    Diamond,
    #[token("|>")]
    PipeRight,
    #[token(">>")]
    ComposeRight,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("?")]
    Question,
    #[token("=")]
    Eq,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token("|")]
    Pipe,
    #[token(",")]
    Comma,
    #[token("..")]
    DotDot,
    #[token(".")]
    Dot,
    #[token("^")]
    Caret,

    // ── Delimiters ──────────────────────────────────────────
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    // ── Whitespace & Structure ──────────────────────────────
    #[regex(r"\n")]
    Newline,

    /// Synthetic token: indentation increased
    Indent,
    /// Synthetic token: indentation decreased
    Dedent,

    // ── Comments ────────────────────────────────────────────
    #[regex(r"---[^\n]*")]
    DocComment,

    #[regex(r"--[^\n]*")]
    LineComment,

    // ── Special ─────────────────────────────────────────────
    Eof,

    /// Lexer error token
    Error,
}

impl Token {
    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Fn
                | Token::Let
                | Token::Match
                | Token::With
                | Token::Type
                | Token::Module
                | Token::Import
                | Token::If
                | Token::Then
                | Token::Else
                | Token::Use
                | Token::Own
                | Token::Ref
                | Token::Opaque
                | Token::Trait
                | Token::Impl
                | Token::Deriving
                | Token::Test
                | Token::Property
                | Token::Forall
                | Token::Assert
                | Token::Where
                | Token::Signature
                | Token::Foreign
                | Token::Unsafe
                | Token::Loop
                | Token::While
                | Token::Do
                | Token::Continue
                | Token::Packed
                | Token::Bench
                | Token::Receive
                | Token::Supervisor
                | Token::Strategy
                | Token::Child
                | Token::Pub
                | Token::Resource
                | Token::Process
                | Token::Spawn
                | Token::Send
                | Token::Return
                | Token::Done
                | Token::Fail
                | Token::Panic
                | Token::As
                | Token::In
                | Token::On
                | Token::Alias
        )
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Match => write!(f, "match"),
            Token::With => write!(f, "with"),
            Token::Type => write!(f, "type"),
            Token::Module => write!(f, "module"),
            Token::Import => write!(f, "import"),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::Use => write!(f, "use"),
            Token::Own => write!(f, "own"),
            Token::Ref => write!(f, "ref"),
            Token::Opaque => write!(f, "opaque"),
            Token::Trait => write!(f, "trait"),
            Token::Impl => write!(f, "impl"),
            Token::Test => write!(f, "test"),
            Token::Where => write!(f, "where"),
            Token::Foreign => write!(f, "foreign"),
            Token::Receive => write!(f, "receive"),
            Token::Supervisor => write!(f, "supervisor"),
            Token::Spawn => write!(f, "spawn"),
            Token::Send => write!(f, "send"),
            Token::Return => write!(f, "return"),
            Token::IntLiteral => write!(f, "<int>"),
            Token::HexIntLiteral => write!(f, "<hex_int>"),
            Token::BinIntLiteral => write!(f, "<bin_int>"),
            Token::OctIntLiteral => write!(f, "<oct_int>"),
            Token::FloatLiteral => write!(f, "<float>"),
            Token::StringLiteral => write!(f, "<string>"),
            Token::CharLiteral => write!(f, "<char>"),
            Token::True => write!(f, "True"),
            Token::False => write!(f, "False"),
            Token::LowerIdent => write!(f, "<ident>"),
            Token::UpperIdent => write!(f, "<type_ident>"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::EqEq => write!(f, "=="),
            Token::BangEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::AmpAmp => write!(f, "&&"),
            Token::PipePipe => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::PlusPlus => write!(f, "++"),
            Token::Diamond => write!(f, "<>"),
            Token::PipeRight => write!(f, "|>"),
            Token::ComposeRight => write!(f, ">>"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::Question => write!(f, "?"),
            Token::Eq => write!(f, "="),
            Token::Colon => write!(f, ":"),
            Token::ColonColon => write!(f, "::"),
            Token::Pipe => write!(f, "|"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::DotDot => write!(f, ".."),
            Token::Caret => write!(f, "^"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Newline => write!(f, "<newline>"),
            Token::Indent => write!(f, "<indent>"),
            Token::Dedent => write!(f, "<dedent>"),
            Token::LineComment => write!(f, "<comment>"),
            Token::DocComment => write!(f, "<doc_comment>"),
            Token::Eof => write!(f, "<eof>"),
            Token::Error => write!(f, "<error>"),
            _ => write!(f, "<token>"),
        }
    }
}

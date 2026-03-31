use std::fmt;

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub file: String,
    pub span: Option<Span>,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref span) = self.span {
            write!(f, "{}:{}:{}: {}", self.file, span.line, span.col, self.message)
        } else {
            write!(f, "{}: {}", self.file, self.message)
        }
    }
}

impl std::error::Error for CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>;

pub fn err<T>(msg: impl Into<String>, file: &str, span: Option<Span>) -> Result<T> {
    Err(CompileError {
        message: msg.into(),
        file: file.to_string(),
        span,
    })
}

use std::fmt;

use crate::source::{SourceFile, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn display(&self, source: &SourceFile) -> String {
        let mut out = String::new();
        let line = self.span.start.line;
        let col = self.span.start.column;

        out.push_str(&format!(
            "{}:{}:{}: {}: {}\n",
            source.path.display(),
            line,
            col,
            self.severity,
            self.message
        ));

        if line >= 1 && line <= source.lines.len() {
            let source_line = &source.lines[line - 1];
            out.push_str(&format!("  {source_line}\n"));
            let end_col = if self.span.end.line == line {
                self.span.end.column
            } else {
                source_line.len()
            };
            let underline_len = (end_col - col).max(1);
            out.push_str(&format!("  {}{}\n", " ".repeat(col), "^".repeat(underline_len)));
        }

        out
    }
}

#[derive(Debug)]
pub struct CompileError {
    pub diagnostic: Diagnostic,
}

impl CompileError {
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self { diagnostic }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.diagnostic.severity, self.diagnostic.message)
    }
}

impl std::error::Error for CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>;

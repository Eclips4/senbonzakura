use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,   // 1-based
    pub column: usize, // 0-based
    pub offset: usize, // byte offset
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        let pos = Position {
            line: 0,
            column: 0,
            offset: 0,
        };
        Self {
            start: pos,
            end: pos,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start.line, self.start.column)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub lines: Vec<String>,
}

impl SourceFile {
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines = content.lines().map(String::from).collect();
        Ok(Self {
            path: path.to_path_buf(),
            content,
            lines,
        })
    }

    pub fn from_string(content: &str) -> Self {
        let lines = content.lines().map(String::from).collect();
        Self {
            path: PathBuf::from("<string>"),
            content: content.to_string(),
            lines,
        }
    }
}

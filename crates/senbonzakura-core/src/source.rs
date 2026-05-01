use std::fmt;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use encoding_rs::{Encoding};

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
        let file_bytes = std::fs::read(path)?;
        let content = SourceFile::read_content_from_bytes(file_bytes)?;

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

    fn read_content_from_bytes(bytes: Vec<u8>) -> std::io::Result<String> {
        SourceFile::validate_no_bom(&bytes)?;

        let content = String::from_utf8(bytes)
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Source file contains invalid UTF8"))?;

        Ok(content)
    }

    fn validate_no_bom(bytes: &Vec<u8>) -> std::io::Result<()> {
        if Encoding::for_bom(bytes).is_some() {
            return Err(Error::new(ErrorKind::InvalidInput, "Source file has BOM"));
        }

        Ok(())
    }
}

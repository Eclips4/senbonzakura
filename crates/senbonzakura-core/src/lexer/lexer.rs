use crate::errors::{CompileError, Diagnostic};
use crate::lexer::token::{lookup_keyword, Token, TokenKind};
use crate::source::{Position, SourceFile, Span};

pub struct Lexer<'a> {
    source: &'a SourceFile,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    indent_stack: Vec<usize>,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            chars: source.content.chars().collect(),
            pos: 0,
            line: 1,
            column: 0,
            tokens: Vec::new(),
            indent_stack: vec![0],
            at_line_start: true,
        }
    }

    pub fn tokenize(mut self) -> crate::errors::Result<Vec<Token>> {
        while !self.is_at_end() {
            if self.at_line_start {
                self.handle_indentation()?;
                self.at_line_start = false;
            }

            if self.is_at_end() {
                break;
            }

            let ch = self.peek();
            match ch {
                ' ' | '\t' => {
                    self.advance();
                }
                '\n' | '\r' => {
                    self.emit_newline();
                    self.skip_newline();
                    self.at_line_start = true;
                }
                '#' => {
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '"' => self.lex_string()?,
                _ if ch.is_ascii_digit() => self.lex_number()?,
                _ if ch.is_alphabetic() || ch == '_' => self.lex_identifier(),
                _ => self.lex_punctuation()?,
            }
        }

        let eof_pos = self.current_position();
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens.push(Token {
                kind: TokenKind::Dedent,
                text: String::new(),
                span: Span::new(eof_pos, eof_pos),
            });
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            text: String::new(),
            span: Span::new(eof_pos, eof_pos),
        });

        Ok(self.tokens)
    }

    fn handle_indentation(&mut self) -> crate::errors::Result<()> {
        loop {
            let saved_pos = self.pos;
            let saved_line = self.line;
            let saved_col = self.column;

            let mut indent = 0;
            while !self.is_at_end() && self.peek() == ' ' {
                self.advance();
                indent += 1;
            }
            while !self.is_at_end() && self.peek() == '\t' {
                self.advance();
                indent = (indent / 4 + 1) * 4;
            }

            if self.is_at_end() {
                return Ok(());
            }

            match self.peek() {
                '\n' | '\r' => {
                    self.skip_newline();
                    continue;
                }
                '#' => {
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                    if !self.is_at_end() {
                        self.skip_newline();
                    }
                    continue;
                }
                _ => {}
            }

            let pos = Position {
                line: saved_line,
                column: saved_col,
                offset: saved_pos,
            };
            let span = Span::new(pos, self.current_position());
            let current_indent = *self.indent_stack.last().unwrap();

            if indent > current_indent {
                self.indent_stack.push(indent);
                self.tokens.push(Token {
                    kind: TokenKind::Indent,
                    text: String::new(),
                    span,
                });
            } else if indent < current_indent {
                while *self.indent_stack.last().unwrap() > indent {
                    self.indent_stack.pop();
                    self.tokens.push(Token {
                        kind: TokenKind::Dedent,
                        text: String::new(),
                        span,
                    });
                }
                if *self.indent_stack.last().unwrap() != indent {
                    return Err(CompileError::new(Diagnostic::error(
                        "inconsistent indentation",
                        span,
                    )));
                }
            }

            break;
        }
        Ok(())
    }

    fn lex_number(&mut self) -> crate::errors::Result<()> {
        let start = self.current_position();
        let mut text = String::new();
        let mut is_float = false;

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            text.push(self.peek());
            self.advance();
        }

        if !self.is_at_end() && self.peek() == '.' {
            if self.pos + 1 < self.chars.len() && self.chars[self.pos + 1].is_ascii_digit() {
                is_float = true;
                text.push(self.peek());
                self.advance();
                while !self.is_at_end() && self.peek().is_ascii_digit() {
                    text.push(self.peek());
                    self.advance();
                }
            }
        }

        let end = self.current_position();
        self.tokens.push(Token {
            kind: if is_float {
                TokenKind::FloatLit
            } else {
                TokenKind::IntLit
            },
            text,
            span: Span::new(start, end),
        });
        Ok(())
    }

    fn lex_string(&mut self) -> crate::errors::Result<()> {
        let start = self.current_position();
        self.advance();
        let mut text = String::new();

        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                return Err(CompileError::new(Diagnostic::error(
                    "unterminated string literal",
                    Span::new(start, self.current_position()),
                )));
            }
            if self.peek() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(CompileError::new(Diagnostic::error(
                        "unterminated string literal",
                        Span::new(start, self.current_position()),
                    )));
                }
                match self.peek() {
                    'n' => text.push('\n'),
                    't' => text.push('\t'),
                    '\\' => text.push('\\'),
                    '"' => text.push('"'),
                    c => {
                        return Err(CompileError::new(Diagnostic::error(
                            format!("unknown escape sequence: \\{c}"),
                            Span::new(start, self.current_position()),
                        )));
                    }
                }
                self.advance();
            } else {
                text.push(self.peek());
                self.advance();
            }
        }

        if self.is_at_end() {
            return Err(CompileError::new(Diagnostic::error(
                "unterminated string literal",
                Span::new(start, self.current_position()),
            )));
        }

        self.advance();
        let end = self.current_position();
        self.tokens.push(Token {
            kind: TokenKind::StringLit,
            text,
            span: Span::new(start, end),
        });
        Ok(())
    }

    fn lex_identifier(&mut self) {
        let start = self.current_position();
        let mut text = String::new();

        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            text.push(self.peek());
            self.advance();
        }

        let end = self.current_position();
        let kind = lookup_keyword(&text).unwrap_or(TokenKind::Ident);
        self.tokens.push(Token {
            kind,
            text,
            span: Span::new(start, end),
        });
    }

    fn lex_punctuation(&mut self) -> crate::errors::Result<()> {
        let start = self.current_position();
        let ch = self.peek();
        self.advance();

        let (kind, text) = match ch {
            '+' => (TokenKind::Plus, "+"),
            '*' => (TokenKind::Star, "*"),
            '/' => (TokenKind::Slash, "/"),
            '(' => (TokenKind::LParen, "("),
            ')' => (TokenKind::RParen, ")"),
            '[' => (TokenKind::LBracket, "["),
            ']' => (TokenKind::RBracket, "]"),
            ':' => (TokenKind::Colon, ":"),
            ',' => (TokenKind::Comma, ","),
            '.' => (TokenKind::Dot, "."),
            '-' => {
                if !self.is_at_end() && self.peek() == '>' {
                    self.advance();
                    (TokenKind::Arrow, "->")
                } else {
                    (TokenKind::Minus, "-")
                }
            }
            '=' => {
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    (TokenKind::EqEq, "==")
                } else {
                    (TokenKind::Eq, "=")
                }
            }
            '!' => {
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    (TokenKind::BangEq, "!=")
                } else {
                    return Err(CompileError::new(Diagnostic::error(
                        format!("unexpected character: '{ch}'"),
                        Span::new(start, self.current_position()),
                    )));
                }
            }
            '<' => {
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    (TokenKind::LtEq, "<=")
                } else {
                    (TokenKind::Lt, "<")
                }
            }
            '>' => {
                if !self.is_at_end() && self.peek() == '=' {
                    self.advance();
                    (TokenKind::GtEq, ">=")
                } else {
                    (TokenKind::Gt, ">")
                }
            }
            _ => {
                return Err(CompileError::new(Diagnostic::error(
                    format!("unexpected character: '{ch}'"),
                    Span::new(start, self.current_position()),
                )));
            }
        };

        let end = self.current_position();
        self.tokens.push(Token {
            kind,
            text: text.to_string(),
            span: Span::new(start, end),
        });
        Ok(())
    }

    fn emit_newline(&mut self) {
        if let Some(last) = self.tokens.last() {
            if last.kind == TokenKind::Newline {
                return;
            }
        }
        let pos = self.current_position();
        self.tokens.push(Token {
            kind: TokenKind::Newline,
            text: String::new(),
            span: Span::new(pos, pos),
        });
    }

    fn skip_newline(&mut self) {
        if !self.is_at_end() && self.peek() == '\r' {
            self.advance();
        }
        if !self.is_at_end() && self.peek() == '\n' {
            self.advance();
        }
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn advance(&mut self) {
        if self.pos < self.chars.len() {
            if self.chars[self.pos] == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn current_position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
            offset: self.pos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<Token> {
        let source = SourceFile::from_string(input);
        Lexer::new(&source).tokenize().unwrap()
    }

    fn kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_integer_literal() {
        assert_eq!(kinds("42"), vec![TokenKind::IntLit, TokenKind::Eof]);
    }

    #[test]
    fn test_float_literal() {
        assert_eq!(kinds("3.14"), vec![TokenKind::FloatLit, TokenKind::Eof]);
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize("\"hello\"");
        assert_eq!(tokens[0].kind, TokenKind::StringLit);
        assert_eq!(tokens[0].text, "hello");
    }

    #[test]
    fn test_string_escape() {
        let tokens = tokenize("\"hello\\nworld\"");
        assert_eq!(tokens[0].text, "hello\nworld");
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            kinds("def let data return"),
            vec![
                TokenKind::KwDef,
                TokenKind::KwLet,
                TokenKind::KwData,
                TokenKind::KwReturn,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            kinds("+ - * / == != -> <= >="),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Arrow,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_let_binding() {
        assert_eq!(
            kinds("let x: Int = 42"),
            vec![
                TokenKind::KwLet,
                TokenKind::Ident,
                TokenKind::Colon,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::IntLit,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_indentation() {
        let input = "def foo():\n    return 42\n";
        assert_eq!(
            kinds(input),
            vec![
                TokenKind::KwDef,
                TokenKind::Ident,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::KwReturn,
                TokenKind::IntLit,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_nested_indentation() {
        let input = "if True:\n    if False:\n        42\n";
        assert_eq!(
            kinds(input),
            vec![
                TokenKind::KwIf,
                TokenKind::KwTrue,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::KwIf,
                TokenKind::KwFalse,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::IntLit,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_comment_ignored() {
        assert_eq!(
            kinds("42 # this is a comment"),
            vec![TokenKind::IntLit, TokenKind::Eof]
        );
    }

    #[test]
    fn test_blank_lines_ignored() {
        assert_eq!(
            kinds("42\n\n\n43"),
            vec![
                TokenKind::IntLit,
                TokenKind::Newline,
                TokenKind::IntLit,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_while() {
        assert_eq!(
            kinds("while True:\n    42\n"),
            vec![
                TokenKind::KwWhile,
                TokenKind::KwTrue,
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Indent,
                TokenKind::IntLit,
                TokenKind::Newline,
                TokenKind::Dedent,
                TokenKind::Eof,
            ]
        );
    }
}

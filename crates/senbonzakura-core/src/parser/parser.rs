use crate::errors::{CompileError, Diagnostic};
use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;
use crate::source::{SourceFile, Span};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'a SourceFile,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, source: &'a SourceFile) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
    }

    pub fn parse_module(&mut self) -> crate::errors::Result<Module> {
        let start = self.current_span();
        let mut body = Vec::new();

        self.skip_newlines();
        while !self.check(TokenKind::Eof) {
            body.push(self.parse_statement()?);
            self.skip_newlines();
        }

        let end = self.current_span();
        Ok(Module {
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_statement(&mut self) -> crate::errors::Result<Statement> {
        match self.peek() {
            TokenKind::KwLet | TokenKind::KwMut => self.parse_let_binding(),
            TokenKind::KwDef => self.parse_func_def().map(Statement::FuncDef),
            TokenKind::KwData => self.parse_data_decl().map(Statement::DataDecl),
            TokenKind::KwIf => self.parse_if_stmt().map(Statement::If),
            TokenKind::KwReturn => self.parse_return_stmt().map(Statement::Return),
            TokenKind::KwTypeclass => self.parse_typeclass_decl().map(Statement::TypeclassDecl),
            TokenKind::KwImpl => self.parse_impl_block().map(Statement::Impl),
            TokenKind::KwFor => self.parse_for_stmt().map(Statement::For),
            TokenKind::KwWhile => self.parse_while_stmt().map(Statement::While),
            TokenKind::KwImport => self.parse_import_stmt().map(Statement::Import),
            TokenKind::KwFrom => self.parse_from_import_stmt().map(Statement::Import),
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_let_binding(&mut self) -> crate::errors::Result<Statement> {
        let start = self.current_span();
        let mutable = self.check(TokenKind::KwMut);
        self.advance();

        let name = self.expect_ident()?;

        let type_annotation = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;
        let end = value.span();
        self.expect_newline_or_eof()?;

        Ok(Statement::Let(LetBinding {
            name,
            type_annotation,
            value,
            mutable,
            span: Span::new(start.start, end.end),
        }))
    }

    fn parse_func_def(&mut self) -> crate::errors::Result<FuncDef> {
        let start = self.current_span();
        self.advance();

        let name = self.expect_ident()?;
        let mut type_params = Vec::new();
        if self.match_token(TokenKind::LBracket) {
            while !self.check(TokenKind::RBracket) {
                if !type_params.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                let tp_start = self.current_span();
                let tp_name = self.expect_ident()?;
                let mut constraints = Vec::new();
                if self.match_token(TokenKind::Colon) {
                    constraints.push(self.expect_ident()?);
                    while self.match_token(TokenKind::Plus) {
                        constraints.push(self.expect_ident()?);
                    }
                }
                let tp_end = self.current_span();
                type_params.push(TypeParamDef {
                    name: tp_name,
                    constraints,
                    span: Span::new(tp_start.start, tp_end.end),
                });
            }
            self.expect(TokenKind::RBracket)?;
        }

        self.expect(TokenKind::LParen)?;

        let mut params = Vec::new();
        while !self.check(TokenKind::RParen) {
            if !params.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            let param_start = self.current_span();
            let param_name = self.expect_ident()?;
            let type_annotation = if self.match_token(TokenKind::Colon) {
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            let param_end = self.current_span();
            params.push(Param {
                name: param_name,
                type_annotation,
                span: Span::new(param_start.start, param_end.end),
            });
        }
        self.expect(TokenKind::RParen)?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect(TokenKind::Colon)?;
        let body = self.parse_block()?;
        let end = self.current_span();

        Ok(FuncDef {
            name,
            type_params,
            params,
            return_type,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_data_decl(&mut self) -> crate::errors::Result<DataDecl> {
        let start = self.current_span();
        self.advance();

        let name = self.expect_ident()?;

        let mut type_params = Vec::new();
        if self.match_token(TokenKind::LBracket) {
            while !self.check(TokenKind::RBracket) {
                if !type_params.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                type_params.push(self.expect_ident()?);
            }
            self.expect(TokenKind::RBracket)?;
        }

        self.expect(TokenKind::Colon)?;
        self.expect_newline()?;
        self.expect(TokenKind::Indent)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.check(TokenKind::Dedent) && !self.check(TokenKind::Eof) {
            self.skip_newlines();
            if self.check(TokenKind::Dedent) || self.check(TokenKind::Eof) {
                break;
            }
            if self.check(TokenKind::KwDef) {
                methods.push(self.parse_func_def()?);
            } else {
                fields.push(self.parse_field_decl()?);
            }
            self.skip_newlines();
        }

        if self.check(TokenKind::Dedent) {
            self.advance();
        }
        let end = self.current_span();

        Ok(DataDecl {
            name,
            type_params,
            fields,
            methods,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_import_stmt(&mut self) -> crate::errors::Result<ImportStmt> {
        let start = self.current_span();
        self.advance();

        let mut module_path = vec![self.expect_ident()?];
        while self.match_token(TokenKind::Dot) {
            module_path.push(self.expect_ident()?);
        }

        let alias = if self.match_token(TokenKind::KwAs) {
            Some(self.expect_ident()?)
        } else {
            None
        };

        let end = self.current_span();
        self.expect_newline_or_eof()?;

        Ok(ImportStmt {
            kind: ImportKind::Simple { module_path, alias },
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_from_import_stmt(&mut self) -> crate::errors::Result<ImportStmt> {
        let start = self.current_span();
        self.advance();

        let mut module_path = vec![self.expect_ident()?];
        while self.match_token(TokenKind::Dot) {
            module_path.push(self.expect_ident()?);
        }

        self.expect(TokenKind::KwImport)?;

        let mut names = Vec::new();
        loop {
            let name = self.expect_ident()?;
            let alias = if self.match_token(TokenKind::KwAs) {
                Some(self.expect_ident()?)
            } else {
                None
            };
            names.push(ImportName { name, alias });
            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        let end = self.current_span();
        self.expect_newline_or_eof()?;

        Ok(ImportStmt {
            kind: ImportKind::From { module_path, names },
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_typeclass_decl(&mut self) -> crate::errors::Result<TypeclassDecl> {
        let start = self.current_span();
        self.advance();

        let name = self.expect_ident()?;
        self.expect(TokenKind::LBracket)?;
        let mut type_params = Vec::new();
        while !self.check(TokenKind::RBracket) {
            if !type_params.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            type_params.push(self.expect_ident()?);
        }
        self.expect(TokenKind::RBracket)?;
        self.expect(TokenKind::Colon)?;
        self.expect_newline()?;
        self.expect(TokenKind::Indent)?;

        let mut methods = Vec::new();
        self.skip_newlines();
        while !self.check(TokenKind::Dedent) && !self.check(TokenKind::Eof) {
            let sig_start = self.current_span();
            self.expect(TokenKind::KwDef)?;
            let method_name = self.expect_ident()?;
            self.expect(TokenKind::LParen)?;

            let mut params = Vec::new();
            while !self.check(TokenKind::RParen) {
                if !params.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                let param_start = self.current_span();
                let param_name = self.expect_ident()?;
                let type_annotation = if self.match_token(TokenKind::Colon) {
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                let param_end = self.current_span();
                params.push(Param {
                    name: param_name,
                    type_annotation,
                    span: Span::new(param_start.start, param_end.end),
                });
            }
            self.expect(TokenKind::RParen)?;
            self.expect(TokenKind::Arrow)?;
            let return_type = self.parse_type_expr()?;
            let sig_end = return_type.span();
            self.expect_newline_or_eof()?;

            methods.push(MethodSig {
                name: method_name,
                params,
                return_type,
                span: Span::new(sig_start.start, sig_end.end),
            });
            self.skip_newlines();
        }

        if self.check(TokenKind::Dedent) {
            self.advance();
        }
        let end = self.current_span();

        Ok(TypeclassDecl {
            name,
            type_params,
            methods,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_impl_block(&mut self) -> crate::errors::Result<ImplBlock> {
        let start = self.current_span();
        self.advance();

        let name = self.expect_ident()?;

        let kind = if self.check(TokenKind::LBracket) {
            self.expect(TokenKind::LBracket)?;
            let mut type_args = Vec::new();
            while !self.check(TokenKind::RBracket) {
                if !type_args.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                type_args.push(self.parse_type_expr()?);
            }
            self.expect(TokenKind::RBracket)?;
            ImplKind::Instance {
                typeclass_name: name,
                type_args,
            }
        } else {
            ImplKind::Inherent { target_type: name }
        };

        self.expect(TokenKind::Colon)?;
        self.expect_newline()?;
        self.expect(TokenKind::Indent)?;

        let mut methods = Vec::new();
        self.skip_newlines();
        while !self.check(TokenKind::Dedent) && !self.check(TokenKind::Eof) {
            methods.push(self.parse_func_def()?);
            self.skip_newlines();
        }

        if self.check(TokenKind::Dedent) {
            self.advance();
        }
        let end = self.current_span();

        Ok(ImplBlock {
            kind,
            methods,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_field_decl(&mut self) -> crate::errors::Result<FieldDecl> {
        let start = self.current_span();
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon)?;
        let type_annotation = self.parse_type_expr()?;
        let end = type_annotation.span();
        self.expect_newline_or_eof()?;

        Ok(FieldDecl {
            name,
            type_annotation,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_if_stmt(&mut self) -> crate::errors::Result<IfStmt> {
        let start = self.current_span();
        self.advance();

        let condition = self.parse_expr()?;
        self.expect(TokenKind::Colon)?;
        let then_body = self.parse_block()?;

        let mut elif_clauses = Vec::new();
        let mut else_body = None;

        while self.check(TokenKind::KwElif) {
            self.advance();
            let elif_cond = self.parse_expr()?;
            self.expect(TokenKind::Colon)?;
            let elif_body = self.parse_block()?;
            elif_clauses.push((elif_cond, elif_body));
        }

        if self.match_token(TokenKind::KwElse) {
            self.expect(TokenKind::Colon)?;
            else_body = Some(self.parse_block()?);
        }

        let end = self.current_span();
        Ok(IfStmt {
            condition,
            then_body,
            elif_clauses,
            else_body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_for_stmt(&mut self) -> crate::errors::Result<ForStmt> {
        let start = self.current_span();
        self.advance();

        let var_name = self.expect_ident()?;
        self.expect(TokenKind::KwIn)?;
        let iterable = self.parse_expr()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_block()?;
        let end = self.current_span();

        Ok(ForStmt {
            var_name,
            iterable,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_while_stmt(&mut self) -> crate::errors::Result<WhileStmt> {
        let start = self.current_span();
        self.advance();

        let condition = self.parse_expr()?;
        self.expect(TokenKind::Colon)?;
        let body = self.parse_block()?;
        let end = self.current_span();

        Ok(WhileStmt {
            condition,
            body,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_return_stmt(&mut self) -> crate::errors::Result<ReturnStmt> {
        let start = self.current_span();
        self.advance();

        let value = if !self.check(TokenKind::Newline) && !self.check(TokenKind::Eof) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = self.current_span();
        self.expect_newline_or_eof()?;

        Ok(ReturnStmt {
            value,
            span: Span::new(start.start, end.end),
        })
    }

    fn parse_expr_or_assign(&mut self) -> crate::errors::Result<Statement> {
        let expr = self.parse_expr()?;

        if self.match_token(TokenKind::Eq) {
            let value = self.parse_expr()?;
            let span = Span::new(expr.span().start, value.span().end);
            self.expect_newline_or_eof()?;
            Ok(Statement::Assign(Assignment {
                target: expr,
                value,
                span,
            }))
        } else {
            let span = expr.span();
            self.expect_newline_or_eof()?;
            Ok(Statement::Expr(ExprStmt { expr, span }))
        }
    }

    fn parse_block(&mut self) -> crate::errors::Result<Vec<Statement>> {
        self.expect_newline()?;
        self.expect(TokenKind::Indent)?;

        let mut stmts = Vec::new();
        self.skip_newlines();

        while !self.check(TokenKind::Dedent) && !self.check(TokenKind::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }

        if self.check(TokenKind::Dedent) {
            self.advance();
        }
        Ok(stmts)
    }

    fn parse_expr(&mut self) -> crate::errors::Result<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> crate::errors::Result<Expr> {
        let mut left = self.parse_and_expr()?;
        while self.check(TokenKind::KwOr) {
            self.advance();
            let right = self.parse_and_expr()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expr::BinOp(Box::new(BinOpExpr {
                left,
                op: BinOp::Or,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> crate::errors::Result<Expr> {
        let mut left = self.parse_not_expr()?;
        while self.check(TokenKind::KwAnd) {
            self.advance();
            let right = self.parse_not_expr()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expr::BinOp(Box::new(BinOpExpr {
                left,
                op: BinOp::And,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> crate::errors::Result<Expr> {
        if self.check(TokenKind::KwNot) {
            let start = self.current_span();
            self.advance();
            let operand = self.parse_not_expr()?;
            let span = Span::new(start.start, operand.span().end);
            Ok(Expr::UnaryOp(Box::new(UnaryOpExpr {
                op: UnaryOp::Not,
                operand,
                span,
            })))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> crate::errors::Result<Expr> {
        let mut left = self.parse_addition()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::NotEq,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_addition()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expr::BinOp(Box::new(BinOpExpr {
                left,
                op,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_addition(&mut self) -> crate::errors::Result<Expr> {
        let mut left = self.parse_multiplication()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expr::BinOp(Box::new(BinOpExpr {
                left,
                op,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_multiplication(&mut self) -> crate::errors::Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expr::BinOp(Box::new(BinOpExpr {
                left,
                op,
                right,
                span,
            }));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> crate::errors::Result<Expr> {
        if self.check(TokenKind::Minus) {
            let start = self.current_span();
            self.advance();
            let operand = self.parse_unary()?;
            let span = Span::new(start.start, operand.span().end);
            Ok(Expr::UnaryOp(Box::new(UnaryOpExpr {
                op: UnaryOp::Neg,
                operand,
                span,
            })))
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> crate::errors::Result<Expr> {
        let mut expr = self.parse_atom()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        if !args.is_empty() {
                            self.expect(TokenKind::Comma)?;
                        }
                        args.push(self.parse_expr()?);
                    }
                    let end = self.current_span();
                    self.expect(TokenKind::RParen)?;
                    let span = Span::new(expr.span().start, end.end);
                    expr = Expr::Call(Box::new(CallExpr {
                        callee: expr,
                        args,
                        span,
                    }));
                }
                TokenKind::Dot => {
                    self.advance();
                    let attr = self.expect_ident()?;
                    let end = self.current_span();
                    let span = Span::new(expr.span().start, end.end);
                    expr = Expr::Attr(Box::new(AttrExpr {
                        object: expr,
                        attr,
                        span,
                    }));
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    let end = self.current_span();
                    self.expect(TokenKind::RBracket)?;
                    let span = Span::new(expr.span().start, end.end);
                    expr = Expr::Index(Box::new(IndexExpr {
                        object: expr,
                        index,
                        span,
                    }));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> crate::errors::Result<Expr> {
        let span = self.current_span();
        match self.peek() {
            TokenKind::IntLit => {
                let text = self.current_text().to_string();
                self.advance();
                let value: i64 = text.parse().map_err(|_| {
                    CompileError::new(Diagnostic::error("invalid integer literal", span))
                })?;
                Ok(Expr::IntLiteral(value, span))
            }
            TokenKind::FloatLit => {
                let text = self.current_text().to_string();
                self.advance();
                let value: f64 = text.parse().map_err(|_| {
                    CompileError::new(Diagnostic::error("invalid float literal", span))
                })?;
                Ok(Expr::FloatLiteral(value, span))
            }
            TokenKind::StringLit => {
                let text = self.current_text().to_string();
                self.advance();
                Ok(Expr::StringLiteral(text, span))
            }
            TokenKind::KwTrue => {
                self.advance();
                Ok(Expr::BoolLiteral(true, span))
            }
            TokenKind::KwFalse => {
                self.advance();
                Ok(Expr::BoolLiteral(false, span))
            }
            TokenKind::KwNone => {
                self.advance();
                Ok(Expr::NoneLiteral(span))
            }
            TokenKind::Ident => {
                let name = self.current_text().to_string();
                self.advance();
                Ok(Expr::Name(name, span))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !self.check(TokenKind::RBracket) {
                    if !elements.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    elements.push(self.parse_expr()?);
                }
                let end = self.current_span();
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::ListLiteral(elements, Span::new(span.start, end.end)))
            }
            _ => Err(CompileError::new(Diagnostic::error(
                format!("expected expression, found {}", self.peek()),
                span,
            ))),
        }
    }

    fn parse_type_expr(&mut self) -> crate::errors::Result<TypeExpr> {
        let span = self.current_span();
        let name = self.expect_ident()?;

        if self.match_token(TokenKind::LBracket) {
            let mut args = Vec::new();
            while !self.check(TokenKind::RBracket) {
                if !args.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                args.push(self.parse_type_expr()?);
            }
            let end = self.current_span();
            self.expect(TokenKind::RBracket)?;
            Ok(TypeExpr::Parameterized(
                name,
                args,
                Span::new(span.start, end.end),
            ))
        } else {
            Ok(TypeExpr::Named(name, span))
        }
    }

    fn peek(&self) -> TokenKind {
        self.tokens.get(self.pos).map_or(TokenKind::Eof, |t| t.kind)
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek() == kind
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> crate::errors::Result<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(CompileError::new(Diagnostic::error(
                format!("expected {kind}, found {}", self.peek()),
                self.current_span(),
            )))
        }
    }

    fn expect_ident(&mut self) -> crate::errors::Result<String> {
        if self.check(TokenKind::Ident) {
            let name = self.current_text().to_string();
            self.advance();
            Ok(name)
        } else {
            Err(CompileError::new(Diagnostic::error(
                format!("expected identifier, found {}", self.peek()),
                self.current_span(),
            )))
        }
    }

    fn expect_newline(&mut self) -> crate::errors::Result<()> {
        if self.check(TokenKind::Newline) {
            self.advance();
            self.skip_newlines();
            Ok(())
        } else {
            Err(CompileError::new(Diagnostic::error(
                format!("expected newline, found {}", self.peek()),
                self.current_span(),
            )))
        }
    }

    fn expect_newline_or_eof(&mut self) -> crate::errors::Result<()> {
        if self.check(TokenKind::Newline) || self.check(TokenKind::Eof) || self.check(TokenKind::Dedent) {
            if self.check(TokenKind::Newline) {
                self.advance();
            }
            Ok(())
        } else {
            Err(CompileError::new(Diagnostic::error(
                format!("expected newline, found {}", self.peek()),
                self.current_span(),
            )))
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(TokenKind::Newline) {
            self.advance();
        }
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map_or(Span::dummy(), |t| t.span)
    }

    fn current_text(&self) -> &str {
        self.tokens
            .get(self.pos)
            .map_or("", |t| t.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer::Lexer;

    fn parse(input: &str) -> Module {
        let source = SourceFile::from_string(input);
        let tokens = Lexer::new(&source).tokenize().unwrap();
        Parser::new(tokens, &source).parse_module().unwrap()
    }

    #[test]
    fn test_let_binding() {
        let module = parse("let x: Int = 42\n");
        assert_eq!(module.body.len(), 1);
        match &module.body[0] {
            Statement::Let(binding) => {
                assert_eq!(binding.name, "x");
                assert!(!binding.mutable);
                assert!(binding.type_annotation.is_some());
            }
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_mut_binding() {
        let module = parse("mut x: Int = 42\n");
        match &module.body[0] {
            Statement::Let(binding) => {
                assert!(binding.mutable);
            }
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_func_def() {
        let module = parse("def add(a: Int, b: Int) -> Int:\n    return a + b\n");
        assert_eq!(module.body.len(), 1);
        match &module.body[0] {
            Statement::FuncDef(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert!(f.return_type.is_some());
                assert_eq!(f.body.len(), 1);
            }
            _ => panic!("expected func def"),
        }
    }

    #[test]
    fn test_data_decl() {
        let module = parse("data Point:\n    x: Int\n    y: Int\n");
        match &module.body[0] {
            Statement::DataDecl(d) => {
                assert_eq!(d.name, "Point");
                assert_eq!(d.fields.len(), 2);
                assert_eq!(d.fields[0].name, "x");
                assert_eq!(d.fields[1].name, "y");
            }
            _ => panic!("expected data decl"),
        }
    }

    #[test]
    fn test_data_with_type_params() {
        let module = parse("data Box[T]:\n    value: T\n");
        match &module.body[0] {
            Statement::DataDecl(d) => {
                assert_eq!(d.name, "Box");
                assert_eq!(d.type_params, vec!["T"]);
            }
            _ => panic!("expected data decl"),
        }
    }

    #[test]
    fn test_if_elif_else() {
        let module = parse("if x:\n    1\nelif y:\n    2\nelse:\n    3\n");
        match &module.body[0] {
            Statement::If(stmt) => {
                assert_eq!(stmt.elif_clauses.len(), 1);
                assert!(stmt.else_body.is_some());
            }
            _ => panic!("expected if stmt"),
        }
    }

    #[test]
    fn test_while() {
        let module = parse("while a > 0:\n    42");

        match &module.body[0] {
            Statement::While(stmt) => {
                if let Expr::BinOp(bin_op) = &stmt.condition {
                    assert_eq!(bin_op.op, BinOp::Gt);
                } else {
                    panic!("Expected binary operation in while condition");
                }
                    assert_eq!(stmt.body.len(), 1);
            }
            _ => panic!("expected while stmt"),
        }
    }

    #[test]
    fn test_binary_expr_precedence() {
        let module = parse("let x: Int = 1 + 2 * 3\n");
        match &module.body[0] {
            Statement::Let(binding) => match &binding.value {
                Expr::BinOp(e) => {
                    assert_eq!(e.op, BinOp::Add);
                    match &e.right {
                        Expr::BinOp(inner) => assert_eq!(inner.op, BinOp::Mul),
                        _ => panic!("expected mul on right"),
                    }
                }
                _ => panic!("expected binop"),
            },
            _ => panic!("expected let"),
        }
    }

    #[test]
    fn test_function_call() {
        let module = parse("foo(1, 2)\n");
        match &module.body[0] {
            Statement::Expr(stmt) => match &stmt.expr {
                Expr::Call(call) => {
                    assert_eq!(call.args.len(), 2);
                }
                _ => panic!("expected call"),
            },
            _ => panic!("expected expr stmt"),
        }
    }

    #[test]
    fn test_attr_access() {
        let module = parse("obj.field\n");
        match &module.body[0] {
            Statement::Expr(stmt) => match &stmt.expr {
                Expr::Attr(attr) => {
                    assert_eq!(attr.attr, "field");
                }
                _ => panic!("expected attr"),
            },
            _ => panic!("expected expr stmt"),
        }
    }

    #[test]
    fn test_import_simple() {
        let module = parse("import math\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::Simple { module_path, alias } => {
                    assert_eq!(module_path, &["math"]);
                    assert!(alias.is_none());
                }
                _ => panic!("expected simple import"),
            },
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_import_alias() {
        let module = parse("import math as m\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::Simple { module_path, alias } => {
                    assert_eq!(module_path, &["math"]);
                    assert_eq!(alias.as_deref(), Some("m"));
                }
                _ => panic!("expected simple import"),
            },
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_import_dotted() {
        let module = parse("import os.path\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::Simple { module_path, .. } => {
                    assert_eq!(module_path, &["os", "path"]);
                }
                _ => panic!("expected simple import"),
            },
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_from_import() {
        let module = parse("from math import add\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::From { module_path, names } => {
                    assert_eq!(module_path, &["math"]);
                    assert_eq!(names.len(), 1);
                    assert_eq!(names[0].name, "add");
                    assert!(names[0].alias.is_none());
                }
                _ => panic!("expected from import"),
            },
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_from_import_multiple() {
        let module = parse("from math import add, sub\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::From { names, .. } => {
                    assert_eq!(names.len(), 2);
                    assert_eq!(names[0].name, "add");
                    assert_eq!(names[1].name, "sub");
                }
                _ => panic!("expected from import"),
            },
            _ => panic!("expected import"),
        }
    }

    #[test]
    fn test_from_import_alias() {
        let module = parse("from os.path import join as j\n");
        match &module.body[0] {
            Statement::Import(imp) => match &imp.kind {
                ImportKind::From { module_path, names } => {
                    assert_eq!(module_path, &["os", "path"]);
                    assert_eq!(names[0].name, "join");
                    assert_eq!(names[0].alias.as_deref(), Some("j"));
                }
                _ => panic!("expected from import"),
            },
            _ => panic!("expected import"),
        }
    }
}

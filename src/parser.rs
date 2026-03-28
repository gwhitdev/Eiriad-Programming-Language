use crate::ast::{BinaryOp, Expr, MatchArm, Pattern, Stmt, UnaryOp};
use crate::error::{EiriadError, EiriadResult};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> EiriadResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        self.skip_terminators();
        while !self.is_eof() {
            stmts.push(self.parse_stmt()?);
            self.skip_terminators();
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> EiriadResult<Stmt> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let(false),
            TokenKind::Mut => self.parse_let(true),
            TokenKind::Fn => self.parse_fn_decl(),
            TokenKind::Ident(_) if self.lookahead_is_eq() => self.parse_assign(),
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_fn_decl(&mut self) -> EiriadResult<Stmt> {
        self.expect_token(TokenKind::Fn, "Expected 'fn'")?;
        let name = self.expect_ident()?;
        self.expect_token(TokenKind::LParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                params.push(self.expect_ident()?);
                if matches!(self.peek_kind(), TokenKind::Colon) {
                    self.advance();
                    self.skip_type_annotation_until_param_boundary();
                }
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect_token(TokenKind::RParen, "Expected ')' after parameter list")?;

        if matches!(self.peek_kind(), TokenKind::Arrow) {
            self.advance();
            self.skip_type_annotation_until_block_or_expr();
        }

        let body = self.parse_block_expr()?;
        Ok(Stmt::Fn { name, params, body })
    }

    fn parse_let(&mut self, mutable: bool) -> EiriadResult<Stmt> {
        self.advance();
        let name = self.expect_ident()?;

        // Skip type annotations for now: let x: Int = ...
        if matches!(self.peek_kind(), TokenKind::Colon) {
            self.advance();
            self.skip_type_annotation();
        }

        self.expect_token(TokenKind::Eq, "Expected '=' in declaration")?;
        let expr = self.parse_expr()?;
        Ok(Stmt::Let {
            name,
            mutable,
            expr,
        })
    }

    fn parse_assign(&mut self) -> EiriadResult<Stmt> {
        let name = self.expect_ident()?;
        self.expect_token(TokenKind::Eq, "Expected '=' in assignment")?;
        let expr = self.parse_expr()?;
        Ok(Stmt::Assign { name, expr })
    }

    fn parse_expr(&mut self) -> EiriadResult<Expr> {
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_or()?;
        while matches!(self.peek_kind(), TokenKind::PipeGt) {
            self.advance();
            let rhs = self.parse_call()?;
            expr = rewrite_pipe(expr, rhs)?;
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_and()?;
        while matches!(self.peek_kind(), TokenKind::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_equality()?;
        while matches!(self.peek_kind(), TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::Ne => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_power()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> EiriadResult<Expr> {
        let expr = self.parse_unary()?;
        if matches!(self.peek_kind(), TokenKind::Caret) {
            self.advance();
            let right = self.parse_power()?;
            Ok(Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Pow,
                right: Box::new(right),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_unary(&mut self) -> EiriadResult<Expr> {
        match self.peek_kind() {
            TokenKind::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> EiriadResult<Expr> {
        let mut expr = self.parse_primary()?;
        while matches!(self.peek_kind(), TokenKind::LParen) {
            self.advance();
            let mut args = Vec::new();
            if !matches!(self.peek_kind(), TokenKind::RParen) {
                loop {
                    args.push(self.parse_expr()?);
                    if matches!(self.peek_kind(), TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
            self.expect_token(TokenKind::RParen, "Expected ')' after arguments")?;
            expr = Expr::Call {
                callee: Box::new(expr),
                args,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> EiriadResult<Expr> {
        let token = self.advance();
        match token.kind {
            TokenKind::Int(value) => Ok(Expr::Int(value)),
            TokenKind::Float(value) => Ok(Expr::Float(value)),
            TokenKind::True => Ok(Expr::Bool(true)),
            TokenKind::False => Ok(Expr::Bool(false)),
            TokenKind::None => Ok(Expr::None),
            TokenKind::Str(value) => Ok(Expr::Str(value)),
            TokenKind::Ident(name) => Ok(Expr::Ident(name)),
            TokenKind::LParen => self.parse_group_or_lambda(),
            TokenKind::Match => self.parse_match_expr(),
            _ => Err(EiriadError::new(format!(
                "Unexpected token in expression: {}",
                token.lexeme
            ))),
        }
    }

    fn parse_group_or_lambda(&mut self) -> EiriadResult<Expr> {
        let saved = self.pos;

        let mut params = Vec::new();
        let mut lambda_ok = true;
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                match self.peek_kind() {
                    TokenKind::Ident(name) => {
                        params.push(name.clone());
                        self.advance();
                    }
                    _ => {
                        lambda_ok = false;
                        break;
                    }
                }

                if matches!(self.peek_kind(), TokenKind::Colon) {
                    self.advance();
                    self.skip_type_annotation_until_param_boundary();
                }

                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }

        if lambda_ok
            && matches!(self.peek_kind(), TokenKind::RParen)
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| matches!(t.kind, TokenKind::Arrow))
        {
            self.advance();
            self.advance();
            let body = if matches!(self.peek_kind(), TokenKind::LBrace) {
                self.parse_block_expr()?
            } else {
                self.parse_expr()?
            };
            Ok(Expr::Lambda {
                params,
                body: Box::new(body),
            })
        } else {
            self.pos = saved;
            let expr = self.parse_expr()?;
            self.expect_token(TokenKind::RParen, "Expected ')' after expression")?;
            Ok(expr)
        }
    }

    fn parse_block_expr(&mut self) -> EiriadResult<Expr> {
        self.expect_token(TokenKind::LBrace, "Expected '{'")?;
        self.skip_terminators();
        let expr = self.parse_expr()?;
        self.skip_terminators();
        self.expect_token(TokenKind::RBrace, "Expected '}'")?;
        Ok(expr)
    }

    fn parse_match_expr(&mut self) -> EiriadResult<Expr> {
        let value = self.parse_expr()?;
        self.expect_token(TokenKind::LBrace, "Expected '{' after match value")?;
        self.skip_terminators();

        let mut arms = Vec::new();
        while !matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof) {
            let pattern = self.parse_pattern()?;
            self.expect_token(TokenKind::Arrow, "Expected '->' in match arm")?;
            let expr = self.parse_expr()?;
            arms.push(MatchArm { pattern, expr });

            if matches!(self.peek_kind(), TokenKind::Comma) {
                self.advance();
            }
            self.skip_terminators();
        }

        self.expect_token(TokenKind::RBrace, "Expected '}' to end match")?;
        if arms.is_empty() {
            return Err(EiriadError::new("match requires at least one arm"));
        }
        Ok(Expr::Match {
            value: Box::new(value),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> EiriadResult<Pattern> {
        let token = self.advance();
        match token.kind {
            TokenKind::None => Ok(Pattern::None),
            TokenKind::Ident(name) if name == "_" => Ok(Pattern::Wildcard),
            TokenKind::Ident(name) if name == "Some" => {
                self.expect_token(TokenKind::LParen, "Expected '(' after Some")?;
                let inner = self.parse_pattern()?;
                self.expect_token(TokenKind::RParen, "Expected ')' after Some pattern")?;
                Ok(Pattern::Some(Box::new(inner)))
            }
            TokenKind::Ident(name) if name == "Ok" => {
                self.expect_token(TokenKind::LParen, "Expected '(' after Ok")?;
                let inner = self.parse_pattern()?;
                self.expect_token(TokenKind::RParen, "Expected ')' after Ok pattern")?;
                Ok(Pattern::Ok(Box::new(inner)))
            }
            TokenKind::Ident(name) if name == "Err" => {
                self.expect_token(TokenKind::LParen, "Expected '(' after Err")?;
                let inner = self.parse_pattern()?;
                self.expect_token(TokenKind::RParen, "Expected ')' after Err pattern")?;
                Ok(Pattern::Err(Box::new(inner)))
            }
            TokenKind::Ident(name) => Ok(Pattern::Ident(name)),
            _ => Err(EiriadError::new("Invalid match pattern")),
        }
    }

    fn skip_type_annotation(&mut self) {
        let mut depth = 0usize;
        while !self.is_eof() {
            match self.peek_kind() {
                TokenKind::Lt | TokenKind::LParen => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Gt | TokenKind::RParen => {
                    depth = depth.saturating_sub(1);
                    self.advance();
                }
                TokenKind::Eq if depth == 0 => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn skip_type_annotation_until_param_boundary(&mut self) {
        let mut depth = 0usize;
        while !self.is_eof() {
            match self.peek_kind() {
                TokenKind::LParen | TokenKind::Lt => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::RParen | TokenKind::Gt => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    self.advance();
                }
                TokenKind::Comma if depth == 0 => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn skip_type_annotation_until_block_or_expr(&mut self) {
        while !self.is_eof() {
            if matches!(self.peek_kind(), TokenKind::LBrace) {
                break;
            }
            if matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Semi) {
                break;
            }
            self.advance();
        }
    }

    fn lookahead_is_eq(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(_))
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| matches!(t.kind, TokenKind::Eq))
    }

    fn expect_ident(&mut self) -> EiriadResult<String> {
        match self.advance().kind {
            TokenKind::Ident(name) => Ok(name),
            _ => Err(EiriadError::new("Expected identifier")),
        }
    }

    fn expect_token(&mut self, expected: TokenKind, msg: &str) -> EiriadResult<()> {
        if std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(EiriadError::new(msg))
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Semi) {
            self.advance();
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }
}

fn rewrite_pipe(left: Expr, right: Expr) -> EiriadResult<Expr> {
    match right {
        Expr::Call { callee, mut args } => {
            args.insert(0, left);
            Ok(Expr::Call { callee, args })
        }
        Expr::Ident(name) => Ok(Expr::Call {
            callee: Box::new(Expr::Ident(name)),
            args: vec![left],
        }),
        _ => Err(EiriadError::new(
            "Right side of '|>' must be a function call or function name",
        )),
    }
}

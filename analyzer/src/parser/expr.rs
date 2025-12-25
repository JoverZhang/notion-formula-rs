use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp, UnOpKind};
use crate::parser::{Parser, prefix_binding_power};
use crate::parser::{ParseError, infix_binding_power};
use crate::token::{Lit, LitKind, Symbol, TokenKind, TokenRange};

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_bp(0)?;

        if !self.same_kind(self.cur_kind(), &TokenKind::Eof) {
            let tok = self.cur().clone();
            return Err(ParseError::UnexpectedToken {
                expected: "EOF".to_string(),
                found: tok.kind,
                span: tok.span,
            });
        }
        Ok(expr)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let op = match self.cur_kind() {
                TokenKind::Lt => BinOpKind::Lt,
                TokenKind::Le => BinOpKind::Le,
                TokenKind::EqEq => BinOpKind::EqEq,
                TokenKind::Ne => BinOpKind::Ne,
                TokenKind::Ge => BinOpKind::Ge,
                TokenKind::Gt => BinOpKind::Gt,
                TokenKind::AndAnd => BinOpKind::AndAnd,
                TokenKind::OrOr => BinOpKind::OrOr,
                TokenKind::Plus => BinOpKind::Plus,
                TokenKind::Minus => BinOpKind::Minus,
                TokenKind::Star => BinOpKind::Star,
                TokenKind::Slash => BinOpKind::Slash,
                TokenKind::Percent => BinOpKind::Percent,
                TokenKind::Caret => BinOpKind::Caret,
                _ => break,
            };

            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }

            // consume op
            let op_tok_idx = self.cur_idx();
            let op_tok = self.bump();

            let rhs = self.parse_expr_bp(r_bp)?;

            let tokens = TokenRange::new(lhs.tokens.lo, rhs.tokens.hi);
            let span = self.span_from_tokens(tokens);

            lhs = self.mk_expr(
                span,
                tokens,
                ExprKind::Binary {
                    op: BinOp {
                        node: op,
                        span: op_tok.span,
                    },
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
            );
            // You might want to use op_tok_idx in the future for fixing the operator token location
            let _ = op_tok_idx;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.cur_kind() {
            TokenKind::Bang => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Not))?;
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                Ok(self.mk_expr(
                    span,
                    tokens,
                    ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Not,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                ))
            }
            TokenKind::Minus => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Neg))?;
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                Ok(self.mk_expr(
                    span,
                    tokens,
                    ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Neg,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                ))
            }
            _ => self.parse_postfix_primary(),
        }
    }

    fn parse_postfix_primary(&mut self) -> Result<Expr, ParseError> {
        // primary: literal / ident / (expr)
        let mut expr = self.parse_primary()?;

        // postfix: call
        loop {
            if matches!(self.cur_kind(), TokenKind::OpenParen) {
                let lparen_idx = self.cur_idx();
                self.bump(); // consume '('

                let mut args = Vec::new();

                if !matches!(self.cur_kind(), TokenKind::CloseParen) {
                    args.push(self.parse_expr_bp(0)?);
                    while matches!(self.cur_kind(), TokenKind::Comma) {
                        self.bump(); // ','
                        args.push(self.parse_expr_bp(0)?);
                    }
                }

                self.expect_punct(TokenKind::CloseParen, "')'")?;

                // Only Ident can call
                let callee = match expr.kind {
                    ExprKind::Ident(sym) => sym,
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "call callee (identifier)".to_string(),
                            found: self.cur().kind.clone(),
                            span: self.cur().span,
                        });
                    }
                };

                let tokens = self.mk_token_range(expr.tokens.lo, self.cur_idx());
                let span = self.span_from_tokens(tokens);
                expr = self.mk_expr(span, tokens, ExprKind::Call { callee, args });

                let _ = lparen_idx;
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.cur_kind() {
            TokenKind::Ident(_) => {
                let start = self.cur_idx();
                let tok = self.expect_ident()?;
                let tokens = TokenRange::new(start, start + 1);
                let span = tok.span;

                // The ident text can be directly used from the Symbol in tok.kind
                let sym = match tok.kind {
                    TokenKind::Ident(sym) => sym,
                    _ => unreachable!(),
                };

                Ok(self.mk_expr(span, tokens, ExprKind::Ident(sym)))
            }

            TokenKind::Literal(lit) => match lit.kind {
                LitKind::Number => self.parse_number_literal(),
                LitKind::String => self.parse_string_literal(),
                LitKind::Bool => {
                    // You lexer currently doesn't produce bool tokens (if you add true/false keywords in the future, go here)
                    let tok = self.bump();
                    let idx = (self.token_cursor.pos - 1) as u32;
                    Ok(self.mk_expr(
                        tok.span,
                        TokenRange::new(idx, idx + 1),
                        ExprKind::Lit(Lit {
                            kind: LitKind::Bool,
                            symbol: Symbol {
                                text: self.lit_text(tok.span).into(),
                            },
                        }),
                    ))
                }
            },

            TokenKind::OpenParen => {
                let start = self.cur_idx();
                self.bump(); // '('
                let mut inner = self.parse_expr_bp(0)?;
                self.expect_punct(TokenKind::CloseParen, "')'")?;

                // Wrap the parentheses token range around inner (without keeping the Group node)
                let tokens = TokenRange::new(start, self.cur_idx()); // hi points after ')'
                let span = self.span_from_tokens(tokens);
                inner.tokens = tokens;
                inner.span = span;
                Ok(inner)
            }

            _ => {
                let tok = self.cur().clone();
                Err(ParseError::UnexpectedToken {
                    expected: "primary expression".into(),
                    found: tok.kind,
                    span: tok.span,
                })
            }
        }
    }

    fn parse_number_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.cur_idx();
        let tok = self.expect_literal_kind(LitKind::Number)?;
        let tokens = TokenRange::new(start, start + 1);

        Ok(self.mk_expr(
            tok.span,
            tokens,
            ExprKind::Lit(Lit {
                kind: LitKind::Number,
                symbol: Symbol {
                    text: self.lit_text(tok.span).into(),
                },
            }),
        ))
    }

    fn parse_string_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.cur_idx();
        let tok = self.expect_literal_kind(LitKind::String)?;
        let tokens = TokenRange::new(start, start + 1);

        let text = self.lit_text(tok.span);
        let inner = if text.len() >= 2 {
            &text[1..text.len() - 1]
        } else {
            ""
        };

        Ok(self.mk_expr(
            tok.span,
            tokens,
            ExprKind::Lit(Lit {
                kind: LitKind::String,
                symbol: Symbol { text: inner.into() },
            }),
        ))
    }
}

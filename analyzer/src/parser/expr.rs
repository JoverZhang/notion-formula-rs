use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp, UnOpKind};
use crate::diagnostics::Label;
use crate::parser::{ParseOutput, infix_binding_power};
use crate::parser::{Parser, prefix_binding_power};
use crate::token::{Lit, LitKind, Span, Symbol, TokenKind, TokenRange};

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> ParseOutput {
        let expr = self.parse_expr_bp(0);

        if !self.same_kind(self.cur_kind(), &TokenKind::Eof) {
            let tok = self.cur().clone();
            self.diagnostics.emit_error(tok.span, "expected EOF");
        }
        self.finish(expr)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            if matches!(self.cur_kind(), TokenKind::Question) {
                let q_tok_idx = self.cur_idx();
                let _q_tok = self.bump(); // '?'
                let then_expr = self.parse_expr_bp(0);
                let else_expr = match self.expect_punct(TokenKind::Colon, "':'") {
                    Ok(_) => self.parse_expr_bp(0),
                    Err(()) => self.parse_expr_bp(0),
                };

                let tokens = TokenRange::new(lhs.tokens.lo, else_expr.tokens.hi);
                let span = self.span_from_tokens(tokens);
                lhs = self.mk_expr(
                    span,
                    tokens,
                    ExprKind::Ternary {
                        cond: Box::new(lhs),
                        then: Box::new(then_expr),
                        otherwise: Box::new(else_expr),
                    },
                );

                let _ = q_tok_idx;
                continue;
            }

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

            let rhs = if self.cur().can_begin_expr() {
                self.parse_expr_bp(r_bp)
            } else {
                self.diagnostics
                    .emit_error(op_tok.span, format!("expected expression after '{:?}'", op));
                let err_span = if matches!(self.cur_kind(), TokenKind::Eof) {
                    Span {
                        start: op_tok.span.end,
                        end: op_tok.span.end,
                    }
                } else {
                    self.cur().span
                };
                let sync = [
                    TokenKind::Comma,
                    TokenKind::CloseParen,
                    TokenKind::Colon,
                    TokenKind::Eof,
                ];
                if !sync.iter().any(|k| self.same_kind(self.cur_kind(), k))
                    && !matches!(self.cur_kind(), TokenKind::Eof)
                {
                    self.bump();
                    self.recover_to(&sync);
                }
                self.error_expr_at(err_span)
            };

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

        lhs
    }

    fn parse_prefix(&mut self) -> Expr {
        match self.cur_kind() {
            TokenKind::Bang => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Not));
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                self.mk_expr(
                    span,
                    tokens,
                    ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Not,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                )
            }
            TokenKind::Minus => {
                let start = self.cur_idx();
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Neg));
                let tokens = TokenRange::new(start, expr.tokens.hi);
                let span = self.span_from_tokens(tokens);

                self.mk_expr(
                    span,
                    tokens,
                    ExprKind::Unary {
                        op: UnOp {
                            node: UnOpKind::Neg,
                            span: tok.span,
                        },
                        expr: Box::new(expr),
                    },
                )
            }
            _ => self.parse_postfix_primary(),
        }
    }

    fn parse_postfix_primary(&mut self) -> Expr {
        // primary: literal / ident / (expr)
        let mut expr = self.parse_primary();

        // postfix: call
        loop {
            if matches!(self.cur_kind(), TokenKind::OpenParen) {
                let lparen_idx = self.cur_idx();
                self.bump(); // consume '('

                let mut args = Vec::new();

                if !matches!(self.cur_kind(), TokenKind::CloseParen) {
                    args.push(self.parse_expr_bp(0));
                    while matches!(self.cur_kind(), TokenKind::Comma) {
                        self.bump(); // ','
                        args.push(self.parse_expr_bp(0));
                    }
                }

                if let Err(()) = self.expect_punct(TokenKind::CloseParen, "')'") {
                    self.recover_to(&[TokenKind::CloseParen, TokenKind::Comma, TokenKind::Eof]);
                    if matches!(self.cur_kind(), TokenKind::CloseParen) {
                        self.bump();
                    }
                }

                // Only Ident can call
                let callee = match expr.kind {
                    ExprKind::Ident(sym) => sym,
                    _ => {
                        self.diagnostics
                            .emit_error(self.cur().span, "expected call callee (identifier)");
                        let tokens = self.mk_token_range(expr.tokens.lo, self.cur_idx());
                        let span = self.span_from_tokens(tokens);
                        expr = self.mk_expr(span, tokens, ExprKind::Error);
                        break;
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

        expr
    }

    fn parse_primary(&mut self) -> Expr {
        match self.cur_kind() {
            TokenKind::Ident(_) => {
                let start = self.cur_idx();
                let tok = match self.expect_ident() {
                    Ok(tok) => tok,
                    Err(()) => {
                        return self.error_expr_bump();
                    }
                };
                let tokens = TokenRange::new(start, start + 1);
                let span = tok.span;

                // The ident text can be directly used from the Symbol in tok.kind
                let sym = match tok.kind {
                    TokenKind::Ident(sym) => sym,
                    _ => unreachable!(),
                };

                self.mk_expr(span, tokens, ExprKind::Ident(sym))
            }

            TokenKind::Literal(lit) => match lit.kind {
                LitKind::Number => self.parse_number_literal(),
                LitKind::String => self.parse_string_literal(),
                LitKind::Bool => {
                    // You lexer currently doesn't produce bool tokens (if you add true/false keywords in the future, go here)
                    let tok = self.bump();
                    let idx = (self.token_cursor.pos - 1) as u32;
                    self.mk_expr(
                        tok.span,
                        TokenRange::new(idx, idx + 1),
                        ExprKind::Lit(Lit {
                            kind: LitKind::Bool,
                            symbol: Symbol {
                                text: self.lit_text(tok.span).into(),
                            },
                        }),
                    )
                }
            },

            TokenKind::OpenParen => {
                let start = self.cur_idx();
                let lparen = self.bump(); // '('
                let mut inner = self.parse_expr_bp(0);
                if matches!(self.cur_kind(), TokenKind::CloseParen) {
                    self.bump();
                } else {
                    let found = self.cur().clone();
                    let labels = vec![Label {
                        span: lparen.span,
                        message: Some("this '(' is not closed".into()),
                    }];
                    let primary_span = if matches!(found.kind, TokenKind::Eof) {
                        Span {
                            start: found.span.start,
                            end: found.span.start,
                        }
                    } else {
                        found.span
                    };
                    self.diagnostics.emit_error_with_labels(
                        primary_span,
                        format!("expected ')', found {:?}", found.kind),
                        labels,
                    );
                    self.recover_to(&[
                        TokenKind::CloseParen,
                        TokenKind::Comma,
                        TokenKind::Colon,
                        TokenKind::Eof,
                    ]);
                    if matches!(self.cur_kind(), TokenKind::CloseParen) {
                        self.bump();
                    }
                }

                // Wrap the parentheses token range around inner (without keeping the Group node)
                let tokens = TokenRange::new(start, self.cur_idx()); // hi points after ')'
                let span = self.span_from_tokens(tokens);
                inner.tokens = tokens;
                inner.span = span;
                inner
            }

            _ => {
                let tok = self.cur().clone();
                self.diagnostics.emit_error(
                    tok.span,
                    format!("expected primary expression, found {:?}", tok.kind),
                );
                self.error_expr_bump()
            }
        }
    }

    fn parse_number_literal(&mut self) -> Expr {
        let start = self.cur_idx();
        let tok = match self.expect_literal_kind(LitKind::Number) {
            Ok(tok) => tok,
            Err(()) => {
                return self.error_expr_bump();
            }
        };
        let tokens = TokenRange::new(start, start + 1);

        self.mk_expr(
            tok.span,
            tokens,
            ExprKind::Lit(Lit {
                kind: LitKind::Number,
                symbol: Symbol {
                    text: self.lit_text(tok.span).into(),
                },
            }),
        )
    }

    fn parse_string_literal(&mut self) -> Expr {
        let start = self.cur_idx();
        let tok = match self.expect_literal_kind(LitKind::String) {
            Ok(tok) => tok,
            Err(()) => {
                return self.error_expr_bump();
            }
        };
        let tokens = TokenRange::new(start, start + 1);

        let text = self.lit_text(tok.span);
        let inner = if text.len() >= 2 {
            &text[1..text.len() - 1]
        } else {
            ""
        };

        self.mk_expr(
            tok.span,
            tokens,
            ExprKind::Lit(Lit {
                kind: LitKind::String,
                symbol: Symbol { text: inner.into() },
            }),
        )
    }

    fn error_expr_at(&mut self, span: Span) -> Expr {
        let idx = self.cur_idx();
        self.mk_expr(span, TokenRange::new(idx, idx), ExprKind::Error)
    }

    fn error_expr_bump(&mut self) -> Expr {
        let idx = self.cur_idx();
        let tok = self.cur().clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.bump();
        }
        let hi = (idx + 1).min(self.token_cursor.tokens.len() as u32);
        self.mk_expr(tok.span, TokenRange::new(idx, hi), ExprKind::Error)
    }

    fn recover_to(&mut self, sync: &[TokenKind]) {
        while !sync.iter().any(|k| self.same_kind(self.cur_kind(), k)) {
            if matches!(self.cur_kind(), TokenKind::Eof) {
                return;
            }
            self.bump();
        }
    }
}

//! Expression parsing (Pratt parser).
//!
//! Produces an AST plus parse diagnostics. Spans are UTF-8 byte offsets with half-open semantics
//! `[start, end)`.

use super::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp, UnOpKind};
use super::{ParseOutput, Parser, infix_binding_power, prefix_binding_power};
use crate::diagnostics::Label;
use crate::lexer::{Lit, LitKind, Span, Symbol, TokenKind};

impl<'a> Parser<'a> {
    /// Parse a full expression and return the [`ParseOutput`].
    ///
    /// Supported forms:
    /// - literals and identifiers
    /// - unary `!expr` / `-expr`
    /// - binary operators (`+ - * / % ^`, comparisons, `&&`, `||`)
    /// - ternary `cond ? then : otherwise`
    /// - grouping `(expr)` and list literals `[expr, ...]`
    /// - calls `ident(arg1, ...)` and member calls `receiver.method(arg1, ...)`
    ///
    /// Span contract:
    /// - Spans are UTF-8 byte offsets into the source with half-open semantics `[start, end)`.
    /// - Token consumption skips trivia, so spans are anchored on non-trivia tokens.
    /// - A parent span covers from the first to last non-trivia token of the construct, including
    ///   any trivia that occurs between those anchor tokens.
    ///
    /// ```text
    /// "a + b * c" parses as '+' with rhs '*'
    /// spans cover from 'a' to 'c' (byte-based)
    /// ```
    pub fn parse_expr(&mut self) -> ParseOutput {
        let expr = self.parse_expr_bp(0);

        if !self.same_kind(self.cur_kind(), &TokenKind::Eof) {
            let tok = self.cur().clone();
            self.diagnostics.emit_error(
                tok.span,
                format!("unexpected token {:?} after expression", tok.kind),
            );
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

                let span = Span {
                    start: lhs.span.start,
                    end: else_expr.span.end,
                };
                lhs = self.mk_expr(
                    span,
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
                    TokenKind::CloseBracket,
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

            let span = Span {
                start: lhs.span.start,
                end: rhs.span.end,
            };

            lhs = self.mk_expr(
                span,
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
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Not));
                let span = Span {
                    start: tok.span.start,
                    end: expr.span.end,
                };

                self.mk_expr(
                    span,
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
                let tok = self.bump();
                let expr = self.parse_expr_bp(prefix_binding_power(UnOpKind::Neg));
                let span = Span {
                    start: tok.span.start,
                    end: expr.span.end,
                };

                self.mk_expr(
                    span,
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

        // postfix: call / member-call
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
                        let span = Span {
                            start: expr.span.start,
                            end: self.last_bumped_end(),
                        };
                        expr = self.mk_expr(span, ExprKind::Error);
                        break;
                    }
                };

                let span = Span {
                    start: expr.span.start,
                    end: self.last_bumped_end(),
                };
                expr = self.mk_expr(span, ExprKind::Call { callee, args });

                let _ = lparen_idx;
                continue;
            }

            if matches!(self.cur_kind(), TokenKind::Dot) {
                self.bump(); // '.'

                let method_tok = match self.expect_ident() {
                    Ok(tok) => tok,
                    Err(()) => {
                        let span = Span {
                            start: expr.span.start,
                            end: self.last_bumped_end(),
                        };
                        expr = self.mk_expr(span, ExprKind::Error);
                        break;
                    }
                };

                let method = match method_tok.kind {
                    TokenKind::Ident(sym) => sym,
                    _ => unreachable!(),
                };

                if !matches!(self.cur_kind(), TokenKind::OpenParen) {
                    self.diagnostics.emit_error(
                        method_tok.span,
                        "expected '(' after member name (member access is not supported yet)",
                    );
                    let span = Span {
                        start: expr.span.start,
                        end: self.last_bumped_end(),
                    };
                    expr = self.mk_expr(span, ExprKind::Error);
                    break;
                }

                self.bump(); // '('

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

                let receiver = expr;
                let span = Span {
                    start: receiver.span.start,
                    end: self.last_bumped_end(),
                };
                expr = self.mk_expr(
                    span,
                    ExprKind::MemberCall {
                        receiver: Box::new(receiver),
                        method,
                        args,
                    },
                );

                continue;
            }

            break;
        }

        expr
    }

    fn parse_primary(&mut self) -> Expr {
        match self.cur_kind() {
            TokenKind::Ident(_) => {
                let tok = match self.expect_ident() {
                    Ok(tok) => tok,
                    Err(()) => {
                        return self.error_expr_bump();
                    }
                };
                let span = tok.span;

                // The ident text can be directly used from the Symbol in tok.kind
                let sym = match tok.kind {
                    TokenKind::Ident(sym) => sym,
                    _ => unreachable!(),
                };

                self.mk_expr(span, ExprKind::Ident(sym))
            }

            TokenKind::Literal(lit) => match lit.kind {
                LitKind::Number => self.parse_number_literal(),
                LitKind::String => self.parse_string_literal(),
                LitKind::Bool => {
                    // You lexer currently doesn't produce bool tokens (if you add true/false keywords in the future, go here)
                    let tok = self.bump();
                    self.mk_expr(
                        tok.span,
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
                let lparen = self.bump(); // '('
                let inner = self.parse_expr_bp(0);
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
                        TokenKind::CloseBracket,
                        TokenKind::Comma,
                        TokenKind::Colon,
                        TokenKind::Eof,
                    ]);
                    if matches!(self.cur_kind(), TokenKind::CloseParen) {
                        self.bump();
                    }
                }

                let span = Span {
                    start: lparen.span.start,
                    end: self.last_bumped_end(),
                };
                self.mk_expr(
                    span,
                    ExprKind::Group {
                        inner: Box::new(inner),
                    },
                )
            }

            TokenKind::OpenBracket => self.parse_list_literal(),

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

    /// Parse a list literal: `[expr, expr, ...]`.
    ///
    /// Trailing comma (`[1, 2,]`) is rejected with a dedicated diagnostic:
    /// `trailing comma in list literal is not supported`.
    ///
    /// After a comma, the parser expects an expression and emits:
    /// `expected expression after ',' in list literal`.
    fn parse_list_literal(&mut self) -> Expr {
        let lbrack = self.bump(); // '['
        let mut items = Vec::new();

        if !matches!(self.cur_kind(), TokenKind::CloseBracket) {
            items.push(self.parse_expr_bp(0));
            while matches!(self.cur_kind(), TokenKind::Comma) {
                let comma = self.bump(); // ','

                if matches!(self.cur_kind(), TokenKind::CloseBracket) {
                    self.diagnostics.emit_error(
                        comma.span,
                        "trailing comma in list literal is not supported",
                    );
                    break;
                }

                if !self.cur().can_begin_expr() {
                    let found = self.cur().clone();
                    self.diagnostics
                        .emit_error(found.span, "expected expression after ',' in list literal");
                    if !matches!(found.kind, TokenKind::Eof) {
                        self.bump();
                    }
                    self.recover_to(&[
                        TokenKind::Comma,
                        TokenKind::CloseBracket,
                        TokenKind::CloseParen,
                        TokenKind::Colon,
                        TokenKind::Eof,
                    ]);
                    if !self.cur().can_begin_expr() {
                        continue;
                    }
                }

                items.push(self.parse_expr_bp(0));
            }
        }

        if let Err(()) = self.expect_punct(TokenKind::CloseBracket, "']'") {
            let found = self.cur().clone();
            let labels = vec![Label {
                span: lbrack.span,
                message: Some("this '[' is not closed".into()),
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
                format!("expected ']', found {:?}", found.kind),
                labels,
            );
            self.recover_to(&[
                TokenKind::CloseBracket,
                TokenKind::Comma,
                TokenKind::CloseParen,
                TokenKind::Colon,
                TokenKind::Eof,
            ]);
            if matches!(self.cur_kind(), TokenKind::CloseBracket) {
                self.bump();
            }
        }

        let span = Span {
            start: lbrack.span.start,
            end: self.last_bumped_end(),
        };
        self.mk_expr(span, ExprKind::List { items })
    }

    fn parse_number_literal(&mut self) -> Expr {
        let tok = match self.expect_literal_kind(LitKind::Number) {
            Ok(tok) => tok,
            Err(()) => {
                return self.error_expr_bump();
            }
        };

        self.mk_expr(
            tok.span,
            ExprKind::Lit(Lit {
                kind: LitKind::Number,
                symbol: Symbol {
                    text: self.lit_text(tok.span).into(),
                },
            }),
        )
    }

    fn parse_string_literal(&mut self) -> Expr {
        let tok = match self.expect_literal_kind(LitKind::String) {
            Ok(tok) => tok,
            Err(()) => {
                return self.error_expr_bump();
            }
        };

        let text = self.lit_text(tok.span);
        let inner = if text.len() >= 2 {
            &text[1..text.len() - 1]
        } else {
            ""
        };

        self.mk_expr(
            tok.span,
            ExprKind::Lit(Lit {
                kind: LitKind::String,
                symbol: Symbol { text: inner.into() },
            }),
        )
    }

    fn error_expr_at(&mut self, span: Span) -> Expr {
        self.mk_expr(span, ExprKind::Error)
    }

    fn error_expr_bump(&mut self) -> Expr {
        let tok = self.cur().clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.bump();
        }
        self.mk_expr(tok.span, ExprKind::Error)
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

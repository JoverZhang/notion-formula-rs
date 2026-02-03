//! Expression parsing (Pratt parser).
//!
//! Produces an AST plus parse diagnostics. Spans are UTF-8 byte offsets with half-open semantics
//! `[start, end)`.

use super::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp};
use super::{ParseOutput, Parser, infix_binding_power, prefix_binding_power};
use crate::Token;
use crate::diagnostics::Label;
use crate::lexer::{Lit, LitKind, Span, Symbol, TokenKind};

impl<'a> Parser<'a> {
    /// Parser's entry point
    ///
    /// Supported forms:
    /// - literals: `1`, `"hello"`, `true`, `false`, and list literals: `[expr, ...]`
    /// - identifiers: `x`, `y`, `z`
    /// - grouping(parentheses): `(expr)`
    /// - unary: [`UnOp`]
    /// - binary: [`BinOpKind`]
    /// - ternary: `cond ? then : otherwise`
    /// - calls `ident(arg1, ...)` and member calls `receiver.method(arg1, ...)`
    ///
    /// Span contract:
    /// - Spans are UTF-8 byte offsets into the source with half-open semantics `[start, end)`.
    /// - Token consumption skips trivia, so spans are anchored on non-trivia tokens.
    /// - A parent span covers from the first to last non-trivia token of the construct, including
    ///   any trivia that occurs between those anchor tokens.
    ///
    /// ```text
    /// `a + b * c` parses as `+` with rhs `*`
    /// spans cover from `a` to `c` (byte-based)
    ///
    /// `2 ^ 3 ^ 2` parses as `2 ^ (3 ^ 2)`   // right-associative
    /// `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`   // right-associative
    /// `1 + 2 * 3` parses as `1 + (2 * 3)`
    /// `1 > 2 || 3 > 4 ? "x" : "y"` parses as `(1 > 2 || 3 > 4) ? "x" : "y"`
    /// ```
    pub fn parse(&mut self) -> ParseOutput {
        let expr = self.parse_expr();

        if self.cur().kind != TokenKind::Eof {
            self.diagnostics.emit_err(
                self.cur().span,
                format!("unexpected token {:?} after expression", self.cur().kind),
            );
        }
        ParseOutput {
            expr,
            diagnostics: std::mem::take(&mut self.diagnostics.diags),
            tokens: self.token_cursor.tokens.clone(),
        }
    }

    /// Parses an expression.
    pub fn parse_expr(&mut self) -> Expr {
        self.parse_expr_assoc_with(0)
    }

    /// Parses an associative expression with operators of at least `min_bp` precedence.
    fn parse_expr_assoc_with(&mut self, min_bp: u8) -> Expr {
        let lhs = self.parse_expr_prefix();
        self.parse_expr_assoc_rest_with(min_bp, lhs)
    }

    /// Parses the rest of an associative expression (i.e. the part after the lhs) with operators
    /// of at least `min_bp` precedence.
    fn parse_expr_assoc_rest_with(&mut self, min_bp: u8, lhs: Expr) -> Expr {
        let mut lhs = lhs;

        loop {
            match self.cur().kind {
                // Ternary expression: `cond(lhs) ? then : otherwise`
                TokenKind::Question => {
                    let (l_bp, r_bp) = Self::ternary_binding_power();
                    if l_bp < min_bp {
                        break;
                    }

                    let q_tok = self.bump(); // '?'
                    lhs = self.parse_ternary_suffix(lhs, q_tok.span, r_bp);
                }

                // Binary expression: `lhs op rhs`
                _ => {
                    let Some(op) = self.peek_binop_kind() else {
                        break;
                    };

                    let (l_bp, r_bp) = infix_binding_power(op);
                    if l_bp < min_bp {
                        break;
                    }

                    let op_tok = self.bump(); // operator token

                    let rhs = if self.cur().can_begin_expr() {
                        self.parse_expr_assoc_with(r_bp)
                    } else {
                        self.recover_from_infix(op_tok.clone())
                    };

                    let span = self.mk_expr_sp(lhs.span, rhs.span);
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
                }
            }
        }

        lhs
    }

    /// Parses a prefix-unary-operator expr.
    /// Note: when adding new unary operators, don't forget to adjust [`Token::can_begin_expr()`]
    fn parse_expr_prefix(&mut self) -> Expr {
        match self.cur().kind {
            // `!expr`
            TokenKind::Bang => self.parse_expr_unary(UnOp::Not),
            // `-expr`
            TokenKind::Minus => self.parse_expr_unary(UnOp::Neg),
            // Parses `a.b()` or `a(13)` or just `a`.
            _ => {
                let base = self.parse_expr_primary();
                self.parse_expr_dot_or_call(base)
            }
        }
    }

    fn parse_expr_unary(&mut self, op: UnOp) -> Expr {
        let tok = self.bump();
        let expr = self.parse_expr_assoc_with(prefix_binding_power(op));
        let span = self.mk_expr_sp(tok.span, expr.span);
        self.mk_expr(
            span,
            ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
        )
    }

    fn parse_expr_dot_or_call(&mut self, mut expr: Expr) -> Expr {
        loop {
            if self.cur().kind == TokenKind::OpenParen {
                let lparen = self.cur(); // '('
                let (args, end) = self.parse_paren_arg_list();

                // Only Ident can call
                let callee = match expr.kind {
                    ExprKind::Ident(sym) => sym,
                    _ => {
                        self.diagnostics
                            .emit_err(lparen.span, "expected call callee (identifier)");
                        let span = Span {
                            start: expr.span.start,
                            end,
                        };
                        expr = self.mk_expr(span, ExprKind::Error);
                        break;
                    }
                };

                let span = Span {
                    start: expr.span.start,
                    end,
                };
                expr = self.mk_expr(span, ExprKind::Call { callee, args });
                continue;
            }

            if self.cur().kind == TokenKind::Dot {
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

                if self.cur().kind != TokenKind::OpenParen {
                    self.diagnostics.emit_err(
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

                let (args, end) = self.parse_paren_arg_list();

                let receiver = expr;
                let span = Span {
                    start: receiver.span.start,
                    end,
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

    fn parse_paren_arg_list(&mut self) -> (Vec<Expr>, u32) {
        self.bump(); // '('

        let mut args = Vec::new();
        if self.cur().kind != TokenKind::CloseParen {
            args.push(self.parse_expr_assoc_with(0));
            while self.cur().kind == TokenKind::Comma {
                self.bump(); // ','
                args.push(self.parse_expr_assoc_with(0));
            }
        }

        if let Err(()) = self.expect_punct(TokenKind::CloseParen, "')'") {
            self.recover_to(&[TokenKind::CloseParen, TokenKind::Comma, TokenKind::Eof]);
            if self.cur().kind == TokenKind::CloseParen {
                self.bump();
            }
        }

        (args, self.last_bumped_end())
    }

    fn parse_ternary_suffix(&mut self, lhs: Expr, _q_span: Span, else_min_bp: u8) -> Expr {
        let then_expr = self.parse_expr_assoc_with(0);

        if let Err(()) = self.expect_punct(TokenKind::Colon, "':'") {
            // Missing ':' in `cond ? then : else`.
            // Recover to the next plausible boundary; if we find ':', consume it and continue.
            self.recover_to(&[
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::CloseParen,
                TokenKind::CloseBracket,
                TokenKind::Eof,
            ]);
            if self.cur().kind == TokenKind::Colon {
                self.bump();
            } else {
                let insertion = then_expr.span.end;
                let else_expr = self.error_expr_at(Span {
                    start: insertion,
                    end: insertion,
                });
                let span = self.mk_expr_sp(lhs.span, else_expr.span);
                return self.mk_expr(
                    span,
                    ExprKind::Ternary {
                        cond: Box::new(lhs),
                        then: Box::new(then_expr),
                        otherwise: Box::new(else_expr),
                    },
                );
            }
        }

        let else_expr = self.parse_expr_assoc_with(else_min_bp);
        let span = self.mk_expr_sp(lhs.span, else_expr.span);
        self.mk_expr(
            span,
            ExprKind::Ternary {
                cond: Box::new(lhs),
                then: Box::new(then_expr),
                otherwise: Box::new(else_expr),
            },
        )
    }

    fn peek_binop_kind(&self) -> Option<BinOpKind> {
        match self.cur().kind {
            TokenKind::Lt => Some(BinOpKind::Lt),
            TokenKind::Le => Some(BinOpKind::Le),
            TokenKind::EqEq => Some(BinOpKind::EqEq),
            TokenKind::Ne => Some(BinOpKind::Ne),
            TokenKind::Ge => Some(BinOpKind::Ge),
            TokenKind::Gt => Some(BinOpKind::Gt),
            TokenKind::AndAnd => Some(BinOpKind::AndAnd),
            TokenKind::OrOr => Some(BinOpKind::OrOr),
            TokenKind::Plus => Some(BinOpKind::Plus),
            TokenKind::Minus => Some(BinOpKind::Minus),
            TokenKind::Star => Some(BinOpKind::Star),
            TokenKind::Slash => Some(BinOpKind::Slash),
            TokenKind::Percent => Some(BinOpKind::Percent),
            TokenKind::Caret => Some(BinOpKind::Caret),
            _ => None,
        }
    }

    /// Parses a primary expression: `a`, `1`, `"hello"`, `true`, `false`, `(expr)`, `[expr, ...]`.
    fn parse_expr_primary(&mut self) -> Expr {
        match self.cur().kind {
            TokenKind::Ident(_) => self.parse_ident(),

            TokenKind::Literal(lit) => match lit.kind {
                LitKind::Bool => self.parse_bool_literal(),
                LitKind::Number => self.parse_number_literal(),
                LitKind::String => self.parse_string_literal(),
            },

            TokenKind::OpenParen => self.parse_expr_tuple_parens(),

            TokenKind::OpenBracket => self.parse_list_literal(),

            _ => {
                let tok = self.cur();
                self.diagnostics.emit_err(
                    tok.span,
                    format!("expected primary expression, found {:?}", tok.kind),
                );
                self.error_expr_bump()
            }
        }
    }

    fn parse_ident(&mut self) -> Expr {
        let tok = self.bump(); // identifier

        // The ident text can be directly used from the Symbol in tok.kind
        let sym = match tok.kind {
            TokenKind::Ident(sym) => sym,
            _ => unreachable!(),
        };

        self.mk_expr(tok.span, ExprKind::Ident(sym))
    }

    fn parse_bool_literal(&mut self) -> Expr {
        let tok = self.bump(); // bool literal
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

    fn parse_number_literal(&mut self) -> Expr {
        let tok = self.bump(); // number literal
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
        let tok = self.bump(); // string literal

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

    fn parse_expr_tuple_parens(&mut self) -> Expr {
        let lparen = self.bump(); // '('
        let inner = self.parse_expr_assoc_with(0);
        if self.cur().kind == TokenKind::CloseParen {
            self.bump();
        } else {
            let found = self.cur().clone();
            let labels = vec![Label {
                span: lparen.span,
                message: Some("this '(' is not closed".into()),
            }];
            let primary_span = if found.kind == TokenKind::Eof {
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
            if self.cur().kind == TokenKind::CloseParen {
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

    /// Parse a list literal: `[expr, expr, ...]`.
    ///
    /// Trailing comma (`[1, 2,]`) is rejected with a dedicated diagnostic:
    /// `trailing comma in list literal is not supported`.
    ///
    /// After a comma, the parser expects an expression and emits:
    /// `expected expression after ',' in list literal`.
    ///
    /// ```text
    /// "[1,2,]" -> "trailing comma in list literal is not supported"
    /// ```
    fn parse_list_literal(&mut self) -> Expr {
        let lbrack = self.bump(); // '['
        let mut items = Vec::new();

        if self.cur().kind != TokenKind::CloseBracket {
            items.push(self.parse_expr_assoc_with(0));
            while self.cur().kind == TokenKind::Comma {
                let comma = self.bump(); // ','

                if self.cur().kind == TokenKind::CloseBracket {
                    self.diagnostics.emit_err(
                        comma.span,
                        "trailing comma in list literal is not supported",
                    );
                    break;
                }

                if !self.cur().can_begin_expr() {
                    let found = self.cur().clone();
                    self.diagnostics
                        .emit_err(found.span, "expected expression after ',' in list literal");
                    if found.kind != TokenKind::Eof {
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

                items.push(self.parse_expr_assoc_with(0));
            }
        }

        if let Err(()) = self.expect_punct(TokenKind::CloseBracket, "']'") {
            let found = self.cur().clone();
            let labels = vec![Label {
                span: lbrack.span,
                message: Some("this '[' is not closed".into()),
            }];
            let primary_span = if found.kind == TokenKind::Eof {
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
            if self.cur().kind == TokenKind::CloseBracket {
                self.bump();
            }
        }

        let span = Span {
            start: lbrack.span.start,
            end: self.last_bumped_end(),
        };
        self.mk_expr(span, ExprKind::List { items })
    }

    fn error_expr_at(&mut self, span: Span) -> Expr {
        self.mk_expr(span, ExprKind::Error)
    }

    fn error_expr_bump(&mut self) -> Expr {
        let tok = self.cur();
        if tok.kind != TokenKind::Eof {
            self.bump();
        }
        self.mk_expr(tok.span, ExprKind::Error)
    }

    /// Skip tokens until reaching a synchronization token.
    ///
    /// This is used after emitting a diagnostic to avoid cascading errors.
    /// Typical sync sets in this parser include `,`, `)`, `]`, `:`, and `Eof`.
    ///
    /// Error nodes produced during recovery are [`ExprKind::Error`] and usually carry:
    /// - the span of the unexpected token, or
    /// - an empty span at end-of-input (an insertion point).
    ///
    /// ```text
    /// source: "(a + b"
    /// parsing: expects ')', finds Eof
    /// recovery: recover_to([CloseParen, Comma, Eof]) stops at Eof; an Error expr may use an empty span
    ///
    /// source: "a + )"
    /// parsing: expects an expression after '+', finds ')'
    /// recovery: sync includes ')', so an Error expr often uses the ')' token span
    /// ```
    fn recover_to(&mut self, sync: &[TokenKind]) {
        while !sync.iter().any(|k| self.cur().kind == *k) {
            if self.cur().kind == TokenKind::Eof {
                return;
            }
            self.bump();
        }
    }

    fn ternary_binding_power() -> (u8, u8) {
        // Lower precedence than `||` so `a || b ? c : d` parses as `(a || b) ? c : d`.
        // Right-associative: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.
        (0, 0)
    }

    fn recover_from_infix(&mut self, op_tok: Token) -> Expr {
        self.diagnostics.emit_err(
            op_tok.span,
            format!("expected expression after '{:?}'", op_tok.kind),
        );
        let err_span = if self.cur().kind == TokenKind::Eof {
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
        if !sync.iter().any(|k| self.cur().kind == *k) && self.cur().kind != TokenKind::Eof {
            self.bump();
            self.recover_to(&sync);
        }
        self.error_expr_at(err_span)
    }
}

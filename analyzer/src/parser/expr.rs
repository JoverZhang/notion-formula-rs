//! Expression parsing (Pratt parser).
//!
//! Produces an AST plus parse diagnostics. Spans are UTF-8 byte offsets with half-open semantics
//! `[start, end)`.

use super::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp};
use super::{ParseOutput, Parser};
use crate::Token;
use crate::diagnostics::{DiagnosticCode, Label, ParseDiagnostic};
use crate::lexer::{Lit, LitKind, Span, Symbol, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeqContext {
    CallArgList,
    ListLiteral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delimiter {
    Paren,
    Bracket,
}

impl Delimiter {
    fn close(self) -> TokenKind {
        match self {
            Delimiter::Paren => TokenKind::CloseParen,
            Delimiter::Bracket => TokenKind::CloseBracket,
        }
    }

    fn close_expected(self) -> &'static str {
        match self {
            Delimiter::Paren => "')'",
            Delimiter::Bracket => "']'",
        }
    }

    fn unclosed_label(self) -> &'static str {
        match self {
            Delimiter::Paren => "this '(' is not closed",
            Delimiter::Bracket => "this '[' is not closed",
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct DelimDepth {
    paren: u32,
    bracket: u32,
}

impl DelimDepth {
    fn at_top_level(self) -> bool {
        self.paren == 0 && self.bracket == 0
    }

    fn consume(&mut self, kind: TokenKind) {
        match kind {
            TokenKind::OpenParen => self.paren = self.paren.saturating_add(1),
            TokenKind::OpenBracket => self.bracket = self.bracket.saturating_add(1),
            TokenKind::CloseParen => {
                if self.paren > 0 {
                    self.paren -= 1;
                }
            }
            TokenKind::CloseBracket => {
                if self.bracket > 0 {
                    self.bracket -= 1;
                }
            }
            _ => {}
        }
    }
}

fn is_closing_delim(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::CloseParen | TokenKind::CloseBracket)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoverScanDecision {
    Continue,
    Stop,
    ConsumeAndStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoverScanResult {
    HitEof,
    Stopped,
    Consumed,
}

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
            let tok = self.cur();
            self.diagnostics.emit(
                DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                self.cur().span,
                format!(
                    "unexpected token {} after expression",
                    Self::describe_token(&tok.kind)
                ),
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

                    let (l_bp, r_bp) = op.infix_binding_power();
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
                            op,
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
        let expr = self.parse_expr_assoc_with(op.prefix_binding_power());
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
            match self.cur().kind {
                // Call expression: `expr(arg1, ...)`
                TokenKind::OpenParen => {
                    let lparen = self.cur(); // '('
                    let (args, end) = self.parse_paren_arg_list();

                    // Only Ident can call
                    let callee = match expr.kind {
                        ExprKind::Ident(sym) => sym,
                        _ => {
                            self.diagnostics.emit(
                                DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                                lparen.span,
                                "expected call callee (identifier)",
                            );
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
                }

                // Member call expression: `expr.method(arg1, ...)`
                TokenKind::Dot => {
                    self.bump(); // '.'

                    let method_tok = loop {
                        match self.cur().kind {
                            TokenKind::Ident(..) => break Ok(self.bump()),
                            // `a..if(...)`: recover by skipping extra dots.
                            TokenKind::Dot => {
                                let extra = self.bump();
                                self.diagnostics.emit(
                                    DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                                    extra.span,
                                    "unexpected '.' in member call",
                                );
                            }
                            _ => break Err(()),
                        }
                    };

                    let method_tok = match method_tok {
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
                        self.diagnostics.emit(
                            DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
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
                }
                _ => break,
            }
        }

        expr
    }

    fn parse_paren_arg_list(&mut self) -> (Vec<Expr>, u32) {
        let (_lparen, args) = self.parse_delimited(Delimiter::Paren, |p| {
            p.parse_seq_to_before_tokens(&[TokenKind::CloseParen], SeqContext::CallArgList)
        });
        (args, self.last_bumped_end())
    }

    fn parse_ternary_suffix(&mut self, lhs: Expr, q_span: Span, else_min_bp: u8) -> Expr {
        let then_expr = if self.cur().can_begin_expr() {
            self.parse_expr_assoc_with(0)
        } else {
            // `cond ? : else` / `cond ?` at end-of-input
            let insertion = q_span.end;
            let span = Span {
                start: insertion,
                end: insertion,
            };
            self.diagnostics.emit(
                DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                span,
                "expected expression after '?' in ternary expression",
            );
            self.error_expr_at(span)
        };

        if let Err(()) = self.expect_punct(TokenKind::Colon, "':'") {
            // Missing ':' in `cond ? then : else`.
            // Recover to the next plausible boundary; if we find ':', consume it and continue.
            if !self.recover_to_punct_or_expr_boundary(TokenKind::Colon) {
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

        let else_expr = if self.cur().can_begin_expr() {
            self.parse_expr_assoc_with(else_min_bp)
        } else {
            // `cond ? then : )` / `cond ? then :` at end-of-input
            let found = self.cur().clone();
            let insertion = self.last_bumped_end();
            let span = if found.kind == TokenKind::Eof {
                Span {
                    start: insertion,
                    end: insertion,
                }
            } else {
                found.span
            };
            self.diagnostics.emit(
                DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                span,
                "expected expression after ':' in ternary expression",
            );

            if !matches!(
                found.kind,
                TokenKind::Comma | TokenKind::CloseBracket | TokenKind::CloseParen | TokenKind::Eof
            ) {
                self.bump();
                self.recover_to_expr_boundary();
            }

            self.error_expr_at(span)
        };
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

    fn peek_binop_kind(&self) -> Option<BinOp> {
        let kind = match self.cur().kind {
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
            _ => return None,
        };
        Some(BinOp {
            node: kind,
            span: self.cur().span,
        })
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

            TokenKind::OpenParen => self.parse_paren_expr(),

            TokenKind::OpenBracket => self.parse_list_literal(),

            _ => {
                let tok = self.cur();
                self.diagnostics.emit(
                    DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                    tok.span,
                    format!(
                        "expected expression, found {}",
                        Self::describe_token(&tok.kind)
                    ),
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

    fn parse_paren_expr(&mut self) -> Expr {
        let (lparen, inner) =
            self.parse_delimited(Delimiter::Paren, |p| p.parse_expr_assoc_with(0));

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
        let (lbrack, items) = self.parse_delimited(Delimiter::Bracket, |p| {
            p.parse_seq_to_before_tokens(&[TokenKind::CloseBracket], SeqContext::ListLiteral)
        });

        let span = Span {
            start: lbrack.span.start,
            end: self.last_bumped_end(),
        };
        self.mk_expr(span, ExprKind::List { items })
    }

    fn parse_delimited<T>(
        &mut self,
        delim: Delimiter,
        parse_contents: impl FnOnce(&mut Self) -> T,
    ) -> (Token, T) {
        let open = self.bump(); // assumes the caller has checked `cur().kind`
        let value = parse_contents(self);
        self.expect_closing_delimiter(
            delim.close(),
            delim.close_expected(),
            open.span,
            delim.unclosed_label(),
        );
        (open, value)
    }

    fn parse_seq_to_before_tokens(
        &mut self,
        end_tokens: &[TokenKind],
        ctx: SeqContext,
    ) -> Vec<Expr> {
        debug_assert!(
            !end_tokens.is_empty(),
            "seq parsing needs at least one end token"
        );

        let mut items = Vec::new();
        let mut expecting_item = true;
        let mut after_comma = false;

        loop {
            let kind = self.cur().kind;

            if kind == TokenKind::Eof {
                if expecting_item && after_comma {
                    // `...,` then end-of-input: avoid calling into expression parsing so we don't
                    // consume would-be delimiters or cascade errors.
                    let insertion = self.cur().span.start;
                    let span = Span {
                        start: insertion,
                        end: insertion,
                    };
                    self.diagnostics.emit(
                        DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                        span,
                        match ctx {
                            SeqContext::CallArgList => {
                                "expected expression after ',' in argument list"
                            }
                            SeqContext::ListLiteral => {
                                "expected expression after ',' in list literal"
                            }
                        },
                    );
                }
                break;
            }

            if end_tokens.iter().any(|t| kind == *t) {
                break;
            }

            if expecting_item {
                if kind == TokenKind::Comma {
                    // `f(,a)` / `[1,,2]`: missing item before the comma.
                    let comma = self.bump();
                    self.diagnostics.emit(
                        DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                        comma.span,
                        match ctx {
                            SeqContext::CallArgList => {
                                "expected expression before ',' in argument list"
                            }
                            SeqContext::ListLiteral => {
                                "expected expression before ',' in list literal"
                            }
                        },
                    );
                    items.push(self.error_expr_at(comma.span));
                    after_comma = true;
                    continue;
                }

                if !self.cur().can_begin_expr() {
                    let found = self.cur().clone();
                    self.diagnostics.emit(
                        DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                        found.span,
                        match (ctx, after_comma) {
                            (SeqContext::CallArgList, true) => {
                                "expected expression after ',' in argument list"
                            }
                            (SeqContext::ListLiteral, true) => {
                                "expected expression after ',' in list literal"
                            }
                            (SeqContext::CallArgList, false) => {
                                "expected expression in argument list"
                            }
                            (SeqContext::ListLiteral, false) => {
                                "expected expression in list literal"
                            }
                        },
                    );
                    if found.kind != TokenKind::Eof {
                        self.bump();
                    }
                    self.recover_seq_to_comma_or_tokens(end_tokens);
                    after_comma = false;
                    continue;
                }

                after_comma = false;
                items.push(self.parse_expr_assoc_with(0));
                expecting_item = false;
                continue;
            }

            // expecting `,` or the closing delimiter
            if kind == TokenKind::Comma {
                let comma = self.bump();
                after_comma = true;
                expecting_item = true;

                if end_tokens.iter().any(|t| self.cur().kind == *t) {
                    let msg = match ctx {
                        SeqContext::CallArgList => {
                            "trailing comma in argument list is not supported"
                        }
                        SeqContext::ListLiteral => {
                            "trailing comma in list literal is not supported"
                        }
                    };
                    self.diagnostics.emit_with_labels(
                        DiagnosticCode::Parse(ParseDiagnostic::TrailingComma),
                        comma.span,
                        msg,
                        vec![Label {
                            span: comma.span,
                            message: Some("remove this comma".into()),
                        }],
                    );
                    break;
                }

                continue;
            }

            if self.cur().can_begin_expr() {
                // Likely a missing comma: `f(a b)` / `[1 2]`.
                let found = self.cur().clone();
                let close_expected = Self::expected_seq_end(end_tokens);
                let insertion = Span {
                    start: found.span.start,
                    end: found.span.start,
                };
                self.diagnostics.emit_with_labels(
                    DiagnosticCode::Parse(ParseDiagnostic::MissingComma),
                    found.span,
                    format!(
                        "expected ',' or {close_expected}, found {}",
                        Self::describe_token(&found.kind)
                    ),
                    vec![Label {
                        span: insertion,
                        message: Some("insert ','".into()),
                    }],
                );
                expecting_item = true;
                after_comma = false;
                continue;
            }

            // Unexpected token between items; skip forward to a plausible boundary.
            let found = self.cur().clone();
            let close_expected = Self::expected_seq_end(end_tokens);
            self.diagnostics.emit(
                DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                found.span,
                format!(
                    "expected ',' or {close_expected}, found {}",
                    Self::describe_token(&found.kind)
                ),
            );
            if found.kind != TokenKind::Eof {
                self.bump();
            }
            self.recover_seq_to_comma_or_tokens(end_tokens);
        }

        items
    }

    fn recover_seq_to_comma_or_tokens(&mut self, end_tokens: &[TokenKind]) {
        let _ = self.recover_scan(|kind, depth| {
            if depth.at_top_level()
                && (kind == TokenKind::Comma
                    || is_closing_delim(&kind)
                    || end_tokens.iter().any(|t| kind == *t))
            {
                RecoverScanDecision::Stop
            } else {
                RecoverScanDecision::Continue
            }
        });
    }

    fn expected_seq_end(end_tokens: &[TokenKind]) -> &'static str {
        match end_tokens.first() {
            Some(TokenKind::CloseParen) => "')'",
            Some(TokenKind::CloseBracket) => "']'",
            Some(TokenKind::Eof) | None => "end of input",
            _ => "<end>",
        }
    }

    fn expect_closing_delimiter(
        &mut self,
        close: TokenKind,
        expected: &'static str,
        open_span: Span,
        open_label: &'static str,
    ) -> bool {
        if self.cur().kind == close {
            self.bump();
            return true;
        }

        let found = self.cur().clone();
        let is_mismatched_closing = found.kind != TokenKind::Eof && is_closing_delim(&found.kind);
        let primary_span = if found.kind == TokenKind::Eof {
            Span {
                start: found.span.start,
                end: found.span.start,
            }
        } else {
            found.span
        };
        let insertion = Span {
            start: primary_span.start,
            end: primary_span.start,
        };

        let (code, labels) = if is_mismatched_closing {
            (
                DiagnosticCode::Parse(ParseDiagnostic::MismatchedDelimiter),
                vec![
                    Label {
                        span: open_span,
                        message: Some(open_label.into()),
                    },
                    Label {
                        span: found.span,
                        message: Some(format!(
                            "replace {} with {expected}",
                            Self::describe_token(&found.kind)
                        )),
                    },
                ],
            )
        } else {
            (
                DiagnosticCode::Parse(ParseDiagnostic::UnclosedDelimiter),
                vec![
                    Label {
                        span: open_span,
                        message: Some(open_label.into()),
                    },
                    Label {
                        span: insertion,
                        message: Some(format!("insert {expected}")),
                    },
                ],
            )
        };
        self.diagnostics.emit_with_labels(
            code,
            primary_span,
            format!(
                "expected {expected}, found {}",
                Self::describe_token(&found.kind)
            ),
            labels,
        );

        self.recover_to_close_delimiter(close)
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

    fn recover_scan(
        &mut self,
        mut decide: impl FnMut(TokenKind, DelimDepth) -> RecoverScanDecision,
    ) -> RecoverScanResult {
        let mut depth = DelimDepth::default();
        loop {
            let kind = self.cur().kind;
            if kind == TokenKind::Eof {
                return RecoverScanResult::HitEof;
            }

            match decide(kind, depth) {
                RecoverScanDecision::Continue => {}
                RecoverScanDecision::Stop => return RecoverScanResult::Stopped,
                RecoverScanDecision::ConsumeAndStop => {
                    let tok = self.bump();
                    depth.consume(tok.kind);
                    return RecoverScanResult::Consumed;
                }
            }

            let tok = self.bump();
            depth.consume(tok.kind);
        }
    }

    fn recover_to_expr_boundary(&mut self) {
        let _ = self.recover_scan(|kind, depth| {
            if depth.at_top_level()
                && (kind == TokenKind::Comma || kind == TokenKind::Colon || is_closing_delim(&kind))
            {
                RecoverScanDecision::Stop
            } else {
                RecoverScanDecision::Continue
            }
        });
    }

    fn recover_to_punct_or_expr_boundary(&mut self, punct: TokenKind) -> bool {
        matches!(
            self.recover_scan(|kind, depth| {
                if !depth.at_top_level() {
                    return RecoverScanDecision::Continue;
                }

                if kind == punct {
                    return RecoverScanDecision::ConsumeAndStop;
                }

                if kind == TokenKind::Comma || kind == TokenKind::Colon || is_closing_delim(&kind) {
                    return RecoverScanDecision::Stop;
                }

                RecoverScanDecision::Continue
            }),
            RecoverScanResult::Consumed
        )
    }

    fn recover_to_close_delimiter(&mut self, close: TokenKind) -> bool {
        matches!(
            self.recover_scan(|kind, depth| {
                if !depth.at_top_level() {
                    return RecoverScanDecision::Continue;
                }

                if kind == close {
                    return RecoverScanDecision::ConsumeAndStop;
                }

                if is_closing_delim(&kind) {
                    // A mismatched closing delimiter likely belongs to an outer context.
                    return RecoverScanDecision::Stop;
                }

                RecoverScanDecision::Continue
            }),
            RecoverScanResult::Consumed
        )
    }

    fn ternary_binding_power() -> (u8, u8) {
        // Lower precedence than `||` so `a || b ? c : d` parses as `(a || b) ? c : d`.
        // Right-associative: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.
        (0, 0)
    }

    fn recover_from_infix(&mut self, op_tok: Token) -> Expr {
        self.diagnostics.emit(
            DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
            op_tok.span,
            format!(
                "expected expression after {}",
                Self::describe_token(&op_tok.kind)
            ),
        );
        let err_span = if self.cur().kind == TokenKind::Eof {
            Span {
                start: op_tok.span.end,
                end: op_tok.span.end,
            }
        } else {
            self.cur().span
        };
        if !matches!(
            self.cur().kind,
            TokenKind::Comma
                | TokenKind::CloseBracket
                | TokenKind::CloseParen
                | TokenKind::Colon
                | TokenKind::Eof
        ) {
            self.bump();
            self.recover_to_expr_boundary();
        }
        self.error_expr_at(err_span)
    }
}

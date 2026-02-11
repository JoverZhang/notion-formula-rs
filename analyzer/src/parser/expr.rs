//! Expression parsing (Pratt parser).
//!
//! Produces an AST plus parse diagnostics. Spans are UTF-8 byte offsets with half-open semantics
//! `[start, end)`.

use super::{ParseOutput, Parser};
use crate::Token;
use crate::ast::{AssocOp, Expr, ExprKind, NotKind, UnOp};
use crate::diagnostics::{DiagnosticCode, Label, ParseDiagnostic};
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

        while let Some(op) = AssocOp::from_tok(self.cur()) {
            let (l_bp, r_bp) = op.infix_binding_power();

            if l_bp < min_bp {
                break;
            }

            match op {
                // Ternary expression: `cond(lhs) ? then : otherwise`
                AssocOp::Ternary => {
                    lhs = self.parse_ternary_suffix(lhs, r_bp);
                }

                // Binary expression: `lhs op rhs`
                AssocOp::Binary(op) => {
                    let op_tok = self.bump(); // operator token

                    let rhs = if self.cur().can_begin_expr() {
                        self.parse_expr_assoc_with(r_bp)
                    } else {
                        self.recover_from_infix(op_tok)
                    };

                    lhs = self.mk_expr(
                        lhs.span.to(rhs.span),
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
            TokenKind::Bang => self.parse_expr_unary(UnOp::Not(NotKind::Bang)),
            // `not expr`
            TokenKind::Not => self.parse_expr_unary(UnOp::Not(NotKind::Keyword)),
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
    /// Trailing comma (`[1, 2,]`) is rejected with a dedicated diagnostic.
    ///
    /// After a comma, the parser expects an expression and emits a diagnostic when it's missing.
    ///
    /// ```text
    /// "[1,2,]" -> "trailing comma is not supported"
    /// ```
    fn parse_list_literal(&mut self) -> Expr {
        let (lbrack, items) = self.parse_delimited(Delimiter::Bracket, |p| {
            p.parse_seq_to_before_tokens(&[TokenKind::CloseBracket], TokenKind::Comma)
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
        let open = self.bump(); // TODO: check if the open token is the expected one
        let value = parse_contents(self);
        self.expect_closing_delimiter(
            delim.close(),
            delim.close_expected(),
            open.span,
            delim.unclosed_label(),
        );
        (open, value)
    }

    fn parse_expr_dot_or_call(&mut self, mut expr: Expr) -> Expr {
        loop {
            let e = match self.cur().kind {
                // Call expression: `expr(arg1, ...)`
                TokenKind::OpenParen => self.parse_expr_fn_call(expr),
                // Member call expression: `expr.method(arg1, ...)`
                TokenKind::Dot => self.parse_expr_member_call(expr),
                _ => break,
            };

            match e {
                // Continue parsing for chaining: `expr(arg1, ...).method(arg1, ...)`
                Ok(e) => expr = e,
                Err(e) => {
                    expr = e;
                    break;
                }
            }
        }

        expr
    }

    fn parse_expr_fn_call(&mut self, expr: Expr) -> Result<Expr, Expr> {
        let lparen = self.cur(); // '('
        let (args, end) = self.parse_paren_arg_list();

        let span = Span {
            start: expr.span.start,
            end,
        };
        // Only Ident can call
        match expr.kind {
            ExprKind::Ident(sym) => {
                let e = self.mk_expr(span, ExprKind::Call { callee: sym, args });
                Ok(e)
            }
            _ => {
                self.diagnostics.emit(
                    DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                    lparen.span,
                    "expected call callee (identifier)",
                );
                Err(self.error_expr_at(span))
            }
        }
    }

    fn parse_expr_member_call(&mut self, receiver: Expr) -> Result<Expr, Expr> {
        self.bump(); // '.'

        let method_tok = loop {
            match self.cur().kind {
                TokenKind::Ident(..) => break self.bump(),
                // `a..if(...)`: recover by skipping extra dots.
                TokenKind::Dot => {
                    let extra = self.bump();
                    self.diagnostics.emit(
                        DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                        extra.span,
                        "unexpected '.' in member call",
                    );
                }
                _ => {
                    let span = Span {
                        start: receiver.span.start,
                        end: self.last_bumped_end(),
                    };
                    return Err(self.error_expr_at(span));
                }
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
                start: receiver.span.start,
                end: self.last_bumped_end(),
            };
            return Err(self.error_expr_at(span));
        }

        let (args, end) = self.parse_paren_arg_list();

        Ok(self.mk_expr(
            Span {
                start: receiver.span.start,
                end,
            },
            ExprKind::MemberCall {
                receiver: Box::new(receiver),
                method,
                args,
            },
        ))
    }

    fn parse_paren_arg_list(&mut self) -> (Vec<Expr>, u32) {
        let (_lparen, args) = self.parse_delimited(Delimiter::Paren, |p| {
            p.parse_seq_to_before_tokens(&[TokenKind::CloseParen], TokenKind::Comma)
        });
        (args, self.last_bumped_end())
    }

    /// Parses the suffix of a ternary expression: `? then : otherwise`.
    fn parse_ternary_suffix(&mut self, lhs: Expr, else_min_bp: u8) -> Expr {
        let q_tok = self.bump(); // '?'

        let then_expr = if self.cur().can_begin_expr() {
            self.parse_expr_assoc_with(0)
        } else {
            // `cond ? : else` / `cond ?` at end-of-input
            let insertion = q_tok.span.end;
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
                return self.mk_expr(
                    lhs.span.to(else_expr.span),
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

        self.mk_expr(
            lhs.span.to(else_expr.span),
            ExprKind::Ternary {
                cond: Box::new(lhs),
                then: Box::new(then_expr),
                otherwise: Box::new(else_expr),
            },
        )
    }

    fn parse_seq_to_before_tokens(
        &mut self,
        closes_expected: &[TokenKind],
        sep: TokenKind,
    ) -> Vec<Expr> {
        debug_assert!(
            !closes_expected.is_empty(),
            "seq parsing needs at least one close token"
        );

        let mut items = Vec::new();
        let mut expecting_item = true;
        let mut after_sep = false;

        let sep_expected = sep
            .to_str()
            .map(|s| format!("'{s}'"))
            .unwrap_or_else(|| "<token>".to_string());
        let close_expected = Self::expected_closes(closes_expected);

        loop {
            let kind = self.cur().kind;

            if kind == TokenKind::Eof
                || is_closing_delim(&kind)
                || Self::is_close(closes_expected, &kind)
            {
                break;
            }

            if expecting_item {
                if kind == sep {
                    // `f(,a)` / `[1,,2]`: missing item before the separator.
                    let sep_tok = self.bump();
                    self.diagnostics.emit(
                        DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                        sep_tok.span,
                        format!("expected expression before {}", Self::describe_token(&sep)),
                    );
                    items.push(self.error_expr_at(sep_tok.span));
                    after_sep = true;
                    continue;
                }

                if self.cur().can_begin_expr() {
                    items.push(self.parse_expr_assoc_with(0));
                    expecting_item = false;
                    after_sep = false;
                    continue;
                }

                // Missing / malformed item.
                let found = self.cur().clone();
                let msg = if after_sep {
                    format!(
                        "expected expression after {}, found {}",
                        Self::describe_token(&sep),
                        Self::describe_token(&found.kind)
                    )
                } else {
                    format!(
                        "expected expression, found {}",
                        Self::describe_token(&found.kind)
                    )
                };
                self.diagnostics.emit(
                    DiagnosticCode::Parse(ParseDiagnostic::MissingExpr),
                    found.span,
                    msg,
                );
                if found.kind != TokenKind::Eof {
                    self.bump();
                }
                self.recover_seq_to_sep_or_closes(closes_expected, &sep);
                items.push(self.error_expr_at(found.span));
                expecting_item = false;
                after_sep = false;
                continue;
            }

            // Expecting `sep` or the closing delimiter.
            if kind == sep {
                let sep_tok = self.bump();
                expecting_item = true;
                after_sep = true;

                // Trailing separator: `f(1,)` / `[1,2,]`.
                let next = self.cur().kind;
                if Self::is_close(closes_expected, &next) {
                    let actions = self
                        .quick_fix_action("Remove trailing comma", sep_tok.span, "")
                        .into_iter()
                        .collect();
                    self.diagnostics.emit_with_labels_and_actions(
                        DiagnosticCode::Parse(ParseDiagnostic::TrailingComma),
                        sep_tok.span,
                        "trailing comma is not supported",
                        vec![Label {
                            span: sep_tok.span,
                            message: Some("remove this comma".into()),
                        }],
                        actions,
                    );
                    break;
                }

                // If we hit EOF or a mismatched closing delimiter after the separator,
                // prefer the delimiter diagnostic (and avoid cascading sequence noise).
                if next == TokenKind::Eof || is_closing_delim(&next) {
                    break;
                }

                continue;
            }

            if self.cur().can_begin_expr() {
                // Likely a missing separator: `f(a b)` / `[1 2]`.
                let found = self.cur().clone();
                let insertion = Span {
                    start: found.span.start,
                    end: found.span.start,
                };
                let actions = self
                    .quick_fix_action("Insert `,`", insertion, ",")
                    .into_iter()
                    .collect();
                self.diagnostics.emit_with_labels_and_actions(
                    DiagnosticCode::Parse(ParseDiagnostic::MissingComma),
                    found.span,
                    format!(
                        "expected {sep_expected} or {close_expected}, found {}",
                        Self::describe_token(&found.kind)
                    ),
                    vec![Label {
                        span: insertion,
                        message: Some(format!("insert {sep_expected}")),
                    }],
                    actions,
                );
                expecting_item = true;
                after_sep = false;
                continue;
            }

            // Unexpected token between items; skip forward to a plausible boundary.
            let found = self.cur().clone();
            self.diagnostics.emit(
                DiagnosticCode::Parse(ParseDiagnostic::UnexpectedToken),
                found.span,
                format!(
                    "expected {sep_expected} or {close_expected}, found {}",
                    Self::describe_token(&found.kind)
                ),
            );
            if found.kind != TokenKind::Eof {
                self.bump();
            }
            self.recover_seq_to_sep_or_closes(closes_expected, &sep);
        }

        items
    }

    fn recover_seq_to_sep_or_closes(&mut self, closes_expected: &[TokenKind], sep: &TokenKind) {
        let _ = self.recover_scan(|kind, depth| {
            if depth.at_top_level()
                && (kind == *sep
                    || is_closing_delim(&kind)
                    || Self::is_close(closes_expected, &kind))
            {
                RecoverScanDecision::Stop
            } else {
                RecoverScanDecision::Continue
            }
        });
    }

    fn is_close(closes_expected: &[TokenKind], kind: &TokenKind) -> bool {
        closes_expected.iter().any(|p| kind == p)
    }

    fn expected_closes(closes_expected: &[TokenKind]) -> String {
        let mut out = String::new();
        for (i, kind) in closes_expected.iter().enumerate() {
            if i > 0 {
                out.push_str(" or ");
            }
            let expected = kind
                .to_str()
                .map(|s| format!("'{s}'"))
                .unwrap_or_else(|| "<token>".to_string());
            out.push_str(&expected);
        }
        out
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

        let found = self.cur();
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

        let (code, labels, actions) = if is_mismatched_closing {
            let replacement = close.to_str().unwrap_or_default().to_string();
            let actions = self
                .quick_fix_action(
                    format!(
                        "Replace {} with `{}`",
                        Self::describe_token(&found.kind),
                        close.to_str().unwrap_or_default()
                    ),
                    found.span,
                    replacement,
                )
                .into_iter()
                .collect::<Vec<_>>();
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
                actions,
            )
        } else {
            let replacement = close.to_str().unwrap_or_default().to_string();
            let actions = self
                .quick_fix_action(
                    format!("Insert `{}`", close.to_str().unwrap_or_default()),
                    insertion,
                    replacement,
                )
                .into_iter()
                .collect::<Vec<_>>();
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
                actions,
            )
        };
        self.diagnostics.emit_with_labels_and_actions(
            code,
            primary_span,
            format!(
                "expected {expected}, found {}",
                Self::describe_token(&found.kind)
            ),
            labels,
            actions,
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

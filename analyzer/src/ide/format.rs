//! Pretty-prints an `Expr` back to source text.
//! Spans are UTF-8 byte offsets and use half-open ranges `[start, end)`.
//! Uses `TokenQuery` for safe trivia/comment attachment.

use std::collections::HashSet;

use crate::ast::{BinOp, BinOpKind, Expr, ExprKind, UnOp};
use crate::lexer::{CommentKind, Lit, LitKind, Span, Token, TokenKind, TokenRange};
use crate::parser::TokenQuery;
use crate::source_map::SourceMap;

const INDENT: usize = 2;
const MAX_WIDTH: usize = 80;

fn source_has_newline(span: Span, source: &str) -> bool {
    let len = source.len();
    let start = span.start as usize;
    let end = span.end as usize;

    if start >= end || start >= len {
        return false;
    }

    let end = end.min(len);
    source.get(start..end).is_some_and(|s| {
        let trimmed = if s.ends_with('\n') {
            &s[..s.len().saturating_sub(1)]
        } else {
            s
        };
        trimmed.contains('\n')
    })
}

/// Formats an expression using the original `source` and its lexed `tokens`.
///
/// `tokens` must come from lexing the same `source`, or comment placement may be wrong.
pub struct Formatter<'a> {
    source: &'a str,
    tokens: &'a [Token],
    token_query: TokenQuery<'a>,
    used_comments: HashSet<usize>,
    sm: SourceMap<'a>,
}

#[derive(Debug, Clone)]
struct Line {
    indent: usize,
    text: String,
}

#[derive(Debug, Clone, Default)]
struct Rendered {
    lines: Vec<Line>,
}

impl Rendered {
    fn single(indent: usize, text: impl Into<String>) -> Self {
        Self {
            lines: vec![Line {
                indent,
                text: text.into(),
            }],
        }
    }

    fn push_line(&mut self, indent: usize, text: impl Into<String>) {
        self.lines.push(Line {
            indent,
            text: text.into(),
        });
    }

    fn append(&mut self, mut other: Rendered) {
        self.lines.append(&mut other.lines);
    }

    fn append_trailing(&mut self, text: &str) {
        if let Some(last) = self.lines.last_mut() {
            if !last.text.is_empty() {
                last.text.push(' ');
            }
            last.text.push_str(text);
        } else {
            self.lines.push(Line {
                indent: 0,
                text: text.to_string(),
            });
        }
    }

    fn render(self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.into_iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            for _ in 0..(line.indent * INDENT) {
                out.push(' ');
            }
            out.push_str(&line.text);
        }
        out
    }
}

/// Formats `expr` into a stable string, ending with a single trailing `\n`.
///
/// `source` and `tokens` must describe the same original text.
pub fn format_expr(expr: &Expr, source: &str, tokens: &[Token]) -> String {
    let mut fmt = Formatter::new(source, tokens);
    let mut s = fmt.format_expr_rendered(expr, 0).render();
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

impl<'a> Formatter<'a> {
    /// Creates a formatter for `source` and its `tokens`.
    pub fn new(source: &'a str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            token_query: TokenQuery::new(tokens),
            used_comments: HashSet::new(),
            sm: SourceMap::new(source),
        }
    }

    fn token_query(&self) -> &TokenQuery<'a> {
        &self.token_query
    }

    /// Run an inline-layout attempt, rolling back comment attachment on failure.
    fn try_inline<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let saved = self.used_comments.clone();
        let out = f(self);
        if out.is_none() {
            self.used_comments = saved;
        }
        out
    }

    fn format_delimited_seq(
        &mut self,
        mut out: Rendered,
        indent: usize,
        open: String,
        open_appends_to_last: bool,
        close: &str,
        items: &[Expr],
    ) -> Rendered {
        // Member calls keep `).method(` chained on the receiver's last line.
        if open_appends_to_last {
            if let Some(last) = out.lines.last_mut() {
                last.text.push_str(&open);
            } else {
                out.push_line(indent, open);
            }
        } else {
            out.push_line(indent, open);
        }

        for (idx, item) in items.iter().enumerate() {
            let mut item_r = self.format_expr_rendered(item, indent + 1);
            let is_last = idx + 1 == items.len();
            if !is_last && let Some(last) = item_r.lines.last_mut() {
                last.text.push(',');
            }
            out.append(item_r);
        }

        out.push_line(indent, close);
        out
    }

    fn format_expr_rendered(&mut self, expr: &Expr, indent: usize) -> Rendered {
        let mut out = Rendered::default();

        let (leading_comments, inline_block_comment) = self.take_leading_comments(expr);

        for idx in leading_comments {
            out.push_line(indent, self.render_comment(idx));
        }

        let mut body = self.format_expr_kind(expr, indent);

        if let Some(idx) = inline_block_comment {
            let prefix = format!("{} ", self.render_comment(idx));
            if let Some(first) = body.lines.first_mut() {
                first.text = format!("{}{}", prefix, first.text);
            } else {
                body.push_line(indent, prefix.trim_end().to_string());
            }
        }

        if let Some(idx) = self.take_trailing_comment(expr) {
            body.append_trailing(&self.render_comment(idx));
        }

        out.append(body);
        out
    }

    fn format_expr_kind(&mut self, expr: &Expr, indent: usize) -> Rendered {
        match &expr.kind {
            ExprKind::Ident(sym) => Rendered::single(indent, sym.text.clone()),
            ExprKind::Group { inner } => self.format_group(expr, indent, inner),
            ExprKind::List { items } => self.format_list(expr, indent, items),
            ExprKind::Lit(lit) => Rendered::single(indent, render_literal(lit)),
            ExprKind::Call { callee, args } => self.format_call(expr, indent, &callee.text, args),
            ExprKind::MemberCall {
                receiver,
                method,
                args,
            } => self.format_member_call(expr, indent, receiver, &method.text, args),
            ExprKind::Unary { op, expr: inner } => self.format_unary(expr, indent, *op, inner),
            ExprKind::Binary { op, left, right } => {
                self.format_binary(expr, indent, *op, left, right)
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => self.format_ternary(expr, indent, cond, then, otherwise),
            ExprKind::Error => Rendered::single(indent, "<error>"),
        }
    }

    fn format_group(&mut self, expr: &Expr, indent: usize, inner: &Expr) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let inline = this.format_expr_single_line(inner, indent)?;
                let text = format!("({inline})");
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        let mut out = Rendered::default();
        out.push_line(indent, "(");
        let inner_rendered = self.format_expr_rendered(inner, indent + 1);
        out.append(inner_rendered);
        out.push_line(indent, ")");
        out
    }

    fn format_list(&mut self, expr: &Expr, indent: usize, items: &[Expr]) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let mut parts = Vec::new();
                for item in items {
                    parts.push(this.format_expr_single_line(item, indent)?);
                }
                let text = format!("[{}]", parts.join(", "));
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        self.format_delimited_seq(
            Rendered::default(),
            indent,
            "[".to_string(),
            false,
            "]",
            items,
        )
    }

    fn format_unary(&mut self, expr: &Expr, indent: usize, op: UnOp, inner: &Expr) -> Rendered {
        let op_str = op.as_str();
        let needs_space = matches!(op, UnOp::Not(crate::ast::NotKind::Keyword));

        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let inline = this.format_expr_single_line(inner, indent)?;
                let text = if needs_space {
                    format!("{op_str} {inline}")
                } else {
                    format!("{op_str}{inline}")
                };
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        let mut out = Rendered::default();
        let lparen = if needs_space {
            format!("{op_str} (")
        } else {
            format!("{op_str}(")
        };
        out.push_line(indent, lparen);
        let inner_rendered = self.format_expr_rendered(inner, indent + 1);
        out.append(inner_rendered);
        out.push_line(indent, ")");
        out
    }

    fn format_binary(
        &mut self,
        expr: &Expr,
        indent: usize,
        op: BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Rendered {
        let op_str = binop_str(op.node);
        let has_newline = self.expr_has_newline(expr);
        let trailing_line_comment = self
            .available_trailing_comment(expr)
            .and_then(|idx| self.tokens.get(idx))
            .map(|tok| matches!(tok.kind, TokenKind::DocComment(CommentKind::Line, _)))
            .unwrap_or(false);

        if (!has_newline || trailing_line_comment)
            && let Some(out) = self.try_inline(|this| {
                let lhs = this.format_expr_single_line(left, indent)?;
                let rhs = this.format_expr_single_line(right, indent)?;
                let text = format!("{lhs} {op_str} {rhs}");
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        let mut out = Rendered::default();
        let left_rendered = self.format_expr_rendered(left, indent);
        let mut right_rendered = self.format_expr_rendered(right, indent + 1);

        if left_rendered.lines.len() == 1
            && !left_rendered
                .lines
                .last()
                .map(|l| l.text.contains("//"))
                .unwrap_or(false)
            && !right_rendered
                .lines
                .first()
                .map(|l| l.text.trim_start().starts_with("//"))
                .unwrap_or(false)
        {
            if let Some(first) = right_rendered.lines.first_mut() {
                first.text = format!("{} {} {}", left_rendered.lines[0].text, op_str, first.text);
                first.indent = left_rendered.lines[0].indent;
            } else {
                right_rendered.push_line(
                    indent + 1,
                    format!("{} {}", left_rendered.lines[0].text, op_str),
                );
            }
            for line in right_rendered.lines.iter_mut().skip(1) {
                line.indent = line.indent.saturating_sub(1);
            }
            out.append(right_rendered);
        } else {
            if let Some(first) = right_rendered.lines.first_mut() {
                first.text = format!("{} {}", op_str, first.text);
            } else {
                right_rendered.push_line(indent + 1, op_str.to_string());
            }

            out.append(left_rendered);
            out.append(right_rendered);
        }
        out
    }

    fn format_ternary(
        &mut self,
        expr: &Expr,
        indent: usize,
        cond: &Expr,
        then: &Expr,
        otherwise: &Expr,
    ) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let c = this.format_expr_single_line(cond, indent)?;
                let t = this.format_expr_single_line(then, indent)?;
                let o = this.format_expr_single_line(otherwise, indent)?;
                let text = format!("{c} ? {t} : {o}");
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        let mut out = Rendered::default();
        let cond_r = self.format_expr_rendered(cond, indent);
        let mut then_r = self.format_expr_rendered(then, indent + 1);
        let mut else_r = self.format_expr_rendered(otherwise, indent + 1);

        if let Some(first) = then_r.lines.first_mut() {
            first.text = format!("? {}", first.text);
        } else {
            then_r.push_line(indent + 1, "?".to_string());
        }

        if let Some(first) = else_r.lines.first_mut() {
            first.text = format!(": {}", first.text);
        } else {
            else_r.push_line(indent + 1, ":".to_string());
        }

        out.append(cond_r);
        out.append(then_r);
        out.append(else_r);

        out
    }

    fn format_call(&mut self, expr: &Expr, indent: usize, callee: &str, args: &[Expr]) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let mut parts = Vec::new();
                for arg in args {
                    parts.push(this.format_expr_single_line(arg, indent)?);
                }
                let text = format!("{callee}({})", parts.join(", "));
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        self.format_delimited_seq(
            Rendered::default(),
            indent,
            format!("{callee}("),
            false,
            ")",
            args,
        )
    }

    fn format_member_call(
        &mut self,
        expr: &Expr,
        indent: usize,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline
            && let Some(out) = self.try_inline(|this| {
                let receiver_inline = this.format_expr_single_line(receiver, indent)?;
                let mut parts = Vec::new();
                for arg in args {
                    parts.push(this.format_expr_single_line(arg, indent)?);
                }
                let text = format!("{receiver_inline}.{method}({})", parts.join(", "));
                this.fits_on_line(indent, text.len())
                    .then_some(Rendered::single(indent, text))
            })
        {
            return out;
        }

        let receiver_r = self.format_expr_rendered(receiver, indent);
        self.format_delimited_seq(receiver_r, indent, format!(".{method}("), true, ")", args)
    }

    fn format_expr_single_line(&mut self, expr: &Expr, indent: usize) -> Option<String> {
        let rendered = self.format_expr_rendered(expr, indent);
        if rendered.lines.len() == 1 {
            Some(rendered.lines[0].text.clone())
        } else {
            None
        }
    }

    fn fits_on_line(&self, indent: usize, text_len: usize) -> bool {
        indent * INDENT + text_len <= MAX_WIDTH
    }

    /// Returns a token index range for an expression using its `Span`.
    ///
    /// This is used by comment attachment logic. The range is intersection-based and therefore
    /// may include trivia tokens that lie within the expression span.
    fn expr_token_range(&self, expr: &Expr) -> TokenRange {
        self.token_query().range_for_span(expr.span)
    }

    fn expr_span_from_tokens(&self, expr: &Expr) -> Option<Span> {
        match &expr.kind {
            ExprKind::Binary { left, right, .. } => {
                let start = self.expr_span_from_tokens(left)?.start;
                let end = self.expr_span_from_tokens(right)?.end;
                return Some(Span { start, end });
            }
            ExprKind::Ternary {
                cond, otherwise, ..
            } => {
                let start = self.expr_span_from_tokens(cond)?.start;
                let end = self.expr_span_from_tokens(otherwise)?.end;
                return Some(Span { start, end });
            }
            ExprKind::MemberCall { receiver, args, .. } => {
                let start = self.expr_span_from_tokens(receiver)?.start;
                let end = args
                    .iter()
                    .rev()
                    .find_map(|a| self.expr_span_from_tokens(a).map(|s| s.end))
                    .unwrap_or(expr.span.end);
                return Some(Span { start, end });
            }
            ExprKind::Group { inner } => {
                let inner_span = self.expr_span_from_tokens(inner)?;
                return Some(Span {
                    start: expr.span.start.min(inner_span.start),
                    end: expr.span.end.max(inner_span.end),
                });
            }
            ExprKind::Unary { expr: inner, .. } => {
                let inner_span = self.expr_span_from_tokens(inner)?;
                return Some(Span {
                    start: expr.span.start.min(inner_span.start),
                    end: inner_span.end,
                });
            }
            ExprKind::Call { args, .. } if !args.is_empty() => {
                let start = expr.span.start;
                let end_span = self.expr_span_from_tokens(args.last().unwrap())?;
                return Some(Span {
                    start,
                    end: end_span.end,
                });
            }
            _ => {}
        }

        let range = self.expr_token_range(expr);
        let q = self.token_query();
        let start_idx = q.first_in_range(range)?;
        let end_idx = q.last_in_range(range)?;
        Some(Span {
            start: self.tokens[start_idx].span.start,
            end: self.tokens[end_idx].span.end,
        })
    }

    fn expr_has_newline(&self, expr: &Expr) -> bool {
        let span = self.expr_span_from_tokens(expr).unwrap_or(expr.span);
        source_has_newline(span, self.source)
    }

    fn available_leading_comments(&self, expr: &Expr) -> Vec<usize> {
        let q = self.token_query();
        let range = self.expr_token_range(expr);
        let (start, _) = q.bounds_usize(range);
        q.leading_trivia_before(start)
            .filter(|&i| self.tokens[i].kind.is_comment())
            .filter(|i| !self.used_comments.contains(i))
            .collect()
    }

    fn take_leading_comments(&mut self, expr: &Expr) -> (Vec<usize>, Option<usize>) {
        let comments = self.available_leading_comments(expr);
        let expr_line = self.sm.line_col(expr.span.start).0;
        let mut inline_block = None;
        let mut leading = Vec::new();

        for idx in comments {
            let tok = &self.tokens[idx];
            let is_inline_block = matches!(tok.kind, TokenKind::DocComment(CommentKind::Block, _))
                && self.sm.line_col(tok.span.end.saturating_sub(1)).0 == expr_line
                && !self.slice_has_newline(tok.span.end, expr.span.start);

            if is_inline_block {
                inline_block = Some(idx);
            } else {
                leading.push(idx);
            }

            self.used_comments.insert(idx);
        }

        (leading, inline_block)
    }

    fn available_trailing_comment(&self, expr: &Expr) -> Option<usize> {
        let q = self.token_query();
        let range = self.expr_token_range(expr);
        let (_, hi) = q.bounds_usize(range);
        let last_tok_idx = q.last_in_range(range)?;
        let last_line = self
            .sm
            .line_col(self.tokens[last_tok_idx].span.end.saturating_sub(1))
            .0;

        for idx in q.trailing_trivia_until_newline_or_nontrivia(hi) {
            let tok = &self.tokens[idx];
            match &tok.kind {
                TokenKind::DocComment(CommentKind::Line, _) => {
                    if self.used_comments.contains(&idx) {
                        return None;
                    }
                    let line = self.sm.line_col(tok.span.start).0;
                    if line == last_line {
                        return Some(idx);
                    } else {
                        break;
                    }
                }
                TokenKind::DocComment(CommentKind::Block, _) => {
                    if self.used_comments.contains(&idx) {
                        return None;
                    }
                    let span_range = tok.span.start as usize..tok.span.end as usize;
                    let has_newline = self
                        .source
                        .get(span_range)
                        .map(|s| s.contains('\n'))
                        .unwrap_or(true);
                    if has_newline {
                        break;
                    }
                    let line = self.sm.line_col(tok.span.start).0;
                    if line == last_line {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn take_trailing_comment(&mut self, expr: &Expr) -> Option<usize> {
        let idx = self.available_trailing_comment(expr)?;
        self.used_comments.insert(idx);
        Some(idx)
    }

    fn render_comment(&self, idx: usize) -> String {
        match &self.tokens[idx].kind {
            TokenKind::DocComment(kind, sym) => match kind {
                CommentKind::Line => format!("//{}", sym.text),
                CommentKind::Block => format!("/*{}*/", sym.text),
            },
            _ => String::new(),
        }
    }

    fn slice_has_newline(&self, start: u32, end: u32) -> bool {
        let s = start as usize;
        let e = end as usize;
        if s >= e || s >= self.source.len() {
            return false;
        }
        let e = e.min(self.source.len());
        self.source
            .get(s..e)
            .map(|s| s.contains('\n'))
            .unwrap_or(false)
    }
}

fn binop_str(op: BinOpKind) -> &'static str {
    use BinOpKind::*;
    match op {
        Lt => "<",
        Le => "<=",
        EqEq => "==",
        Ne => "!=",
        Ge => ">=",
        Gt => ">",
        AndAnd => "&&",
        OrOr => "||",
        Plus => "+",
        Minus => "-",
        Star => "*",
        Slash => "/",
        Percent => "%",
        Caret => "^",
    }
}

fn render_literal(lit: &Lit) -> String {
    match lit.kind {
        LitKind::Number | LitKind::Bool => lit.symbol.text.clone(),
        LitKind::String => escape_string(&lit.symbol.text),
    }
}

fn escape_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

use std::collections::HashSet;

use crate::ast::{BinOpKind, Expr, ExprKind, UnOpKind};
use crate::parser::{infix_binding_power, prefix_binding_power};
use crate::source_map::SourceMap;
use crate::token::{CommentKind, Lit, LitKind, Span, Token, TokenKind};

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
    source.get(start..end).map_or(false, |s| {
        let trimmed = if s.ends_with('\n') {
            &s[..s.len().saturating_sub(1)]
        } else {
            s
        };
        trimmed.contains('\n')
    })
}

pub struct Formatter<'a> {
    source: &'a str,
    tokens: &'a [Token],
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

fn format_expr_one_line(expr: &Expr) -> String {
    format_expr_one_line_with_prec(expr, 0)
}

fn format_expr_one_line_with_prec(expr: &Expr, parent_prec: u8) -> String {
    match &expr.kind {
        ExprKind::Ident(sym) => sym.text.clone(),
        ExprKind::Group { inner } => {
            let inner = format_expr_one_line_with_prec(inner, 0);
            format!("({})", inner)
        }
        ExprKind::Lit(lit) => render_literal(lit),

        ExprKind::Call { callee, args } => {
            let mut s = String::new();
            s.push_str(&callee.text);
            s.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format_expr_one_line_with_prec(a, 0));
            }
            s.push(')');
            s
        }

        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            let mut s = String::new();
            s.push_str(&format_expr_one_line_with_prec(receiver, 0));
            s.push('.');
            s.push_str(&method.text);
            s.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&format_expr_one_line_with_prec(a, 0));
            }
            s.push(')');
            s
        }

        ExprKind::Unary { op, expr: inner } => {
            let op_str = unop_str(op.node);
            let inner = format_expr_one_line_with_prec(inner, prefix_binding_power(op.node));
            format!("{}{}", op_str, inner)
        }

        ExprKind::Binary { op, left, right } => {
            let (l_bp, r_bp) = infix_binding_power(op.node);
            let this_prec = l_bp;

            let l = format_expr_one_line_with_prec(left, l_bp);
            let r = format_expr_one_line_with_prec(right, r_bp);

            let op_str = binop_str(op.node);
            let combined = format!("{} {} {}", l, op_str, r);

            if this_prec < parent_prec {
                format!("({})", combined)
            } else {
                combined
            }
        }

        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            let cond = format_expr_one_line_with_prec(cond, 0);
            let then = format_expr_one_line_with_prec(then, 0);
            let otherwise = format_expr_one_line_with_prec(otherwise, 0);
            format!("{} ? {} : {}", cond, then, otherwise)
        }

        ExprKind::Error => "<error>".into(),
    }
}

pub fn format_expr(expr: &Expr, source: &str, tokens: &[Token]) -> String {
    let mut fmt = Formatter::new(source, tokens);
    let one_line = format_expr_one_line(expr);
    let has_newline = fmt.expr_has_newline(expr);
    let force_multiline = fmt.forces_multiline(expr) || has_newline;

    let out = if !force_multiline && !one_line.contains('\n') && fmt.fits_on_line(0, one_line.len())
    {
        let mut s = one_line;
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s
    } else {
        let mut s = fmt.format_expr_rendered(expr, 0, 0).render();
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s
    };

    out
}

impl<'a> Formatter<'a> {
    pub fn new(source: &'a str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            used_comments: HashSet::new(),
            sm: SourceMap::new(source),
        }
    }

    fn format_expr_rendered(&mut self, expr: &Expr, indent: usize, parent_prec: u8) -> Rendered {
        let mut out = Rendered::default();

        let (leading_comments, inline_block_comment) = self.take_leading_comments(expr);

        for idx in leading_comments {
            out.push_line(indent, self.render_comment(idx));
        }

        let mut body = self.format_expr_kind(expr, indent, parent_prec);

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

    fn format_expr_kind(&mut self, expr: &Expr, indent: usize, parent_prec: u8) -> Rendered {
        match &expr.kind {
            ExprKind::Ident(sym) => Rendered::single(indent, sym.text.clone()),
            ExprKind::Group { inner } => self.format_group(expr, indent, parent_prec, inner),
            ExprKind::Lit(lit) => Rendered::single(indent, render_literal(lit)),
            ExprKind::Call { callee, args } => {
                self.format_call(expr, indent, parent_prec, &callee.text, args)
            }
            ExprKind::MemberCall {
                receiver,
                method,
                args,
            } => self.format_member_call(expr, indent, parent_prec, receiver, &method.text, args),
            ExprKind::Unary { op, expr: inner } => {
                self.format_unary(expr, indent, parent_prec, op.node, inner)
            }
            ExprKind::Binary { op, left, right } => {
                self.format_binary(expr, indent, parent_prec, op.node, left, right)
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => self.format_ternary(expr, indent, parent_prec, cond, then, otherwise),
            ExprKind::Error => Rendered::single(indent, "<error>"),
        }
    }

    fn format_group(
        &mut self,
        expr: &Expr,
        indent: usize,
        _parent_prec: u8,
        inner: &Expr,
    ) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline {
            let saved = self.used_comments.clone();
            if let Some(inline) = self.format_expr_single_line(inner, indent, 0) {
                let text = format!("({})", inline);
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
            self.used_comments = saved;
        }

        let mut out = Rendered::default();
        out.push_line(indent, "(");
        let inner_rendered = self.format_expr_rendered(inner, indent + 1, 0);
        out.append(inner_rendered);
        out.push_line(indent, ")");
        out
    }

    fn format_unary(
        &mut self,
        expr: &Expr,
        indent: usize,
        _parent_prec: u8,
        op: UnOpKind,
        inner: &Expr,
    ) -> Rendered {
        let bp = prefix_binding_power(op);
        let op_str = unop_str(op);

        let has_newline = self.expr_has_newline(expr);

        if !has_newline {
            let saved = self.used_comments.clone();
            if let Some(inline) = self.format_expr_single_line(inner, indent, bp) {
                let mut text = String::new();
                text.push_str(op_str);
                text.push_str(&inline);
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
            self.used_comments = saved;
        }

        let mut out = Rendered::default();
        out.push_line(indent, format!("{}(", op_str));
        let inner_rendered = self.format_expr_rendered(inner, indent + 1, bp);
        out.append(inner_rendered);
        out.push_line(indent, ")");
        out
    }

    fn format_binary(
        &mut self,
        expr: &Expr,
        indent: usize,
        _parent_prec: u8,
        op: BinOpKind,
        left: &Expr,
        right: &Expr,
    ) -> Rendered {
        let (l_bp, r_bp) = infix_binding_power(op);
        let op_str = binop_str(op);
        let has_newline = self.expr_has_newline(expr);
        let trailing_line_comment = self
            .available_trailing_comment(expr)
            .and_then(|idx| self.tokens.get(idx))
            .map(|tok| matches!(tok.kind, TokenKind::LineComment(_)))
            .unwrap_or(false);

        if !has_newline || trailing_line_comment {
            let saved = self.used_comments.clone();
            if let Some(lhs) = self.format_expr_single_line(left, indent, l_bp) {
                if let Some(rhs) = self.format_expr_single_line(right, indent, r_bp) {
                    let text = format!("{} {} {}", lhs, op_str, rhs);
                    if self.fits_on_line(indent, text.len()) {
                        return Rendered::single(indent, text);
                    }
                }
            }
            self.used_comments = saved;
        }

        let mut out = Rendered::default();
        let left_rendered = self.format_expr_rendered(left, indent, l_bp);
        let mut right_rendered = self.format_expr_rendered(right, indent + 1, r_bp);

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
        _parent_prec: u8,
        cond: &Expr,
        then: &Expr,
        otherwise: &Expr,
    ) -> Rendered {
        let ternary_prec = 2;
        let has_newline = self.expr_has_newline(expr);

        if !has_newline {
            let saved = self.used_comments.clone();
            if let (Some(c), Some(t), Some(o)) = (
                self.format_expr_single_line(cond, indent, ternary_prec),
                self.format_expr_single_line(then, indent, ternary_prec),
                self.format_expr_single_line(otherwise, indent, ternary_prec),
            ) {
                let text = format!("{} ? {} : {}", c, t, o);
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
            self.used_comments = saved;
        }

        let mut out = Rendered::default();
        let cond_r = self.format_expr_rendered(cond, indent, 0);
        let mut then_r = self.format_expr_rendered(then, indent + 1, 0);
        let mut else_r = self.format_expr_rendered(otherwise, indent + 1, 0);

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

    fn format_call(
        &mut self,
        expr: &Expr,
        indent: usize,
        _parent_prec: u8,
        callee: &str,
        args: &[Expr],
    ) -> Rendered {
        let has_newline = self.expr_has_newline(expr);
        let mut inline_text = None;

        if !has_newline {
            let saved = self.used_comments.clone();
            let mut parts = Vec::new();
            let mut inline_ok = true;
            for arg in args {
                if let Some(text) = self.format_expr_single_line(arg, indent, 0) {
                    parts.push(text);
                } else {
                    inline_ok = false;
                    break;
                }
            }

            if inline_ok {
                let text = format!("{}({})", callee, parts.join(", "));
                let inline_len = text.len();
                if self.fits_on_line(indent, inline_len) && inline_len <= MAX_WIDTH {
                    inline_text = Some(text);
                } else {
                    self.used_comments = saved;
                }
            } else {
                self.used_comments = saved;
            }
        }

        if let Some(text) = inline_text {
            return Rendered::single(indent, text);
        }

        let mut out = Rendered::default();
        out.push_line(indent, format!("{}(", callee));
        for (idx, arg) in args.iter().enumerate() {
            let mut arg_r = self.format_expr_rendered(arg, indent + 1, 0);
            let is_last = idx + 1 == args.len();
            if !is_last {
                if let Some(last) = arg_r.lines.last_mut() {
                    last.text.push(',');
                }
            }
            out.append(arg_r);
        }
        out.push_line(indent, ")");
        out
    }

    fn format_member_call(
        &mut self,
        expr: &Expr,
        indent: usize,
        _parent_prec: u8,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Rendered {
        let has_newline = self.expr_has_newline(expr);

        if !has_newline {
            let saved = self.used_comments.clone();

            if let Some(receiver_inline) = self.format_expr_single_line(receiver, indent, 0) {
                let mut parts = Vec::new();
                let mut inline_ok = true;
                for arg in args {
                    if let Some(text) = self.format_expr_single_line(arg, indent, 0) {
                        parts.push(text);
                    } else {
                        inline_ok = false;
                        break;
                    }
                }

                if inline_ok {
                    let text = format!("{}.{}({})", receiver_inline, method, parts.join(", "));
                    let inline_len = text.len();
                    if self.fits_on_line(indent, inline_len) && inline_len <= MAX_WIDTH {
                        return Rendered::single(indent, text);
                    }
                }
            }

            self.used_comments = saved;
        }

        let mut out = Rendered::default();
        let receiver_r = self.format_expr_rendered(receiver, indent, 0);
        out.append(receiver_r);
        if let Some(last) = out.lines.last_mut() {
            last.text.push_str(&format!(".{}(", method));
        } else {
            out.push_line(indent, format!(".{}(", method));
        }

        for (idx, arg) in args.iter().enumerate() {
            let mut arg_r = self.format_expr_rendered(arg, indent + 1, 0);
            let is_last = idx + 1 == args.len();
            if !is_last {
                if let Some(last) = arg_r.lines.last_mut() {
                    last.text.push(',');
                }
            }
            out.append(arg_r);
        }
        out.push_line(indent, ")");
        out
    }

    fn format_expr_single_line(
        &mut self,
        expr: &Expr,
        indent: usize,
        parent_prec: u8,
    ) -> Option<String> {
        let rendered = self.format_expr_rendered(expr, indent, parent_prec);
        if rendered.lines.len() == 1 {
            Some(rendered.lines[0].text.clone())
        } else {
            None
        }
    }

    fn fits_on_line(&self, indent: usize, text_len: usize) -> bool {
        indent * INDENT + text_len <= MAX_WIDTH
    }

    fn forces_multiline(&self, expr: &Expr) -> bool {
        self.expr_has_comments(expr)
    }

    fn expr_has_comments(&self, expr: &Expr) -> bool {
        if self.has_attached_comments(expr) {
            return true;
        }
        match &expr.kind {
            ExprKind::Group { inner } => self.expr_has_comments(inner),
            ExprKind::Call { args, .. } => args.iter().any(|a| self.expr_has_comments(a)),
            ExprKind::MemberCall {
                receiver, args, ..
            } => self.expr_has_comments(receiver) || args.iter().any(|a| self.expr_has_comments(a)),
            ExprKind::Unary { expr, .. } => self.expr_has_comments(expr),
            ExprKind::Binary { left, right, .. } => {
                self.expr_has_comments(left) || self.expr_has_comments(right)
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => {
                self.expr_has_comments(cond)
                    || self.expr_has_comments(then)
                    || self.expr_has_comments(otherwise)
            }
            _ => false,
        }
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
            ExprKind::MemberCall {
                receiver, args, ..
            } => {
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
                let start_idx = expr.tokens.lo as usize;
                let start = self.tokens.get(start_idx).map(|t| t.span.start)?;
                let end_span = self.expr_span_from_tokens(args.last().unwrap())?;
                return Some(Span {
                    start,
                    end: end_span.end,
                });
            }
            _ => {}
        }

        let start_idx = expr.tokens.lo as usize;
        let end_idx = expr.tokens.hi as usize;
        if start_idx >= self.tokens.len() || end_idx == 0 || end_idx > self.tokens.len() {
            return None;
        }
        if end_idx <= start_idx {
            return None;
        }
        Some(Span {
            start: self.tokens[start_idx].span.start,
            end: self.tokens[end_idx - 1].span.end,
        })
    }

    fn expr_has_newline(&self, expr: &Expr) -> bool {
        let span = self.expr_span_from_tokens(expr).unwrap_or(expr.span);
        source_has_newline(span, self.source)
    }

    fn prev_nontrivia(&self, idx: usize) -> Option<usize> {
        if idx == 0 {
            return None;
        }
        let mut i = idx - 1;
        loop {
            let tok = &self.tokens[i];
            if !tok.is_trivia() {
                return Some(i);
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
        None
    }

    fn available_leading_comments(&self, expr: &Expr) -> Vec<usize> {
        let start = expr.tokens.lo as usize;
        let lo = self.prev_nontrivia(start).map(|i| i + 1).unwrap_or(0);
        (lo..start)
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
            let is_inline_block = matches!(tok.kind, TokenKind::BlockComment(_))
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
        let hi = expr.tokens.hi as usize;
        if hi == 0 || hi > self.tokens.len() {
            return None;
        }
        let last_tok_idx = (expr.tokens.hi - 1) as usize;
        let last_line = self
            .sm
            .line_col(self.tokens[last_tok_idx].span.end.saturating_sub(1))
            .0;

        let mut idx = hi;
        while idx < self.tokens.len() {
            let tok = &self.tokens[idx];
            if tok.is_trivia() {
                match &tok.kind {
                    TokenKind::Newline => break,
                    TokenKind::LineComment(_) => {
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
                    TokenKind::BlockComment(_) => {
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
                idx += 1;
                continue;
            }
            break;
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
            TokenKind::LineComment(sym) => format!("//{}", sym.text),
            TokenKind::BlockComment(sym) => format!("/*{}*/", sym.text),
            TokenKind::DocComment(kind, sym) => match kind {
                CommentKind::Line => format!("##{}", sym.text),
                CommentKind::Block => format!("/**{}*/", sym.text),
            },
            _ => String::new(),
        }
    }

    fn has_attached_comments(&self, expr: &Expr) -> bool {
        !self.available_leading_comments(expr).is_empty()
            || self.available_trailing_comment(expr).is_some()
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

fn unop_str(op: UnOpKind) -> &'static str {
    match op {
        UnOpKind::Not => "!",
        UnOpKind::Neg => "-",
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

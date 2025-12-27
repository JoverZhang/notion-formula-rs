use std::collections::HashSet;

use crate::ast::{BinOpKind, Expr, ExprKind, UnOpKind};
use crate::parser::{infix_binding_power, prefix_binding_power};
use crate::source_map::SourceMap;
use crate::token::{CommentKind, Token, TokenKind};

const INDENT: usize = 2;
const MAX_WIDTH: usize = 80;

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

    fn wrap_parens(mut self) -> Self {
        if let Some(first) = self.lines.first_mut() {
            first.text = format!("({}", first.text);
        }
        if let Some(last) = self.lines.last_mut() {
            last.text.push(')');
        }
        self
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

pub fn format_expr(expr: &Expr, source: &str, tokens: &[Token]) -> String {
    let mut fmt = Formatter::new(source, tokens);
    let one_line = expr.pretty();
    let force_multiline = fmt.forces_multiline(expr);

    let out = if !force_multiline && !one_line.contains('\n') && fmt.fits_on_line(0, one_line.len()) {
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

        for idx in self.take_leading_comments(expr) {
            out.push_line(indent, self.render_comment(idx));
        }

        let mut body = self.format_expr_kind(expr, indent, parent_prec);

        if let Some(idx) = self.take_trailing_comment(expr) {
            body.append_trailing(&self.render_comment(idx));
        }

        out.append(body);
        out
    }

    fn format_expr_kind(&mut self, expr: &Expr, indent: usize, parent_prec: u8) -> Rendered {
        match &expr.kind {
            ExprKind::Ident(sym) => Rendered::single(indent, sym.text.clone()),
            ExprKind::Group { inner } => self.format_group(indent, parent_prec, inner),
            ExprKind::Lit(lit) => match lit.kind {
                crate::token::LitKind::Number => Rendered::single(indent, lit.symbol.text.clone()),
                crate::token::LitKind::String => {
                    Rendered::single(indent, escape_string(&lit.symbol.text))
                }
                crate::token::LitKind::Bool => Rendered::single(indent, lit.symbol.text.clone()),
            },
            ExprKind::Call { callee, args } => {
                self.format_call(indent, parent_prec, callee.text.clone(), args)
            }
            ExprKind::Unary { op, expr: inner } => {
                self.format_unary(indent, parent_prec, op.node, inner)
            }
            ExprKind::Binary { op, left, right } => {
                self.format_binary(indent, parent_prec, op.node, left, right)
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => self.format_ternary(indent, parent_prec, cond, then, otherwise),
            ExprKind::Error => Rendered::single(indent, "<error>"),
        }
    }

    fn format_group(&mut self, indent: usize, _parent_prec: u8, inner: &Expr) -> Rendered {
        if !self.has_attached_comments(inner) {
            if let Some(inline) = self.try_inline(inner, 0) {
                let text = format!("({})", inline);
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
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
        indent: usize,
        parent_prec: u8,
        op: UnOpKind,
        inner: &Expr,
    ) -> Rendered {
        let bp = prefix_binding_power(op);
        let op_str = match op {
            UnOpKind::Not => "!",
            UnOpKind::Neg => "-",
        };

        if !self.has_attached_comments(inner) {
            if let Some(inline) = self.try_inline(inner, bp) {
                let mut text = String::new();
                text.push_str(op_str);
                text.push_str(&inline);
                if bp < parent_prec {
                    text = format!("({})", text);
                }
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
        }

        let mut out = Rendered::default();
        out.push_line(indent, format!("{}(", op_str));
        let inner_rendered = self.format_expr_rendered(inner, indent + 1, bp);
        out.append(inner_rendered);
        out.push_line(indent, ")");
        if bp < parent_prec {
            out = out.wrap_parens();
        }
        out
    }

    fn format_binary(
        &mut self,
        indent: usize,
        parent_prec: u8,
        op: BinOpKind,
        left: &Expr,
        right: &Expr,
    ) -> Rendered {
        let (l_bp, r_bp) = infix_binding_power(op);
        let op_str = binop_str(op);
        let need_parens = l_bp < parent_prec;

        let has_comments = self.has_attached_comments(left) || self.has_attached_comments(right);

        if !has_comments {
            if let (Some(lhs), Some(rhs)) =
                (self.try_inline(left, l_bp), self.try_inline(right, r_bp))
            {
                let mut text = format!("{} {} {}", lhs, op_str, rhs);
                if need_parens {
                    text = format!("({})", text);
                }
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
        }

        let mut out = Rendered::default();
        let left_rendered = self.format_expr_rendered(left, indent, l_bp);
        let mut right_rendered = self.format_expr_rendered(right, indent + 1, r_bp);

        if let Some(first) = right_rendered.lines.first_mut() {
            first.text = format!("{} {}", op_str, first.text);
        } else {
            right_rendered.push_line(indent + 1, op_str.to_string());
        }

        out.append(left_rendered);
        out.append(right_rendered);

        if need_parens {
            out = out.wrap_parens();
        }
        out
    }

    fn format_ternary(
        &mut self,
        indent: usize,
        parent_prec: u8,
        cond: &Expr,
        then: &Expr,
        otherwise: &Expr,
    ) -> Rendered {
        let ternary_prec = 2;
        let need_parens = ternary_prec < parent_prec;

        if !self.has_attached_comments(cond)
            && !self.has_attached_comments(then)
            && !self.has_attached_comments(otherwise)
        {
            if let (Some(c), Some(t), Some(o)) = (
                self.try_inline(cond, 0),
                self.try_inline(then, 0),
                self.try_inline(otherwise, 0),
            ) {
                let mut text = format!("{} ? {} : {}", c, t, o);
                if need_parens {
                    text = format!("({})", text);
                }
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
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

        if need_parens {
            out = out.wrap_parens();
        }

        out
    }

    fn format_call(
        &mut self,
        indent: usize,
        parent_prec: u8,
        callee: String,
        args: &[Expr],
    ) -> Rendered {
        let call_prec = 16;
        let need_parens = call_prec < parent_prec;

        let any_complex_arg = args.iter().any(|a| self.is_complex_expr(a));
        if args.len() <= 1 && !any_complex_arg {
            if let Some(inline_args) = self.try_inline_args(args) {
                let mut text = format!("{}({})", callee, inline_args);
                if need_parens {
                    text = format!("({})", text);
                }
                if self.fits_on_line(indent, text.len()) {
                    return Rendered::single(indent, text);
                }
            }
        }

        let mut out = Rendered::default();
        out.push_line(indent, format!("{}(", callee));
        for arg in args {
            let mut arg_r = self.format_expr_rendered(arg, indent + 1, 0);
            if let Some(last) = arg_r.lines.last_mut() {
                last.text.push(',');
            }
            out.append(arg_r);
        }
        out.push_line(indent, ")");

        if need_parens {
            out = out.wrap_parens();
        }
        out
    }

    fn try_inline_args(&self, args: &[Expr]) -> Option<String> {
        let mut parts = Vec::new();
        for arg in args {
            if self.has_attached_comments(arg) {
                return None;
            }
            parts.push(self.inline_expr(arg, 0)?);
        }
        Some(parts.join(", "))
    }

    fn try_inline(&self, expr: &Expr, parent_prec: u8) -> Option<String> {
        if self.has_attached_comments(expr) {
            return None;
        }
        let inline = self.inline_expr(expr, parent_prec)?;
        if inline.contains('\n') {
            None
        } else {
            Some(inline)
        }
    }

    fn inline_expr(&self, expr: &Expr, parent_prec: u8) -> Option<String> {
        let text = match &expr.kind {
            ExprKind::Ident(sym) => sym.text.clone(),
            ExprKind::Group { inner } => {
                let inner = self.inline_expr(inner, 0)?;
                format!("({})", inner)
            }
            ExprKind::Lit(lit) => match lit.kind {
                crate::token::LitKind::Number => lit.symbol.text.clone(),
                crate::token::LitKind::String => escape_string(&lit.symbol.text),
                crate::token::LitKind::Bool => lit.symbol.text.clone(),
            },
            ExprKind::Call { callee, args } => {
                let mut buf = String::new();
                buf.push_str(&callee.text);
                buf.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        buf.push_str(", ");
                    }
                    buf.push_str(&self.inline_expr(arg, 0)?);
                }
                buf.push(')');
                buf
            }
            ExprKind::Unary { op, expr: inner } => {
                let mut buf = match op.node {
                    UnOpKind::Not => "!".to_string(),
                    UnOpKind::Neg => "-".to_string(),
                };
                buf.push_str(&self.inline_expr(inner, prefix_binding_power(op.node))?);
                buf
            }
            ExprKind::Binary { op, left, right } => {
                let (l_bp, r_bp) = infix_binding_power(op.node);
                let l = self.inline_expr(left, l_bp)?;
                let r = self.inline_expr(right, r_bp)?;
                let base = format!("{} {} {}", l, binop_str(op.node), r);
                if l_bp < parent_prec {
                    format!("({})", base)
                } else {
                    base
                }
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => {
                let c = self.inline_expr(cond, 0)?;
                let t = self.inline_expr(then, 0)?;
                let o = self.inline_expr(otherwise, 0)?;
                let base = format!("{} ? {} : {}", c, t, o);
                if parent_prec > 0 {
                    format!("({})", base)
                } else {
                    base
                }
            }
            ExprKind::Error => "<error>".to_string(),
        };

        Some(text)
    }

    fn is_complex_expr(&self, expr: &Expr) -> bool {
        (match &expr.kind {
            ExprKind::Binary { .. } | ExprKind::Ternary { .. } | ExprKind::Call { .. } => true,
            ExprKind::Unary { expr, .. } => self.is_complex_expr(expr),
            ExprKind::Group { inner } => self.is_complex_expr(inner),
            _ => false,
        }) || self.has_attached_comments(expr)
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

    fn take_leading_comments(&mut self, expr: &Expr) -> Vec<usize> {
        let comments = self.available_leading_comments(expr);
        for idx in &comments {
            self.used_comments.insert(*idx);
        }
        comments
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
        Dot => ".",
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

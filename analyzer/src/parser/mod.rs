use crate::ast::{BinOpKind, Expr, ExprKind, UnOpKind};
use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::token::{LitKind, NodeId, Span, Token, TokenKind, TokenRange};
use crate::tokenstream::TokenCursor;

mod expr;
mod pretty;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: TokenKind,
        span: Span,
    },
    LexError(String),
}
pub struct Parser<'a> {
    token_cursor: TokenCursor<'a>,
    next_id: NodeId,
    diagnostics: Diagnostics,
}

#[derive(Debug)]
pub struct ParseOutput {
    pub expr: Expr,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(token_cursor: TokenCursor<'a>) -> Self {
        Parser {
            token_cursor,
            next_id: 0,
            diagnostics: Diagnostics::default(),
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn cur(&self) -> &Token {
        &self.token_cursor.tokens[self.token_cursor.pos]
    }

    fn cur_kind(&self) -> &TokenKind {
        &self.token_cursor.tokens[self.token_cursor.pos].kind
    }

    fn cur_idx(&self) -> u32 {
        self.token_cursor.pos as u32
    }

    fn bump(&mut self) -> Token {
        let tok = self.token_cursor.tokens[self.token_cursor.pos].clone();
        self.token_cursor.pos += 1;
        tok
    }

    fn lit_text(&self, span: Span) -> &'a str {
        &self.token_cursor.source[span.start as usize..span.end as usize]
    }

    #[allow(unused)]
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.same_kind(self.cur_kind(), &kind) {
            self.token_cursor.pos += 1;
            true
        } else {
            false
        }
    }

    /// punctuation
    fn expect_punct(&mut self, kind: TokenKind, expected: &'static str) -> Result<Token, ()> {
        if self.same_kind(self.cur_kind(), &kind) {
            Ok(self.bump())
        } else {
            let tok = self.cur().clone();
            self.emit_unexpected(expected, tok.kind.clone(), tok.span);
            Err(())
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ()> {
        match self.cur_kind() {
            TokenKind::Ident(..) => Ok(self.bump()),
            _ => {
                let tok = self.cur().clone();
                self.emit_unexpected("identifier", tok.kind.clone(), tok.span);
                Err(())
            }
        }
    }

    fn expect_literal_kind(&mut self, k: LitKind) -> Result<Token, ()> {
        match self.cur_kind() {
            TokenKind::Literal(lit) if lit.kind == k => Ok(self.bump()),
            _ => {
                let tok = self.cur().clone();
                self.emit_unexpected(&format!("{:?} literal", k), tok.kind.clone(), tok.span);
                Err(())
            }
        }
    }

    fn same_kind(&self, a: &TokenKind, b: &TokenKind) -> bool {
        use TokenKind::*;
        match (a, b) {
            (Ident(_), Ident(_)) => true,
            (Literal(_), Literal(_)) => true,
            (DocComment(..), DocComment(..)) => true,

            (Lt, Lt) | (Le, Le) | (EqEq, EqEq) | (Ne, Ne) | (Ge, Ge) | (Gt, Gt) => true,
            (AndAnd, AndAnd) | (OrOr, OrOr) | (Bang, Bang) => true,
            (Plus, Plus)
            | (Minus, Minus)
            | (Star, Star)
            | (Slash, Slash)
            | (Percent, Percent)
            | (Caret, Caret) => true,
            (Dot, Dot)
            | (Comma, Comma)
            | (Colon, Colon)
            | (Pound, Pound)
            | (Question, Question) => true,
            (OpenParen, OpenParen) | (CloseParen, CloseParen) | (Eof, Eof) => true,
            _ => false,
        }
    }

    fn span_from_tokens(&self, range: TokenRange) -> Span {
        let lo = range.lo as usize;
        let hi = range.hi as usize;
        if lo >= self.token_cursor.tokens.len()
            || hi == 0
            || hi > self.token_cursor.tokens.len()
            || lo >= hi
        {
            // fallback: empty span
            return self.cur().span;
        }
        let start = self.token_cursor.tokens[lo].span.start;
        let end = self.token_cursor.tokens[hi - 1].span.end;
        Span { start, end }
    }

    fn mk_token_range(&self, start: u32, end: u32) -> TokenRange {
        TokenRange::new(start as u32, end as u32)
    }

    fn mk_expr(&mut self, span: Span, token_range: TokenRange, kind: ExprKind) -> Expr {
        Expr {
            id: self.alloc_id(),
            span: span,
            tokens: token_range,
            kind,
        }
    }

    fn finish(&mut self, expr: Expr) -> ParseOutput {
        ParseOutput {
            expr,
            diagnostics: std::mem::take(&mut self.diagnostics.diags),
        }
    }

    fn emit_unexpected(&mut self, expected: &str, found: TokenKind, span: Span) {
        self.diagnostics
            .emit_error(span, format!("expected {}, found {:?}", expected, found));
    }
}

// precedence: bigger = tighter binding
pub fn infix_binding_power(op: BinOpKind) -> (u8, u8) {
    use BinOpKind::*;

    // Return (left_bp, right_bp)
    // Right-associative: (p, p) or (p, p-1)
    // Left-associative: (p, p+1)
    // Here we use the classic Pratt parser:
    match op {
        OrOr => (1, 2),
        AndAnd => (3, 4),
        EqEq | Ne => (5, 6),
        Lt | Le | Ge | Gt => (7, 8),
        Plus | Minus => (9, 10),
        Star | Slash | Percent => (11, 12),
        Caret => (13, 13),
        Dot => todo!(),
    }
}

pub fn prefix_binding_power(op: UnOpKind) -> u8 {
    match op {
        UnOpKind::Not => 14,
        UnOpKind::Neg => 14,
    }
}

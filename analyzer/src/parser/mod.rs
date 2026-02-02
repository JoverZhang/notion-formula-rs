//! Parser for formula expressions.
//!
//! Inputs: a [`TokenCursor`] over lexer tokens that include trivia and an explicit EOF token.
//! Spans are UTF-8 byte offsets into the original source, with half-open semantics `[start, end)`.
//! The parser skips trivia for `cur()`/`bump()`, but spans remain byte-based.
//!
//! Responsibility: build the AST plus parse diagnostics only. Semantic analysis is handled
//! separately in `analysis`.

use crate::diagnostics::{Diagnostic, Diagnostics};

pub mod ast;
use crate::lexer::{NodeId, Span, Token, TokenKind};
use ast::{BinOpKind, Expr, ExprKind, UnOp};
mod expr;
mod tokenstream;
pub use tokenstream::{TokenCursor, TokenQuery};

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
    pub tokens: Vec<Token>,
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

    fn cur(&self) -> Token {
        let idx = self.next_nontrivia_idx(self.token_cursor.pos);
        self.token_cursor.tokens[idx].clone()
    }

    fn bump(&mut self) -> Token {
        let idx = self.next_nontrivia_idx(self.token_cursor.pos);
        self.token_cursor.pos = idx;
        let tok = self.token_cursor.tokens[self.token_cursor.pos].clone();
        self.token_cursor.pos += 1;
        tok
    }

    fn last_bumped(&self) -> Option<&Token> {
        self.token_cursor
            .pos
            .checked_sub(1)
            .and_then(|i| self.token_cursor.tokens.get(i))
    }

    fn last_bumped_end(&self) -> u32 {
        self.last_bumped()
            .map(|t| t.span.end)
            .unwrap_or(self.cur().span.end)
    }

    fn lit_text(&self, span: Span) -> &'a str {
        &self.token_cursor.source[span.start as usize..span.end as usize]
    }

    #[allow(unused)]
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.same_kind(&self.cur().kind, &kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// punctuation
    fn expect_punct(&mut self, kind: TokenKind, expected: &'static str) -> Result<Token, ()> {
        if self.same_kind(&self.cur().kind, &kind) {
            Ok(self.bump())
        } else {
            let tok = self.cur().clone();
            self.emit_unexpected(expected, tok.kind.clone(), tok.span);
            Err(())
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ()> {
        match self.cur().kind {
            TokenKind::Ident(..) => Ok(self.bump()),
            _ => {
                let tok = self.cur().clone();
                self.emit_unexpected("identifier", tok.kind.clone(), tok.span);
                Err(())
            }
        }
    }

    fn same_kind(&self, a: &TokenKind, b: &TokenKind) -> bool {
        use TokenKind::*;

        matches!(
            (a, b),
            // Exact matches for individual token kinds
            (Ident(_), Ident(_))
            | (Literal(_), Literal(_))
            | (DocComment(..), DocComment(..))
            | (LineComment(_), LineComment(_))
            | (BlockComment(_), BlockComment(_))
            | (Newline, Newline)

            // Relational operators
            | (Lt, Lt)
            | (Le, Le)
            | (EqEq, EqEq)
            | (Ne, Ne)
            | (Ge, Ge)
            | (Gt, Gt)

            // Logical operators
            | (AndAnd, AndAnd)
            | (OrOr, OrOr)
            | (Bang, Bang)

            // Arithmetic operators
            | (Plus, Plus)
            | (Minus, Minus)
            | (Star, Star)
            | (Slash, Slash)
            | (Percent, Percent)
            | (Caret, Caret)

            // Punctuation
            | (Dot, Dot)
            | (Comma, Comma)
            | (Colon, Colon)
            | (Pound, Pound)
            | (Question, Question)

            // Parentheses and EOF
            | (OpenParen, OpenParen)
            | (CloseParen, CloseParen)
            | (OpenBracket, OpenBracket)
            | (CloseBracket, CloseBracket)
            | (Eof, Eof)
        )
    }

    fn is_trivia(kind: TokenKind) -> bool {
        kind.is_trivia()
    }

    fn next_nontrivia_idx(&self, mut idx: usize) -> usize {
        while idx < self.token_cursor.tokens.len() {
            if Self::is_trivia(self.token_cursor.tokens[idx].clone().kind) {
                idx += 1;
                continue;
            }
            break;
        }
        idx.min(self.token_cursor.tokens.len().saturating_sub(1))
    }

    fn mk_expr(&mut self, span: Span, kind: ExprKind) -> Expr {
        Expr {
            id: self.alloc_id(),
            span,
            kind,
        }
    }

    fn mk_expr_sp(&self, lhs_sp: Span, rhs_sp: Span) -> Span {
        Span {
            start: lhs_sp.start,
            end: rhs_sp.end,
        }
    }

    fn emit_unexpected(&mut self, expected: &str, found: TokenKind, span: Span) {
        self.diagnostics
            .emit_err(span, format!("expected {}, found {:?}", expected, found));
    }
}

/// Returns the Pratt binding power for an infix operator.
///
/// Larger numbers bind tighter.
///
/// Operator set handled by the expression parser:
/// - Logical: `||`, `&&`
/// - Equality: `==`, `!=`
/// - Comparison: `<`, `<=`, `>=`, `>`
/// - Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`
///
/// Associativity:
/// - Most operators are left-associative (e.g. `a - b - c` parses as `(a - b) - c`).
/// - `^` (power-like) is right-associative (e.g. `2 ^ 2 ^ 3` parses as `2 ^ (2 ^ 3)`).
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
    }
}

/// Returns the Pratt binding power for a prefix operator.
///
/// Prefix operators handled by the expression parser: `!` and unary `-`.
pub fn prefix_binding_power(op: UnOp) -> u8 {
    match op {
        UnOp::Not => 14,
        UnOp::Neg => 14,
    }
}

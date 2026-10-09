//! Lexer tokens and spans.
//!
//! [`Span`] uses UTF-8 byte offsets into the original source and is half-open `[start, end)`.
//! The lexer also emits a [`TokenKind::Eof`] token with an empty span at end of input.

use crate::Span;

pub type NodeId = u32;
pub type TokenIdx = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub text: String,
}

/// Half-open range of token indices: `[lo, hi)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRange {
    pub lo: TokenIdx,
    pub hi: TokenIdx,
}

impl TokenRange {
    pub fn new(lo: TokenIdx, hi: TokenIdx) -> Self {
        Self { lo, hi }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LitKind {
    Bool,
    Number,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lit {
    pub kind: LitKind,
    pub symbol: Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /* Expression-operator symbols. */
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `==`
    EqEq,
    /// `!=`
    Ne,
    /// `>=`
    Ge,
    /// `>`
    Gt,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,
    /// `not`
    Not,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `^`
    Caret,

    /* Structural symbols */
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `:`
    Colon,
    /// `#`
    Pound,
    /// `?`
    Question,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,

    /* Literals */
    /// Literal token.
    Literal(Lit),
    /// Identifier token.
    Ident(Symbol),

    /// A comment token (line or block).
    /// `Symbol` is the comment's data excluding its "quotes" (`/*`, `//`, etc),
    /// similarly to symbols in string literal tokens.
    DocComment(CommentKind, Symbol),
    /// Newline trivia (`\n`).
    Newline,

    /// End Of File
    Eof,
}

#[derive(Debug, Clone)]
/// A token with its source span.
///
/// `span` is a byte offset range into the original source (`[start, end)`).
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn can_begin_expr(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Bang
                | TokenKind::Not
                | TokenKind::Minus
                | TokenKind::Ident(..)
                | TokenKind::Literal(..)
                | TokenKind::OpenParen
                | TokenKind::OpenBracket
        )
    }

    pub fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }
}

impl TokenKind {
    pub fn is_trivia(&self) -> bool {
        matches!(self, TokenKind::DocComment(..) | TokenKind::Newline)
    }

    pub fn is_comment(&self) -> bool {
        matches!(self, TokenKind::DocComment(..))
    }

    pub fn to_str(&self) -> Option<&'static str> {
        use TokenKind::*;
        Some(match self {
            // Relational operators
            Lt => "<",
            Le => "<=",
            EqEq => "==",
            Ne => "!=",
            Ge => ">=",
            Gt => ">",

            // Logical operators
            AndAnd => "&&",
            OrOr => "||",
            Bang => "!",

            // Arithmetic operators
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            Caret => "^",

            // Punctuation
            Dot => ".",
            Comma => ",",
            Colon => ":",
            Pound => "#",
            Question => "?",

            // Delimiters
            OpenParen => "(",
            CloseParen => ")",
            OpenBracket => "[",
            CloseBracket => "]",

            _ => return None,
        })
    }
}

/// Returns the token-index range `[lo, hi)` covered by `span`.
///
/// `span` is a half-open byte range (UTF-8 offsets) into the original source: `[start, end)`.
/// For non-empty spans, this returns the tokens whose spans intersect `span`.
/// For empty spans (`start >= end`), this returns a stable insertion point `i..i`
/// (the first token with `token.span.start >= start`), which may be the EOF token.
///
/// Example (byte offsets are boundary indices; trivia omitted from token list):
/// ```text
/// Source:  ( a + b )
/// Tokens:  0:'(' 1:'a' 2:'+' 3:'b' 4:')' 5:EOF
///
/// Span [2, 7) covers "a + b"  ->  [lo, hi) = [1, 4)  (tokens 1..4)
/// Span [0, 1) covers "("      ->  [lo, hi) = [0, 1)
/// Span [8, 8) at end-of-input ->  [lo, hi) = [5, 5)  (insertion point at EOF)
/// ```
pub fn tokens_in_span(tokens: &[Token], span: Span) -> TokenRange {
    if tokens.is_empty() {
        return TokenRange::new(0, 0);
    }

    // Empty spans never intersect any half-open token span.
    // We still return a stable "insertion point" based on `span.start`.
    let start = span.start;
    let end = span.end;
    if start >= end {
        let idx = lower_bound_by_start(tokens, start);
        return TokenRange::new(idx, idx);
    }

    let lo = lower_bound_by_end(tokens, start);
    let hi = lower_bound_by_start(tokens, end);
    TokenRange::new(lo, hi)
}

fn lower_bound_by_end(tokens: &[Token], start: u32) -> u32 {
    // First token with `token.span.end > start`.
    let mut lo = 0usize;
    let mut hi = tokens.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if tokens[mid].span.end <= start {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo as u32
}

fn lower_bound_by_start(tokens: &[Token], end: u32) -> u32 {
    // First token with `token.span.start >= end`.
    let mut lo = 0usize;
    let mut hi = tokens.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if tokens[mid].span.start < end {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo as u32
}

//! Lexer token model and span/range helpers.
//!
//! The lexer produces a linear token stream over the original source text.
//!
//! **Canonical invariants**
//! - [`Span`] uses **UTF-8 byte offsets** into the original source string.
//! - Spans and token ranges are **half-open**: `[start, end)` (inclusive start, exclusive end).
//! - The lexer emits an explicit [`TokenKind::Eof`] token whose span is empty at end-of-input.
//!
//! **Read-this-first entry points**
//! - [`Span`]: byte-offset span invariant shared across the analyzer.
//! - [`TokenRange`]: half-open token-index range used by query APIs.
//! - [`tokens_in_span`]: maps a byte span to the intersecting token index range (with stable
//!   insertion-point behavior for empty spans).

pub type NodeId = u32;
pub type TokenIdx = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A half-open span (`[start, end)`) in the original source string, using **UTF-8 byte offsets**.
///
/// Invariant: `start`/`end` must be valid UTF-8 slice boundaries so that `&source[start..end]`
/// is always safe (no panics) when `source` is the same string that was lexed/parsed.
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A half-open range of token indices `[lo, hi)`.
///
/// This is the canonical "token slice" type used by query helpers (notably the parser's
/// `TokenQuery`).
pub struct TokenRange {
    /// Inclusive lower bound.
    pub lo: TokenIdx,
    /// Exclusive upper bound.
    pub hi: TokenIdx,
}

impl TokenRange {
    /// Construct a token range `[lo, hi)`.
    pub fn new(lo: TokenIdx, hi: TokenIdx) -> Self {
        Self { lo, hi }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
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

    /* Literals */
    /// Literal token.
    Literal(Lit),
    /// Identifier token.
    Ident(Symbol),

    /// A doc comment token.
    /// `Symbol` is the doc comment's data excluding its "quotes" (`/*`, `#`, etc)
    /// similarly to symbols in string literal tokens.
    DocComment(CommentKind, Symbol),
    /// A line comment token.
    LineComment(Symbol),
    /// A block comment token.
    BlockComment(Symbol),
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
    #[allow(unused)]
    pub fn can_begin_expr(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Ident(..)
                | TokenKind::OpenParen
                | TokenKind::Literal(..)
                | TokenKind::Bang
                | TokenKind::Minus
        )
    }

    pub fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }
}

impl TokenKind {
    pub fn is_trivia(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_)
                | TokenKind::BlockComment(_)
                | TokenKind::DocComment(..)
                | TokenKind::Newline
        )
    }

    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            TokenKind::LineComment(_) | TokenKind::BlockComment(_) | TokenKind::DocComment(..)
        )
    }
}

/// Returns the index range of all tokens whose `Token::span` intersects `span`.
///
/// Spans are half-open byte ranges: `[start, end)`.
///
/// Intersection rule (also half-open):
/// - `token.start < span.end && span.start < token.end`
///
/// The returned `TokenRange` uses the same half-open convention over indices:
/// - `lo` is inclusive
/// - `hi` is exclusive
///
/// If `span` is empty (`start >= end`), the result is always empty (`lo == hi`).
///
/// This assumes tokens are in source order (monotonic by `Token::span.start`).
///
/// EOF note:
/// - The lexer emits an EOF token with an empty span at the end of the input.
/// - Because EOF has an empty span, non-empty query spans will not intersect it, so it is naturally excluded.
/// - For empty query spans, the returned insertion point may be the EOF token index (e.g. at end-of-input).
///
/// Insertion point semantics for empty spans:
/// - When `span.start >= span.end`, this returns `idx..idx` where `idx` is the first token whose
///   `Token::span.start >= span.start` (i.e. the place a new token would be inserted without
///   breaking source order). This may be `tokens.len() - 1` when the EOF token is present.
///
/// ASCII example (byte offsets shown as *boundary indices*; ASCII-only illustration):
/// ```text
/// Source:  ( a + b )
/// Char:    ( ␠ a ␠ + ␠ b ␠ )
/// Index:   0 1 2 3 4 5 6 7 8
/// Tokens:  0:'(' 1:'a' 2:'+' 3:'b' 4:')'   // trivia omitted from token list for brevity
/// Span:        [2, 7) covers 'a' '+' 'b' (including any trivia between them)
/// Result:  lo=1, hi=4   (tokens[1..4])
///
/// Boundary example (half-open end is excluded):
/// Source:  a+b
/// Tokens:  0:'a' 1:'+' 2:'b'
/// Span:    [0,1) covers only 'a' (it does NOT include '+')
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

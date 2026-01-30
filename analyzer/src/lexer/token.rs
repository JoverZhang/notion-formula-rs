//! Lexer tokens and spans.
//!
//! [`Span`] uses UTF-8 byte offsets into the original source and is half-open `[start, end)`.
//! The lexer also emits a [`TokenKind::Eof`] token with an empty span at end of input.

pub type NodeId = u32;
pub type TokenIdx = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Half-open byte span into the source string: `[start, end)`.
///
/// `start` and `end` must be valid UTF-8 slice boundaries for that same source string.
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Half-open range of token indices: `[lo, hi)`.
pub struct TokenRange {
    pub lo: TokenIdx,
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
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,

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
                | TokenKind::OpenBracket
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

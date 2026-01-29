use crate::lexer::{Span, Token, TokenIdx, TokenKind, TokenRange, tokens_in_span};

pub struct TokenCursor<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
    pub pos: usize,
}

/// Read-only query helpers over a token slice.
///
/// This type centralizes "span → token range → neighbor token scanning" in one place so
/// callers (notably the formatter) don't duplicate index arithmetic or trivia-skipping loops.
///
/// # Index & range semantics
/// - All indices returned by this API are **token indices** into the underlying slice.
/// - All ranges follow Rust's standard **half-open** convention: `[lo, hi)`.
/// - Methods that take an `idx: usize` treat it as a **boundary index** in `[0, tokens.len()]`.
///   Passing an out-of-bounds value is treated as if it were `tokens.len()`.
///
/// # Trivia policy
/// - `Token::is_trivia()` is the sole definition of trivia here (comments + newlines today).
/// - EOF (`TokenKind::Eof`) is **not** trivia.
///
/// # EOF handling
/// The lexer emits an explicit EOF token. Neighbor scans treat EOF like any other non-trivia
/// token and may return its index when scanning across trailing trivia at end-of-input.
#[derive(Debug, Clone, Copy)]
pub struct TokenQuery<'a> {
    tokens: &'a [Token],
}

impl<'a> TokenQuery<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens }
    }

    /// Returns the index range of all tokens whose `Token::span` intersects `span`.
    ///
    /// This delegates to [`tokens_in_span`]; see that function for the precise intersection
    /// rule and empty-span behavior.
    ///
    /// The returned [`TokenRange`] is half-open over token indices: `[lo, hi)`.
    pub fn range_for_span(&self, span: Span) -> TokenRange {
        tokens_in_span(self.tokens, span)
    }

    /// Returns the nearest non-trivia token index strictly **before** `idx`.
    ///
    /// - `idx` is a boundary index in `[0, tokens.len()]`.
    /// - Returns `None` if there is no non-trivia token before `idx`.
    pub fn prev_nontrivia(&self, idx: usize) -> Option<usize> {
        let idx = idx.min(self.tokens.len());
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

    /// Returns the nearest non-trivia token index at or **after** `idx`.
    ///
    /// - `idx` is a boundary index in `[0, tokens.len()]`.
    /// - Returns `None` if there is no non-trivia token at or after `idx`.
    #[allow(dead_code)]
    pub fn next_nontrivia(&self, idx: usize) -> Option<usize> {
        let mut i = idx.min(self.tokens.len());
        while i < self.tokens.len() {
            let tok = &self.tokens[i];
            if !tok.is_trivia() {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Returns the first non-trivia token index inside `range` (half-open).
    ///
    /// Returns `None` if the clamped range is empty or contains only trivia.
    #[allow(dead_code)]
    pub fn first_nontrivia(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        let idx = self.next_nontrivia(lo)?;
        (idx < hi).then_some(idx)
    }

    /// Returns the last non-trivia token index inside `range` (half-open).
    ///
    /// Returns `None` if the clamped range is empty or contains only trivia.
    #[allow(dead_code)]
    pub fn last_nontrivia(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        let idx = self.prev_nontrivia(hi)?;
        (idx >= lo).then_some(idx)
    }

    /// Returns the first token index inside `range` (half-open), if non-empty.
    ///
    /// This does **not** skip trivia.
    pub fn first_in_range(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        (lo < hi).then_some(lo)
    }

    /// Returns the last token index inside `range` (half-open), if non-empty.
    ///
    /// This does **not** skip trivia.
    pub fn last_in_range(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        (lo < hi).then_some(hi - 1)
    }

    /// Returns the trivia token indices that occur immediately *before* `idx`,
    /// bounded on the left by the previous non-trivia token.
    ///
    /// Concretely:
    /// - Let `p = prev_nontrivia(idx)`.
    /// - The result is `(p + 1)..idx` if `p` exists, otherwise `0..idx`.
    ///
    /// The returned range is half-open and may include newlines and/or comments.
    pub fn leading_trivia_before(&self, idx: usize) -> std::ops::Range<usize> {
        let idx = idx.min(self.tokens.len());
        let lo = self.prev_nontrivia(idx).map(|i| i + 1).unwrap_or(0);
        lo..idx
    }

    /// Returns the trivia token indices starting at `idx`, stopping before:
    /// - the first non-trivia token, or
    /// - the first newline token.
    ///
    /// This is used by formatter comment attachment to consider only "same line" trivia.
    pub fn trailing_trivia_until_newline_or_nontrivia(&self, idx: usize) -> std::ops::Range<usize> {
        let mut i = idx.min(self.tokens.len());
        while i < self.tokens.len() {
            let tok = &self.tokens[i];
            if !tok.is_trivia() {
                break;
            }
            if matches!(tok.kind, TokenKind::Newline) {
                break;
            }
            i += 1;
        }
        idx.min(self.tokens.len())..i
    }

    /// Returns the clamped `[lo, hi)` bounds of `range` as `usize`.
    ///
    /// - Both ends are clamped to `tokens.len()`.
    /// - If `range.hi < range.lo`, the result is an empty range at `lo`.
    pub fn bounds_usize(&self, range: TokenRange) -> (usize, usize) {
        self.clamp_range_usize(range)
    }

    fn clamp_range_usize(&self, range: TokenRange) -> (usize, usize) {
        let len = self.tokens.len();
        let lo = (range.lo as usize).min(len);
        let hi = (range.hi as usize).min(len);
        (lo, hi.max(lo))
    }

    #[allow(dead_code)]
    fn clamp_token_idx(&self, idx: TokenIdx) -> usize {
        (idx as usize).min(self.tokens.len())
    }
}

impl<'a> TokenCursor<'a> {
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        TokenCursor {
            source,
            tokens,
            pos: 0,
        }
    }
}

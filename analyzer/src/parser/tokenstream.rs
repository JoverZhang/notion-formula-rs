//! Token stream helpers for the parser.
//!
//! Tokens keep trivia and an explicit EOF token, but the parser usually skips trivia.
//! Use [`TokenQuery`] for span-to-range and trivia-aware neighbor scans.

use crate::lexer::{Span, Token, TokenIdx, TokenKind, TokenRange, tokens_in_span};

/// A mutable cursor over a token stream.
///
/// `tokens` includes trivia and EOF. `pos` is a boundary index used by the parser.
pub struct TokenCursor<'a> {
    /// The original source string that token spans index into.
    pub source: &'a str,
    /// Tokens in source order, including trivia and an explicit EOF token.
    pub tokens: Vec<Token>,
    /// Current boundary index into `tokens` used by the parser.
    ///
    /// Parser methods advance `pos` but read tokens via trivia-skipping helpers, so `pos` acts as a
    /// stable boundary even when trivia tokens are present.
    pub pos: usize,
}

/// Trivia-aware query helpers over a token slice.
///
/// Canonical API for span → token-range mapping and neighbor scans.
/// All ranges are half-open `[lo, hi)` over token indices and are clamped to `tokens.len()`.
///
/// EOF: the lexer emits an explicit EOF token with an empty span. Non-empty spans do not intersect
/// it; empty spans may map to an insertion point at the EOF index.
///
/// ```text
/// source: "a\n+ b"
/// tokens (incl trivia): Ident("a") Newline Plus Ident("b") Eof   // spaces skipped by lexer
/// span [0, 1) -> TokenRange covers Ident("a") only
/// next_nontrivia(1) -> Plus     // skips the Newline trivia token
/// prev_nontrivia(3) -> Plus     // looks left from Ident("b")
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TokenQuery<'a> {
    tokens: &'a [Token],
}

impl<'a> TokenQuery<'a> {
    /// Build a query view over a token slice.
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens }
    }

    /// Get the token index range that intersects `span`.
    ///
    /// Delegates to [`tokens_in_span`] for the exact rules (including empty spans).
    pub fn range_for_span(&self, span: Span) -> TokenRange {
        tokens_in_span(self.tokens, span)
    }

    /// Find the nearest non-trivia token strictly before `idx`.
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

    /// Find the nearest non-trivia token at or after `idx`.
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

    /// Find the first non-trivia token inside `range` (`[lo, hi)`).
    #[allow(dead_code)]
    pub fn first_nontrivia(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        let idx = self.next_nontrivia(lo)?;
        (idx < hi).then_some(idx)
    }

    /// Find the last non-trivia token inside `range` (`[lo, hi)`).
    #[allow(dead_code)]
    pub fn last_nontrivia(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        let idx = self.prev_nontrivia(hi)?;
        (idx >= lo).then_some(idx)
    }

    /// Get the first token index inside `range` (`[lo, hi)`), if any.
    pub fn first_in_range(&self, range: TokenRange) -> Option<usize> {
        let (lo, hi) = self.clamp_range_usize(range);
        (lo < hi).then_some(lo)
    }

    /// Get the last token index inside `range` (`[lo, hi)`), if any.
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

    /// Get the clamped `[lo, hi)` bounds of `range` as `usize`.
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
    /// Construct a cursor at the start of `tokens`.
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        TokenCursor {
            source,
            tokens,
            pos: 0,
        }
    }
}

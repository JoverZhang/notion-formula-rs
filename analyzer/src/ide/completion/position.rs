//! Cursor/position helpers for completion.
//! All `cursor` values are UTF-8 byte offsets into the original source text.

use crate::lexer::{LitKind, Span, Token, TokenKind};
use crate::semantic;

/// Coarse completion position, derived from nearby non-trivia tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PositionKind {
    NeedExpr,
    AfterAtom,
    AfterDot,
    None,
}

/// Classifies the cursor position using token neighbors.
pub(super) fn detect_position_kind(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
) -> PositionKind {
    if is_postfix_member_access_position(tokens, cursor) {
        return PositionKind::AfterDot;
    }

    if is_strictly_inside_ident(tokens, cursor) {
        return PositionKind::NeedExpr;
    }

    if has_extending_ident_prefix(tokens, cursor, ctx) {
        return PositionKind::NeedExpr;
    }

    let prev = prev_non_trivia_insertion(tokens, cursor).map(|(_, token)| token);
    if is_expr_start_position(prev) {
        return PositionKind::NeedExpr;
    }

    match prev.map(|token| &token.kind) {
        Some(TokenKind::Ident(_)) | Some(TokenKind::Literal(_)) | Some(TokenKind::CloseParen) => {
            PositionKind::AfterAtom
        }
        _ => PositionKind::None,
    }
}

fn is_postfix_member_access_position(tokens: &[Token], cursor: u32) -> bool {
    postfix_member_access_dot_index(tokens, cursor).is_some()
}

fn dot_has_receiver_atom(tokens: &[Token], dot_idx: usize) -> bool {
    prev_non_trivia_before(tokens, dot_idx).is_some_and(|(_, token)| {
        matches!(
            token.kind,
            TokenKind::Ident(_) | TokenKind::Literal(_) | TokenKind::CloseParen
        )
    })
}

/// Returns the dot token index for member-access completion at `cursor`.
///
/// This is a token-connectivity check (receiver atom + `.` + optional method prefix).
pub(super) fn postfix_member_access_dot_index(tokens: &[Token], cursor: u32) -> Option<usize> {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor)
        && matches!(token.kind, TokenKind::Ident(_))
        && let Some((dot_idx, dot_token)) = prev_non_trivia_before(tokens, idx)
        && matches!(dot_token.kind, TokenKind::Dot)
        && dot_has_receiver_atom(tokens, dot_idx)
    {
        return Some(dot_idx);
    }

    let (prev_idx, prev_token) = prev_non_trivia_insertion(tokens, cursor)?;
    match prev_token.kind {
        TokenKind::Dot if dot_has_receiver_atom(tokens, prev_idx) => Some(prev_idx),
        TokenKind::Ident(_) => {
            let (dot_idx, dot_token) = prev_non_trivia_before(tokens, prev_idx)?;
            if matches!(dot_token.kind, TokenKind::Dot) && dot_has_receiver_atom(tokens, dot_idx) {
                Some(dot_idx)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_strictly_inside_ident(tokens: &[Token], cursor: u32) -> bool {
    let Some((_, token)) = token_containing_cursor(tokens, cursor) else {
        return false;
    };
    let token_is_ident_like = match &token.kind {
        TokenKind::Ident(_) | TokenKind::Not => true,
        TokenKind::Literal(lit) if lit.kind == LitKind::Bool => true,
        _ => false,
    };

    token_is_ident_like && token.span.start < cursor && cursor < token.span.end
}

pub(super) fn cursor_strictly_inside_string_literal(tokens: &[Token], cursor: u32) -> bool {
    let Some((_, token)) = token_containing_cursor(tokens, cursor) else {
        return false;
    };
    let TokenKind::Literal(ref lit) = token.kind else {
        return false;
    };
    lit.kind == LitKind::String && token.span.start < cursor && cursor < token.span.end
}

fn has_extending_ident_prefix(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
) -> bool {
    let Some((_, token)) = prev_non_trivia_insertion(tokens, cursor) else {
        return false;
    };
    if token.span.end != cursor {
        return false;
    }
    let TokenKind::Ident(ref symbol) = token.kind else {
        return false;
    };
    has_extending_completion_prefix(&symbol.text, ctx)
}

fn has_extending_completion_prefix(prefix: &str, ctx: Option<&semantic::Context>) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let prefix_lower = prefix.to_ascii_lowercase();

    if "true".starts_with(&prefix_lower) && prefix_lower != "true" {
        return true;
    }
    if "false".starts_with(&prefix_lower) && prefix_lower != "false" {
        return true;
    }
    if "not".starts_with(&prefix_lower) && prefix_lower != "not" {
        return true;
    }

    let Some(ctx) = ctx else {
        return false;
    };

    if ctx.functions.iter().any(|func| {
        func.name.to_ascii_lowercase().starts_with(&prefix_lower) && func.name != prefix_lower
    }) {
        return true;
    }

    if ctx.properties.iter().any(|prop| {
        prop.name.to_ascii_lowercase().starts_with(&prefix_lower) && prop.name != prefix_lower
    }) {
        return true;
    }

    false
}

/// Like `prev_non_trivia`, but treats `cursor == token.span.start` as “before the token”.
///
/// This makes completion before `)` behave like insertion, not “after `)`”.
pub(super) fn prev_non_trivia_insertion(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
    prev_non_trivia_impl(tokens, cursor, CursorBoundary::Insertion)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorBoundary {
    /// Treat `cursor == token.span.start` as inside the token.
    Containing,
    /// Treat `cursor == token.span.start` as before the token.
    Insertion,
}

fn prev_non_trivia_impl(
    tokens: &[Token],
    cursor: u32,
    boundary: CursorBoundary,
) -> Option<(usize, &Token)> {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor)
        && !token.is_trivia()
        && !matches!(token.kind, TokenKind::Eof)
        && (boundary == CursorBoundary::Containing || token.span.start < cursor)
    {
        return Some((idx, token));
    }

    let mut prev = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.end <= cursor {
            prev = Some((idx, token));
        } else {
            break;
        }
    }
    prev
}

/// Finds the previous non-trivia token before `idx` (token index, not bytes).
pub(super) fn prev_non_trivia_before(tokens: &[Token], idx: usize) -> Option<(usize, &Token)> {
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let token = &tokens[i];
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        return Some((i, token));
    }
    None
}

fn token_containing_cursor(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
    tokens.iter().enumerate().find(|(_, token)| {
        token.span.start <= cursor
            && cursor < token.span.end
            && !matches!(token.kind, TokenKind::Eof)
    })
}

fn is_expr_start_position(prev_token: Option<&Token>) -> bool {
    match prev_token.map(|token| &token.kind) {
        None => true,
        Some(TokenKind::Ident(_)) => false,
        Some(TokenKind::Literal(_)) => false,
        Some(TokenKind::CloseParen) => false,
        _ => true,
    }
}

/// Chooses the replace span for expression-start completion.
///
/// At an identifier boundary, it may return the identifier span for prefix editing.
pub(super) fn replace_span_for_expr_start(tokens: &[Token], cursor: u32) -> Span {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor)
        && matches!(token.kind, TokenKind::Ident(_))
    {
        // Completing right before an identifier inserts (does not replace it).
        if cursor == token.span.start {
            return Span {
                start: cursor,
                end: cursor,
            };
        }

        // Strictly inside an identifier: treat as prefix editing.
        return tokens[idx].span;
    }
    if let Some((_, token)) = prev_non_trivia_insertion(tokens, cursor)
        && matches!(token.kind, TokenKind::Ident(_))
        && token.span.end == cursor
    {
        return token.span;
    }
    Span {
        start: cursor,
        end: cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::prev_non_trivia_insertion;
    use crate::lexer::{TokenKind, lex};

    #[test]
    fn prev_non_trivia_insertion_treats_cursor_at_token_start_as_before() {
        let source = "a)";
        let tokens = lex(source).tokens;

        // Cursor is at the start of `)`.
        let cursor = 1;
        let (_, containing) =
            super::prev_non_trivia_impl(&tokens, cursor, super::CursorBoundary::Containing)
                .unwrap();
        assert!(matches!(&containing.kind, TokenKind::CloseParen));

        let (_, insertion) = prev_non_trivia_insertion(&tokens, cursor).unwrap();
        assert!(matches!(&insertion.kind, TokenKind::Ident(_)));
    }
}

//! Cursor-context detection helpers shared by completion and signature help.
//! All coordinates are UTF-8 byte offsets into the original source text.

use analyzer::semantic;
use analyzer::{LitKind, Span, Token, TokenKind};

/// Coarse completion position derived from nearby non-trivia tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PositionKind {
    NeedExpr,
    AfterAtom,
    AfterDot,
    None,
}

/// Call-site information derived from tokens for the cursor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallContext {
    pub(crate) callee: String,
    pub(crate) lparen_idx: usize,
    pub(crate) arg_index: usize,
}

/// Full cursor context used by IDE help orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CursorContext {
    pub(crate) call_ctx: Option<CallContext>,
    pub(crate) position_kind: PositionKind,
    pub(crate) replace: Span,
    pub(crate) query: Option<String>,
}

/// Detects call/position/replace/query context for the current cursor.
pub(crate) fn detect_cursor_context(
    text: &str,
    tokens: &[Token],
    cursor: u32,
    semantic_ctx: &semantic::Context,
) -> CursorContext {
    let call_ctx = detect_call_context(tokens, cursor);
    let position_kind = if cursor_strictly_inside_string_literal(tokens, cursor) {
        PositionKind::None
    } else {
        detect_position_kind(tokens, cursor, semantic_ctx)
    };

    let replace = match position_kind {
        PositionKind::NeedExpr | PositionKind::AfterDot => {
            replace_span_for_expr_start(tokens, cursor)
        }
        PositionKind::AfterAtom | PositionKind::None => Span {
            start: cursor,
            end: cursor,
        },
    };

    let query = completion_query_for_replace(text, replace);

    CursorContext {
        call_ctx,
        position_kind,
        replace,
        query,
    }
}

/// Classifies the cursor position using token neighbors.
pub(crate) fn detect_position_kind(
    tokens: &[Token],
    cursor: u32,
    ctx: &semantic::Context,
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
pub(crate) fn postfix_member_access_dot_index(tokens: &[Token], cursor: u32) -> Option<usize> {
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

fn cursor_strictly_inside_string_literal(tokens: &[Token], cursor: u32) -> bool {
    let Some((_, token)) = token_containing_cursor(tokens, cursor) else {
        return false;
    };
    let TokenKind::Literal(ref lit) = token.kind else {
        return false;
    };
    lit.kind == LitKind::String && token.span.start < cursor && cursor < token.span.end
}

fn has_extending_ident_prefix(tokens: &[Token], cursor: u32, ctx: &semantic::Context) -> bool {
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

fn has_extending_completion_prefix(prefix: &str, ctx: &semantic::Context) -> bool {
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

/// Like `prev_non_trivia`, but treats `cursor == token.span.start` as "before the token".
///
/// This makes completion before `)` behave like insertion, not "after `)`".
pub(crate) fn prev_non_trivia_insertion(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
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
pub(crate) fn prev_non_trivia_before(tokens: &[Token], idx: usize) -> Option<(usize, &Token)> {
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
pub(crate) fn replace_span_for_expr_start(tokens: &[Token], cursor: u32) -> Span {
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

/// Finds the innermost call whose `(` starts before `cursor`.
pub(crate) fn detect_call_context(tokens: &[Token], cursor: u32) -> Option<CallContext> {
    let mut stack = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::OpenParen => stack.push(idx),
            TokenKind::CloseParen => {
                let _ = stack.pop();
            }
            _ => {}
        }
    }
    let lparen_idx = *stack.last()?;
    let (_, callee_token) = prev_non_trivia_before(tokens, lparen_idx)?;
    let TokenKind::Ident(ref symbol) = callee_token.kind else {
        return None;
    };

    let mut arg_index = 0usize;
    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    for token in tokens.iter().skip(lparen_idx + 1) {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::OpenParen => paren_depth += 1,
            TokenKind::OpenBracket => bracket_depth += 1,
            TokenKind::CloseParen => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            TokenKind::CloseBracket => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            TokenKind::Comma => {
                if paren_depth == 0 && bracket_depth == 0 {
                    arg_index += 1;
                }
            }
            _ => {}
        }
    }

    Some(CallContext {
        callee: symbol.text.clone(),
        lparen_idx,
        arg_index,
    })
}

/// Best-effort expected type for the active argument position.
///
/// Returns `None` for wildcard-ish types (`Unknown` and `Generic(_)`).
pub(crate) fn expected_call_arg_ty(
    call_ctx: Option<&CallContext>,
    ctx: &semantic::Context,
) -> Option<semantic::Ty> {
    let call_ctx = call_ctx?;
    let ty = ctx
        .functions
        .iter()
        .find(|func| func.name == call_ctx.callee)
        .and_then(|func| func.param_for_arg_index(call_ctx.arg_index))
        .map(|param| param.ty.clone())?;

    match ty {
        semantic::Ty::Unknown | semantic::Ty::Generic(_) => None,
        other => Some(other),
    }
}

/// Extracts the normalized query string from the replacement span.
pub(crate) fn completion_query_for_replace(text: &str, replace: Span) -> Option<String> {
    if replace.start == replace.end {
        return None;
    }

    let start = usize::try_from(u32::min(replace.start, replace.end)).ok()?;
    let end = usize::try_from(u32::max(replace.start, replace.end)).ok()?;
    if end > text.len() {
        return None;
    }
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }

    let raw = text.get(start..end)?;
    if raw.chars().all(|c| c.is_whitespace()) {
        return None;
    }
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c.is_whitespace())
    {
        return None;
    }

    let query: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if query.is_empty() {
        return None;
    }

    Some(query)
}

#[cfg(test)]
mod tests {
    use super::prev_non_trivia_insertion;
    use analyzer::TokenKind;

    #[test]
    fn prev_non_trivia_insertion_treats_cursor_at_token_start_as_before() {
        let source = "a)";
        let tokens = analyzer::analyze_syntax(source).tokens;

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

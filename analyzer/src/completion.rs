use crate::lexer::lex;
use crate::semantic;
use crate::token::{LitKind, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionOutput {
    pub items: Vec<CompletionItem>,
    pub replace: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub insert_text: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Keyword,
    Property,
    Operator,
    Literal,
    Snippet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropState {
    AfterPropIdent,
    AfterPropLParen,
    InPropStringContent(Span),
    None,
}

/// Compute completions using byte offsets for the cursor and replace span.
pub fn complete_with_context(
    text: &str,
    cursor: usize,
    ctx: Option<&semantic::Context>,
) -> CompletionOutput {
    let cursor_u32 = u32::try_from(cursor).unwrap_or(u32::MAX);
    let lex_output = lex(text);
    let tokens = lex_output.tokens;

    let default_replace = Span {
        start: cursor_u32,
        end: cursor_u32,
    };

    if tokens.is_empty()
        || tokens
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Eof))
    {
        let items = if cursor == 0 {
            expr_start_items()
        } else {
            Vec::new()
        };
        return CompletionOutput {
            items,
            replace: default_replace,
        };
    }

    match detect_prop_state(&tokens, cursor_u32) {
        PropState::AfterPropIdent => {
            return CompletionOutput {
                items: vec![CompletionItem {
                    label: "(".to_string(),
                    kind: CompletionKind::Operator,
                    insert_text: "(".to_string(),
                    detail: None,
                }],
                replace: default_replace,
            };
        }
        PropState::AfterPropLParen => {
            return CompletionOutput {
                items: vec![CompletionItem {
                    label: "\"".to_string(),
                    kind: CompletionKind::Literal,
                    insert_text: "\"".to_string(),
                    detail: None,
                }],
                replace: default_replace,
            };
        }
        PropState::InPropStringContent(content_span) => {
            let items = ctx
                .map(|ctx| {
                    ctx.properties
                        .iter()
                        .map(|prop| CompletionItem {
                            label: prop.name.clone(),
                            kind: CompletionKind::Property,
                            insert_text: prop.name.clone(),
                            detail: None,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            return CompletionOutput {
                items,
                replace: content_span,
            };
        }
        PropState::None => {}
    }

    let prev_token = prev_non_trivia(tokens.as_slice(), cursor_u32);
    if !is_expr_start_position(prev_token.map(|(_, token)| token)) {
        return CompletionOutput {
            items: Vec::new(),
            replace: default_replace,
        };
    }

    let replace = replace_span_for_expr_start(tokens.as_slice(), cursor_u32);
    CompletionOutput {
        items: expr_start_items(),
        replace,
    }
}

fn expr_start_items() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "prop(\"".to_string(),
            kind: CompletionKind::Snippet,
            insert_text: "prop(\"".to_string(),
            detail: Some("Property reference".to_string()),
        },
        CompletionItem {
            label: "if(".to_string(),
            kind: CompletionKind::Snippet,
            insert_text: "if(".to_string(),
            detail: Some("if(condition, then, else)".to_string()),
        },
        CompletionItem {
            label: "sum(".to_string(),
            kind: CompletionKind::Snippet,
            insert_text: "sum(".to_string(),
            detail: Some("sum(number)".to_string()),
        },
        CompletionItem {
            label: "true".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "true".to_string(),
            detail: None,
        },
        CompletionItem {
            label: "false".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "false".to_string(),
            detail: None,
        },
        CompletionItem {
            label: "(".to_string(),
            kind: CompletionKind::Operator,
            insert_text: "(".to_string(),
            detail: None,
        },
    ]
}

fn prev_non_trivia(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor) {
        if !token.is_trivia() && !matches!(token.kind, TokenKind::Eof) {
            return Some((idx, token));
        }
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

fn next_non_trivia(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            return Some((idx, token));
        }
    }
    None
}

fn prev_non_trivia_before(tokens: &[Token], idx: usize) -> Option<(usize, &Token)> {
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
        token.span.start <= cursor && cursor < token.span.end && !matches!(token.kind, TokenKind::Eof)
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

fn replace_span_for_expr_start(tokens: &[Token], cursor: u32) -> Span {
    if let Some((_, token)) = token_containing_cursor(tokens, cursor) {
        if matches!(token.kind, TokenKind::Ident(_)) {
            return token.span;
        }
    }
    Span {
        start: cursor,
        end: cursor,
    }
}

fn detect_prop_state(tokens: &[Token], cursor: u32) -> PropState {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor) {
        if let TokenKind::Literal(lit) = &token.kind {
            if lit.kind == LitKind::String {
                let content_span = string_content_span(token.span);
                if span_contains_cursor_inclusive(content_span, cursor) {
                    if let Some((open_idx, open_token)) = prev_non_trivia_before(tokens, idx) {
                        if matches!(open_token.kind, TokenKind::OpenParen) {
                            if let Some((_, ident_token)) = prev_non_trivia_before(tokens, open_idx)
                            {
                                if is_prop_ident(ident_token) {
                                    return PropState::InPropStringContent(content_span);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((prev_idx, prev_token)) = prev_non_trivia(tokens, cursor) {
        if matches!(prev_token.kind, TokenKind::OpenParen) && cursor >= prev_token.span.end {
            if let Some((_, ident_token)) = prev_non_trivia_before(tokens, prev_idx) {
                if is_prop_ident(ident_token) {
                    if let Some((_, next_token)) = next_non_trivia(tokens, cursor) {
                        if cursor <= next_token.span.start {
                            return PropState::AfterPropLParen;
                        }
                    } else {
                        return PropState::AfterPropLParen;
                    }
                }
            }
        }
    }

    if let Some((_, prev_token)) = prev_non_trivia(tokens, cursor) {
        if is_prop_ident(prev_token) && cursor >= prev_token.span.end {
            if let Some((_, next_token)) = next_non_trivia(tokens, cursor) {
                if cursor <= next_token.span.start {
                    return PropState::AfterPropIdent;
                }
            } else {
                return PropState::AfterPropIdent;
            }
        }
    }

    PropState::None
}

fn is_prop_ident(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Ident(ref symbol) if symbol.text == "prop")
}

fn string_content_span(span: Span) -> Span {
    let start = span.start.saturating_add(1);
    let end = span.end.saturating_sub(1);
    Span { start, end }
}

fn span_contains_cursor_inclusive(span: Span, cursor: u32) -> bool {
    span.start <= cursor && cursor <= span.end
}

use crate::lexer::lex;
use crate::semantic;
use crate::token::{LitKind, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionOutput {
    pub items: Vec<CompletionItem>,
    pub replace: Span,
    pub signature_help: Option<SignatureHelp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Span,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub insert_text: String,
    pub primary_edit: Option<TextEdit>,
    pub cursor: Option<u32>,
    pub additional_edits: Vec<TextEdit>,
    pub detail: Option<String>,
    pub is_disabled: bool,
    pub disabled_reason: Option<String>,
    pub data: Option<CompletionData>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionData {
    Function { name: String },
    PropertyName { name: String },
    PropExpr { property_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub params: Vec<String>,
    pub active_param: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropState {
    AfterPropIdent,
    AfterPropLParen,
    InPropStringContent(Span),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallContext {
    callee: String,
    lparen_idx: usize,
    arg_index: usize,
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
            expr_start_items(ctx)
        } else {
            Vec::new()
        };
        return finalize_output(
            CompletionOutput {
                items,
                replace: default_replace,
                signature_help: None,
            },
            ctx,
        );
    }

    let output = match detect_prop_state(&tokens, cursor_u32) {
        PropState::AfterPropIdent => {
            let items = ctx.map(prop_variable_items).unwrap_or_default();
            CompletionOutput {
                items: if items.is_empty() {
                    vec![CompletionItem {
                        label: "(".to_string(),
                        kind: CompletionKind::Operator,
                        insert_text: "(".to_string(),
                        primary_edit: None,
                        cursor: None,
                        additional_edits: Vec::new(),
                        detail: None,
                        is_disabled: false,
                        disabled_reason: None,
                        data: None,
                    }]
                } else {
                    items
                },
                replace: default_replace,
                signature_help: None,
            }
        }
        PropState::AfterPropLParen => {
            let items = ctx.map(prop_variable_items).unwrap_or_default();
            CompletionOutput {
                items: if items.is_empty() { vec![] } else { items },
                replace: default_replace,
                signature_help: None,
            }
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
                            primary_edit: None,
                            cursor: None,
                            additional_edits: Vec::new(),
                            detail: None,
                            is_disabled: prop.disabled_reason.is_some(),
                            disabled_reason: prop.disabled_reason.clone(),
                            data: Some(CompletionData::PropertyName {
                                name: prop.name.clone(),
                            }),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            CompletionOutput {
                items,
                replace: content_span,
                signature_help: None,
            }
        }
        PropState::None => {
            let (signature_help, expected_ty) =
                signature_help_at_cursor(tokens.as_slice(), cursor_u32, ctx);

            let prev_token = prev_non_trivia(tokens.as_slice(), cursor_u32);
            let prev_token = prev_token.map(|(_, token)| token);
            let is_prop_prefix = is_prop_prefix_at_cursor(tokens.as_slice(), cursor_u32);
            let mut items = expr_start_items(ctx);
            if !is_expr_start_position(prev_token)
                && !is_prop_prefix
                && !prefix_matches_completion(tokens.as_slice(), cursor_u32, &items)
            {
                CompletionOutput {
                    items: Vec::new(),
                    replace: default_replace,
                    signature_help,
                }
            } else {
                let replace = replace_span_for_expr_start(tokens.as_slice(), cursor_u32);
                if expected_ty.is_some() {
                    apply_type_ranking(&mut items, expected_ty, ctx);
                }
                CompletionOutput {
                    items,
                    replace,
                    signature_help,
                }
            }
        }
    };

    finalize_output(output, ctx)
}

fn prefix_matches_completion(tokens: &[Token], cursor: u32, items: &[CompletionItem]) -> bool {
    let Some(prefix) = ident_prefix_at_cursor(tokens, cursor) else {
        return false;
    };
    items.iter().any(|item| item.label.starts_with(prefix))
}

fn ident_prefix_at_cursor<'a>(tokens: &'a [Token], cursor: u32) -> Option<&'a str> {
    if let Some((_, token)) = token_containing_cursor(tokens, cursor) {
        if let TokenKind::Ident(ref symbol) = token.kind {
            return Some(symbol.text.as_str());
        }
    }
    if let Some((_, token)) = prev_non_trivia(tokens, cursor) {
        if token.span.end == cursor {
            if let TokenKind::Ident(ref symbol) = token.kind {
                return Some(symbol.text.as_str());
            }
        }
    }
    None
}

fn finalize_output(
    mut output: CompletionOutput,
    ctx: Option<&semantic::Context>,
) -> CompletionOutput {
    attach_primary_edits(output.replace, &mut output.items, ctx);
    output
}

fn attach_primary_edits(
    output_replace: Span,
    items: &mut [CompletionItem],
    _ctx: Option<&semantic::Context>,
) {
    for item in items {
        if item.is_disabled {
            item.primary_edit = None;
            item.cursor = None;
            continue;
        }

        item.primary_edit = Some(TextEdit {
            range: output_replace,
            new_text: item.insert_text.clone(),
        });

        item.cursor = match &item.data {
            Some(CompletionData::Function { .. }) => {
                // Function completions are expected to include `(` (e.g. `sum()`), so the editor
                // can place the cursor inside the parentheses. If the inserted text does not
                // contain `(`, we leave cursor placement to the default behavior.
                item.insert_text.find('(').map(|idx| {
                    output_replace
                        .start
                        .saturating_add((idx as u32).saturating_add(1))
                })
            }
            _ => None,
        };
    }
}

fn expr_start_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(ctx) = ctx {
        items.extend(prop_variable_items(ctx));
        items.extend(ctx.functions.iter().map(|func| {
            let detail = func.detail.clone().or_else(|| {
                Some(format!(
                    "{}({}) -> {}",
                    func.name,
                    format_param_list(&func.params),
                    format_ty(func.ret)
                ))
            });
            CompletionItem {
                label: func.name.clone(),
                kind: CompletionKind::Function,
                insert_text: format!("{}()", func.name),
                primary_edit: None,
                cursor: None,
                additional_edits: Vec::new(),
                detail,
                is_disabled: false,
                disabled_reason: None,
                data: Some(CompletionData::Function {
                    name: func.name.clone(),
                }),
            }
        }));
    }
    items.extend(vec![
        CompletionItem {
            label: "true".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "true".to_string(),
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: false,
            disabled_reason: None,
            data: None,
        },
        CompletionItem {
            label: "false".to_string(),
            kind: CompletionKind::Keyword,
            insert_text: "false".to_string(),
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: false,
            disabled_reason: None,
            data: None,
        },
        CompletionItem {
            label: "(".to_string(),
            kind: CompletionKind::Operator,
            insert_text: "(".to_string(),
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: false,
            disabled_reason: None,
            data: None,
        },
    ]);
    items
}

fn prop_variable_items(ctx: &semantic::Context) -> Vec<CompletionItem> {
    if ctx.properties.is_empty() {
        return Vec::new();
    }
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for prop in &ctx.properties {
        let label = format!("prop(\"{}\")", prop.name);
        let item = CompletionItem {
            label: label.clone(),
            kind: CompletionKind::Property,
            insert_text: label,
            primary_edit: None,
            cursor: None,
            additional_edits: Vec::new(),
            detail: None,
            is_disabled: prop.disabled_reason.is_some(),
            disabled_reason: prop.disabled_reason.clone(),
            data: Some(CompletionData::PropExpr {
                property_name: prop.name.clone(),
            }),
        };
        if prop.disabled_reason.is_some() {
            disabled.push(item);
        } else {
            enabled.push(item);
        }
    }
    enabled.extend(disabled);
    enabled
}

fn format_param_list(params: &[semantic::ParamSig]) -> String {
    params
        .iter()
        .map(|param| {
            let mut label = format_ty(param.ty).to_string();
            if param.optional {
                label.push('?');
            }
            label
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_ty(ty: semantic::Ty) -> &'static str {
    match ty {
        semantic::Ty::Number => "number",
        semantic::Ty::String => "string",
        semantic::Ty::Boolean => "boolean",
        semantic::Ty::Date => "date",
        semantic::Ty::Null => "null",
        semantic::Ty::Unknown => "unknown",
    }
}

fn ty_name(ty: semantic::Ty) -> &'static str {
    match ty {
        semantic::Ty::Number => "Number",
        semantic::Ty::String => "String",
        semantic::Ty::Boolean => "Boolean",
        semantic::Ty::Date => "Date",
        semantic::Ty::Null => "Null",
        semantic::Ty::Unknown => "Any",
    }
}

fn format_signature(sig: &semantic::FunctionSig) -> (String, Vec<String>) {
    let params = sig
        .params
        .iter()
        .map(|param| {
            let mut ty = ty_name(param.ty).to_string();
            if param.optional {
                ty.push('?');
            }
            if let Some(name) = &param.name {
                format!("{name}: {ty}")
            } else {
                ty
            }
        })
        .collect::<Vec<_>>();
    let label = format!(
        "{}({}) -> {}",
        sig.name,
        params.join(", "),
        ty_name(sig.ret)
    );
    (label, params)
}

fn signature_help_at_cursor(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
) -> (Option<SignatureHelp>, Option<semantic::Ty>) {
    let ctx = match ctx {
        Some(ctx) => ctx,
        None => return (None, None),
    };
    let call_ctx = match detect_call_context(tokens, cursor) {
        Some(call_ctx) => call_ctx,
        None => return (None, None),
    };
    let func = match ctx
        .functions
        .iter()
        .find(|func| func.name == call_ctx.callee)
    {
        Some(func) => func,
        None => return (None, None),
    };
    let (label, params) = format_signature(func);
    let active_param = if params.is_empty() {
        0
    } else {
        call_ctx.arg_index.min(params.len() - 1)
    };
    let expected_ty = func.params.get(active_param).map(|param| param.ty);
    (
        Some(SignatureHelp {
            label,
            params,
            active_param,
        }),
        expected_ty,
    )
}

fn detect_call_context(tokens: &[Token], cursor: u32) -> Option<CallContext> {
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
    let mut depth = 0i32;
    for token in tokens.iter().skip(lparen_idx + 1) {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        if token.span.start >= cursor {
            break;
        }
        match token.kind {
            TokenKind::OpenParen => depth += 1,
            TokenKind::CloseParen => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            TokenKind::Comma => {
                if depth == 0 {
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

fn apply_type_ranking(
    items: &mut Vec<CompletionItem>,
    expected_ty: Option<semantic::Ty>,
    ctx: Option<&semantic::Context>,
) {
    let expected_ty = match expected_ty {
        Some(expected_ty) => expected_ty,
        None => return,
    };
    let mut scored = items
        .drain(..)
        .enumerate()
        .map(|(idx, item)| {
            let actual = item_result_ty(&item, ctx);
            let score = type_match_score(expected_ty, actual);
            (idx, item, score)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(a_idx, a_item, a_score), (b_idx, b_item, b_score)| {
        let a_key = (a_item.is_disabled, -a_score, *a_idx as i32);
        let b_key = (b_item.is_disabled, -b_score, *b_idx as i32);
        a_key.cmp(&b_key)
    });
    items.extend(scored.into_iter().map(|(_, item, _)| item));
}

fn item_result_ty(item: &CompletionItem, ctx: Option<&semantic::Context>) -> Option<semantic::Ty> {
    if let Some(data) = &item.data {
        let ctx = ctx?;
        return match data {
            CompletionData::Function { name } => ctx
                .functions
                .iter()
                .find(|func| func.name == *name)
                .map(|func| func.ret),
            CompletionData::PropertyName { name } => ctx.lookup(name),
            CompletionData::PropExpr { property_name } => ctx.lookup(property_name),
        };
    }

    match item.kind {
        CompletionKind::Keyword => match item.label.as_str() {
            "true" | "false" => Some(semantic::Ty::Boolean),
            _ => None,
        },
        _ => None,
    }
}

fn type_match_score(expected: semantic::Ty, actual: Option<semantic::Ty>) -> i32 {
    if expected == semantic::Ty::Unknown {
        return 1;
    }
    match actual {
        Some(actual_ty) if actual_ty == semantic::Ty::Unknown => 0,
        Some(actual_ty) if actual_ty == expected => 2,
        Some(_) => -1,
        None => 0,
    }
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

fn replace_span_for_expr_start(tokens: &[Token], cursor: u32) -> Span {
    if let Some((_, token)) = token_containing_cursor(tokens, cursor) {
        if matches!(token.kind, TokenKind::Ident(_)) {
            return token.span;
        }
    }
    if let Some((_, token)) = prev_non_trivia(tokens, cursor) {
        if matches!(token.kind, TokenKind::Ident(_)) && token.span.end == cursor {
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

fn is_prop_prefix_at_cursor(tokens: &[Token], cursor: u32) -> bool {
    if let Some((_, token)) = prev_non_trivia(tokens, cursor) {
        if let TokenKind::Ident(ref symbol) = token.kind {
            if token.span.end == cursor {
                return "prop".starts_with(symbol.text.as_str());
            }
        }
    }
    false
}

fn string_content_span(span: Span) -> Span {
    let start = span.start.saturating_add(1);
    let end = span.end.saturating_sub(1);
    Span { start, end }
}

fn span_contains_cursor_inclusive(span: Span, cursor: u32) -> bool {
    span.start <= cursor && cursor <= span.end
}

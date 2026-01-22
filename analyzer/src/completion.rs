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
    Builtin,
    Property,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionData {
    Function { name: String },
    PropExpr { property_name: String },
    PostfixMethod { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub label: String,
    pub params: Vec<String>,
    pub active_param: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallContext {
    callee: String,
    lparen_idx: usize,
    arg_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionKind {
    NeedExpr,
    AfterAtom,
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

    let call_ctx = detect_call_context(tokens.as_slice(), cursor_u32);
    let signature_help =
        compute_signature_help_if_in_call(tokens.as_slice(), cursor_u32, ctx, call_ctx.as_ref());
    let in_string = cursor_strictly_inside_string_literal(tokens.as_slice(), cursor_u32);
    let position_kind = if in_string {
        PositionKind::None
    } else {
        detect_position_kind(tokens.as_slice(), cursor_u32, ctx, call_ctx.as_ref())
    };

    let mut output = complete_for_position(
        position_kind,
        ctx,
        tokens.as_slice(),
        cursor_u32,
        call_ctx.as_ref(),
    );
    output.signature_help = signature_help;

    finalize_output(output, ctx)
}

fn detect_position_kind(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
    call_ctx: Option<&CallContext>,
) -> PositionKind {
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

    // AfterAtom only makes sense if we're in an expression (either top-level or inside a call).
    // Call context is computed separately; we treat both cases the same for completion contents.
    let _ = call_ctx;

    match prev.map(|token| &token.kind) {
        Some(TokenKind::Ident(_)) | Some(TokenKind::Literal(_)) | Some(TokenKind::CloseParen) => {
            PositionKind::AfterAtom
        }
        _ => PositionKind::None,
    }
}

fn is_strictly_inside_ident(tokens: &[Token], cursor: u32) -> bool {
    let Some((_, token)) = token_containing_cursor(tokens, cursor) else {
        return false;
    };
    matches!(token.kind, TokenKind::Ident(_)) && token.span.start < cursor && cursor < token.span.end
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

fn has_extending_ident_prefix(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
) -> bool {
    let Some((_, token)) = prev_non_trivia(tokens, cursor) else {
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

    if "true".starts_with(prefix) && prefix != "true" {
        return true;
    }
    if "false".starts_with(prefix) && prefix != "false" {
        return true;
    }
    if "not".starts_with(prefix) && prefix != "not" {
        return true;
    }

    let Some(ctx) = ctx else {
        return false;
    };

    if ctx
        .functions
        .iter()
        .any(|func| func.name.starts_with(prefix) && func.name != prefix)
    {
        return true;
    }

    if ctx
        .properties
        .iter()
        .any(|prop| prop.name.starts_with(prefix) && prop.name != prefix)
    {
        return true;
    }

    false
}

fn complete_for_position(
    kind: PositionKind,
    ctx: Option<&semantic::Context>,
    tokens: &[Token],
    cursor: u32,
    call_ctx: Option<&CallContext>,
) -> CompletionOutput {
    let default_replace = Span {
        start: cursor,
        end: cursor,
    };
    match kind {
        PositionKind::NeedExpr => {
            let expected = expected_call_arg_ty(call_ctx, ctx);
            let mut items = expr_start_items(ctx);
            if expected.is_some() {
                apply_type_ranking(&mut items, expected, ctx);
            }
            CompletionOutput {
                items,
                replace: replace_span_for_expr_start(tokens, cursor),
                signature_help: None,
            }
        }
        PositionKind::AfterAtom => CompletionOutput {
            items: after_atom_items(ctx),
            replace: default_replace,
            signature_help: None,
        },
        PositionKind::None => CompletionOutput {
            items: Vec::new(),
            replace: default_replace,
            signature_help: None,
        },
    }
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
            Some(CompletionData::PropExpr { .. }) => {
                // Property completions insert the full expression (e.g., `prop("Title")`).
                // Place the cursor at the end of the inserted text.
                Some(
                    output_replace
                        .start
                        .saturating_add(item.insert_text.len() as u32),
                )
            }
            Some(CompletionData::PostfixMethod { .. }) => item.insert_text.find('(').map(|idx| {
                output_replace
                    .start
                    .saturating_add((idx as u32).saturating_add(1))
            }),
            _ => None,
        };
    }
}

fn expr_start_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(ctx) = ctx {
        items.extend(prop_variable_items(ctx));
        items.extend(builtin_expr_start_items());
        items.extend(ctx.functions.iter().map(|func| {
            let detail = func.detail.clone().or_else(|| {
                Some(format!(
                    "{}({}) -> {}",
                    func.name,
                    format_param_list(&func.params),
                    format_ty(&func.ret)
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
    } else {
        items.extend(builtin_expr_start_items());
    }
    items
}

fn builtin_expr_start_items() -> Vec<CompletionItem> {
    [
        ("not", "not"),
        ("true", "true"),
        ("false", "false"),
    ]
    .into_iter()
    .map(|(label, insert_text)| CompletionItem {
        label: label.to_string(),
        kind: CompletionKind::Builtin,
        insert_text: insert_text.to_string(),
        primary_edit: None,
        cursor: None,
        additional_edits: Vec::new(),
        detail: None,
        is_disabled: false,
        disabled_reason: None,
        data: None,
    })
    .collect()
}

fn after_atom_items(ctx: Option<&semantic::Context>) -> Vec<CompletionItem> {
    const OPS: [&str; 10] = ["==", "!=", ">=", ">", "<=", "<", "+", "-", "*", "/"];

    let mut items = Vec::new();
    items.extend(OPS.into_iter().map(|op| CompletionItem {
        label: op.to_string(),
        kind: CompletionKind::Operator,
        insert_text: op.to_string(),
        primary_edit: None,
        cursor: None,
        additional_edits: Vec::new(),
        detail: None,
        is_disabled: false,
        disabled_reason: None,
        data: None,
    }));

    if let Some(ctx) = ctx {
        if ctx.functions.iter().any(|f| f.name == "if") {
            items.push(CompletionItem {
                label: ".if".to_string(),
                kind: CompletionKind::Operator,
                insert_text: ".if()".to_string(),
                primary_edit: None,
                cursor: None,
                additional_edits: Vec::new(),
                detail: None,
                is_disabled: false,
                disabled_reason: None,
                data: Some(CompletionData::PostfixMethod {
                    name: "if".to_string(),
                }),
            });
        }
    }

    items
}

fn prop_variable_items(ctx: &semantic::Context) -> Vec<CompletionItem> {
    if ctx.properties.is_empty() {
        return Vec::new();
    }
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    for prop in &ctx.properties {
        let label = prop.name.clone();
        let insert_text = format!(r#"prop("{}")"#, prop.name);
        let item = CompletionItem {
            label,
            kind: CompletionKind::Property,
            insert_text,
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
            let mut label = format_ty(&param.ty);
            if param.optional {
                label.push('?');
            }
            label
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_ty(ty: &semantic::Ty) -> String {
    match ty {
        semantic::Ty::Number => "number".into(),
        semantic::Ty::String => "string".into(),
        semantic::Ty::Boolean => "boolean".into(),
        semantic::Ty::Date => "date".into(),
        semantic::Ty::Null => "null".into(),
        semantic::Ty::Unknown => "unknown".into(),
        semantic::Ty::List(inner) => format!("{}[]", format_ty(inner)),
        semantic::Ty::Union(types) => types
            .iter()
            .map(format_ty)
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

fn format_signature(sig: &semantic::FunctionSig) -> (String, Vec<String>) {
    let params = sig
        .params
        .iter()
        .map(|param| {
            let mut ty = format_ty(&param.ty);
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
    let mut label_params = params.join(", ");
    if sig.is_variadic() {
        if !label_params.is_empty() {
            label_params.push_str(", ");
        }
        label_params.push_str("...");
    }
    let label = format!(
        "{}({}) -> {}",
        sig.name,
        label_params,
        format_ty(&sig.ret)
    );
    (label, params)
}

/// Only compute signature help if the cursor is inside a function call argument context
/// (i.e., after the opening parenthesis).
fn compute_signature_help_if_in_call(
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
    call_ctx: Option<&CallContext>,
) -> Option<SignatureHelp> {
    let call_ctx = call_ctx?;
    let lparen_token = tokens.get(call_ctx.lparen_idx)?;

    // Only show signature help if cursor is after the '(' (inside the call)
    if cursor < lparen_token.span.end {
        return None;
    }

    let ctx = ctx?;
    let func = ctx
        .functions
        .iter()
        .find(|func| func.name == call_ctx.callee)?;

    let (label, params) = format_signature(func);
    let active_param = if params.is_empty() {
        0
    } else {
        call_ctx.arg_index.min(params.len() - 1)
    };

    Some(SignatureHelp {
        label,
        params,
        active_param,
    })
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

fn expected_call_arg_ty(
    call_ctx: Option<&CallContext>,
    ctx: Option<&semantic::Context>,
) -> Option<semantic::Ty> {
    let call_ctx = call_ctx?;
    let ctx = ctx?;
    ctx.functions
        .iter()
        .find(|func| func.name == call_ctx.callee)
        .and_then(|func| func.param_for_arg_index(call_ctx.arg_index))
        .map(|param| param.ty.clone())
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
            let score = type_match_score(expected_ty.clone(), actual);
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
                .map(|func| func.ret.clone()),
            CompletionData::PropExpr { property_name } => ctx.lookup(property_name),
            CompletionData::PostfixMethod { .. } => None,
        };
    }

    match item.kind {
        CompletionKind::Builtin => match item.label.as_str() {
            "true" | "false" | "not" => Some(semantic::Ty::Boolean),
            _ => None,
        },
        _ => None,
    }
}

fn type_match_score(expected: semantic::Ty, actual: Option<semantic::Ty>) -> i32 {
    if matches!(expected, semantic::Ty::Unknown) {
        return 1;
    }
    match actual {
        Some(actual_ty) if matches!(actual_ty, semantic::Ty::Unknown) => 0,
        Some(actual_ty) if semantic::ty_accepts(&expected, &actual_ty) => 2,
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

/// Like `prev_non_trivia`, but treats a cursor at a token boundary (`cursor == token.span.start`)
/// as an insertion point *before* that token.
///
/// This prevents `)` from being treated as the "previous" token when completing immediately
/// before a close-paren, while still treating a cursor strictly inside a token as "within" it.
fn prev_non_trivia_insertion(tokens: &[Token], cursor: u32) -> Option<(usize, &Token)> {
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor) {
        if token.span.start < cursor && !token.is_trivia() && !matches!(token.kind, TokenKind::Eof)
        {
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
    if let Some((idx, token)) = token_containing_cursor(tokens, cursor) {
        if matches!(token.kind, TokenKind::Ident(_)) {
            // At an expr-start position, completing before an existing expression should insert
            // instead of replacing tokens to the right.
            if cursor == token.span.start {
                return Span {
                    start: cursor,
                    end: cursor,
                };
            }

            // If the cursor is actually inside the identifier token, treat it as prefix editing.
            return tokens[idx].span;
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

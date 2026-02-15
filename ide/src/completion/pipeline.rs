//! Completion pipeline entry point.
//! Works in UTF-8 byte offsets (cursor and edit ranges).

use super::position::PositionKind;
use super::{CompletionConfig, CompletionOutput};
use analyzer::semantic;
use analyzer::{Span, Token, TokenKind};

/// Computes completion output for a single cursor position.
pub(super) fn complete(
    text: &str,
    cursor: usize,
    ctx: Option<&semantic::Context>,
    config: CompletionConfig,
) -> CompletionOutput {
    let cursor_u32 = u32::try_from(cursor).unwrap_or(u32::MAX);
    let tokens = analyzer::analyze_syntax(text).tokens;

    let default_replace = Span {
        start: cursor_u32,
        end: cursor_u32,
    };

    if tokens
        .iter()
        .all(|token| matches!(token.kind, TokenKind::Eof))
    {
        let items = if cursor == 0 {
            super::items::expr_start_items(ctx)
        } else {
            Vec::new()
        };
        return super::rank::finalize_output(
            text,
            CompletionOutput {
                items,
                replace: default_replace,
                signature_help: None,
                preferred_indices: Vec::new(),
            },
            config,
            PositionKind::NeedExpr,
        );
    }

    let call_ctx = super::signature::detect_call_context(tokens.as_slice(), cursor_u32);
    let signature_help = super::signature::compute_signature_help_if_in_call(
        text,
        tokens.as_slice(),
        cursor_u32,
        ctx,
        call_ctx.as_ref(),
    );
    let position_kind =
        if super::position::cursor_strictly_inside_string_literal(tokens.as_slice(), cursor_u32) {
            PositionKind::None
        } else {
            super::position::detect_position_kind(tokens.as_slice(), cursor_u32, ctx)
        };

    let mut output = complete_for_position(
        text,
        position_kind,
        ctx,
        tokens.as_slice(),
        cursor_u32,
        call_ctx.as_ref(),
    );
    output.signature_help = signature_help;
    super::rank::finalize_output(text, output, config, position_kind)
}

fn complete_for_position(
    text: &str,
    kind: PositionKind,
    ctx: Option<&semantic::Context>,
    tokens: &[Token],
    cursor: u32,
    call_ctx: Option<&super::signature::CallContext>,
) -> CompletionOutput {
    let default_replace = Span {
        start: cursor,
        end: cursor,
    };
    match kind {
        PositionKind::NeedExpr => {
            let expected = super::signature::expected_call_arg_ty(call_ctx, ctx);
            let mut items = super::items::expr_start_items(ctx);
            if expected.is_some() {
                super::rank::apply_type_ranking(&mut items, expected, ctx);
            }
            CompletionOutput {
                items,
                replace: super::position::replace_span_for_expr_start(tokens, cursor),
                signature_help: None,
                preferred_indices: Vec::new(),
            }
        }
        PositionKind::AfterAtom => CompletionOutput {
            items: super::items::after_atom_items(ctx),
            replace: default_replace,
            signature_help: None,
            preferred_indices: Vec::new(),
        },
        PositionKind::AfterDot => CompletionOutput {
            items: super::items::after_dot_items(
                ctx,
                &infer_postfix_receiver_ty(text, tokens, cursor, ctx),
            ),
            replace: super::position::replace_span_for_expr_start(tokens, cursor),
            signature_help: None,
            preferred_indices: Vec::new(),
        },
        PositionKind::None => CompletionOutput {
            items: Vec::new(),
            replace: default_replace,
            signature_help: None,
            preferred_indices: Vec::new(),
        },
    }
}

fn infer_postfix_receiver_ty(
    text: &str,
    tokens: &[Token],
    cursor: u32,
    ctx: Option<&semantic::Context>,
) -> semantic::Ty {
    let Some(ctx) = ctx else {
        return semantic::Ty::Unknown;
    };

    let Some(dot_idx) = super::position::postfix_member_access_dot_index(tokens, cursor) else {
        return semantic::Ty::Unknown;
    };
    let Some(dot_token) = tokens.get(dot_idx) else {
        return semantic::Ty::Unknown;
    };
    let Ok(dot_start) = usize::try_from(dot_token.span.start) else {
        return semantic::Ty::Unknown;
    };
    if dot_start > text.len() || !text.is_char_boundary(dot_start) {
        return semantic::Ty::Unknown;
    }

    let receiver_source = text[..dot_start].trim_end();
    if receiver_source.is_empty() {
        return semantic::Ty::Unknown;
    }

    let parsed = analyzer::analyze_syntax(receiver_source);

    let mut map = analyzer::TypeMap::default();
    analyzer::infer_expr_with_map(&parsed.expr, ctx, &mut map)
}

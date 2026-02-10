//! Completion pipeline entry point.
//! Works in UTF-8 byte offsets (cursor and edit ranges).

use super::position::PositionKind;
use super::{CompletionConfig, CompletionOutput};
use crate::lexer::lex;
use crate::lexer::{Span, Token, TokenKind};
use crate::semantic;

/// Computes completion output for a single cursor position.
pub(super) fn complete(
    text: &str,
    cursor: usize,
    ctx: Option<&semantic::Context>,
    config: CompletionConfig,
) -> CompletionOutput {
    let cursor_u32 = u32::try_from(cursor).unwrap_or(u32::MAX);
    let tokens = lex(text).tokens;

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
            items: super::items::after_dot_items(ctx),
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

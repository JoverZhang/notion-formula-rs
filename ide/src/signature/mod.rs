//! Signature help for calls under the cursor.
//! Uses UTF-8 byte offsets (via tokens/spans) and best-effort type inference.
//!
//! Sub-modules:
//! - [`generics`]: Generic substitution / unification.
//! - [`param_shape`]: Repeat-parameter shape resolution and active-parameter mapping.
//! - [`render`]: Signature rendering into display slots.

mod generics;
mod param_shape;
mod render;

use crate::context::{CallContext, prev_non_trivia_before};
use crate::display::build_signature_segments;
use analyzer::ast::{Expr, ExprKind};
use analyzer::semantic;
use analyzer::{Token, TokenKind};

use generics::instantiate_sig;
use param_shape::active_parameter_for_call;
use render::render_signature;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureItem {
    pub segments: Vec<crate::display::DisplaySegment>,
}

/// Signature display for a call at the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureItem>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

/// Computes signature help when the cursor is inside a call argument list.
///
/// Returns `None` if the cursor is before the `(`, or if the callee is unknown.
pub(crate) fn compute_signature_help_if_in_call(
    source: &str,
    tokens: &[Token],
    cursor: u32,
    ctx: &semantic::Context,
    call_ctx: Option<&CallContext>,
) -> Option<SignatureHelp> {
    let call_ctx = call_ctx?;
    let lparen_token = tokens.get(call_ctx.lparen_idx)?;

    // Only show signature help if cursor is after the '(' (inside the call)
    if cursor < lparen_token.span.end {
        return None;
    }

    let func = ctx
        .functions
        .iter()
        .find(|func| func.name == call_ctx.callee)?;

    let is_postfix_call = || {
        let (callee_idx, callee_token) = prev_non_trivia_before(tokens, call_ctx.lparen_idx)?;
        if !matches!(callee_token.kind, TokenKind::Ident(_)) {
            return None;
        }
        let (dot_idx, dot_token) = prev_non_trivia_before(tokens, callee_idx)?;
        if !matches!(dot_token.kind, TokenKind::Dot) {
            return None;
        }
        let (_, receiver_token) = prev_non_trivia_before(tokens, dot_idx)?;
        matches!(
            receiver_token.kind,
            TokenKind::Ident(_) | TokenKind::Literal(_) | TokenKind::CloseParen
        )
        .then_some(())
    };

    let is_postfix_call = is_postfix_call().is_some();
    let is_method_style = is_postfix_call
        && semantic::postfix_capable_builtin_names().contains(func.name.as_str())
        && semantic::is_postfix_capable(func);

    let arg_tys = infer_call_arg_tys_best_effort(source, tokens, ctx, call_ctx, is_method_style);
    let (inst_param_tys, inst_ret) = instantiate_sig(func, arg_tys.as_slice());

    // `receiver.fn(arg1, ...)` is treated as `fn(receiver, arg1, ...)` internally.
    let arg_index_full = call_ctx.arg_index.saturating_add(is_method_style as usize);
    let total_args_for_shape = arg_tys.len().max(arg_index_full + 1);
    let rendered = render_signature(
        func,
        arg_tys.as_slice(),
        total_args_for_shape,
        inst_param_tys.as_slice(),
        is_method_style,
    );

    let active_parameter =
        active_parameter_for_call(func, arg_index_full, total_args_for_shape, is_method_style);
    let segments =
        build_signature_segments(func.name.as_str(), &rendered, &inst_ret, is_method_style);

    Some(SignatureHelp {
        signatures: vec![SignatureItem { segments }],
        active_signature: 0,
        active_parameter,
    })
}

// ---------------------------------------------------------------------------
// Call-site argument type inference (stays in mod.rs – uses AST + tokens)
// ---------------------------------------------------------------------------

fn find_call_expr_by_lparen<'a>(
    root: &'a Expr,
    callee: &str,
    lparen_start: u32,
) -> Option<&'a Expr> {
    fn visit<'a>(expr: &'a Expr, callee: &str, lparen_start: u32, best: &mut Option<&'a Expr>) {
        let mut visit_child = |child: &'a Expr| visit(child, callee, lparen_start, best);

        match &expr.kind {
            ExprKind::Group { inner } => visit_child(inner),
            ExprKind::List { items } => {
                for item in items {
                    visit_child(item);
                }
            }
            ExprKind::Unary { expr: inner, .. } => visit_child(inner),
            ExprKind::Binary { left, right, .. } => {
                visit_child(left);
                visit_child(right);
            }
            ExprKind::Ternary {
                cond,
                then,
                otherwise,
            } => {
                visit_child(cond);
                visit_child(then);
                visit_child(otherwise);
            }
            ExprKind::Call { args, .. } => {
                for a in args {
                    visit_child(a);
                }
            }
            ExprKind::MemberCall { receiver, args, .. } => {
                visit_child(receiver);
                for a in args {
                    visit_child(a);
                }
            }
            ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => {}
        }

        let matches_call = match &expr.kind {
            ExprKind::Call { callee: c, .. } => c.text == callee,
            ExprKind::MemberCall { method, .. } => method.text == callee,
            _ => false,
        };

        if matches_call && expr.span.start <= lparen_start && lparen_start < expr.span.end {
            match best {
                None => *best = Some(expr),
                Some(prev) => {
                    let prev_len = prev.span.end.saturating_sub(prev.span.start);
                    let cur_len = expr.span.end.saturating_sub(expr.span.start);
                    if cur_len <= prev_len {
                        *best = Some(expr);
                    }
                }
            }
        }
    }

    let mut best = None;
    visit(root, callee, lparen_start, &mut best);
    best
}

/// Infers the type of a single argument expression fragment.
fn infer_one_arg(expr_source: &str, ctx: &semantic::Context) -> Option<semantic::Ty> {
    let trimmed = expr_source.trim();
    if trimmed.is_empty() {
        return None;
    }

    match trimmed {
        "true" | "false" => return Some(semantic::Ty::Boolean),
        _ => {}
    }

    let parsed = analyzer::analyze_syntax(trimmed);
    let mut map = analyzer::TypeMap::default();
    Some(analyzer::infer_expr_with_map(&parsed.expr, ctx, &mut map))
}

/// Splits the token stream after `lparen_idx` into per-argument byte spans,
/// respecting nested parentheses and brackets.
fn arg_spans(tokens: &[Token], lparen_idx: usize, source_len: u32) -> Vec<analyzer::Span> {
    let mut spans = Vec::<analyzer::Span>::new();
    let Some(lparen) = tokens.get(lparen_idx) else {
        return spans;
    };

    let mut paren_depth = 0i32;
    let mut bracket_depth = 0i32;
    let mut start = lparen.span.end;

    for token in tokens.iter().skip(lparen_idx + 1) {
        if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
            continue;
        }

        match token.kind {
            TokenKind::OpenParen => paren_depth += 1,
            TokenKind::OpenBracket => bracket_depth += 1,
            TokenKind::CloseParen => {
                if paren_depth == 0 && bracket_depth == 0 {
                    spans.push(analyzer::Span {
                        start,
                        end: token.span.start,
                    });
                    return spans;
                }
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
            }
            TokenKind::CloseBracket => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            TokenKind::Comma if paren_depth == 0 && bracket_depth == 0 => {
                spans.push(analyzer::Span {
                    start,
                    end: token.span.start,
                });
                start = token.span.end;
            }
            _ => {}
        }
    }

    spans.push(analyzer::Span {
        start,
        end: source_len,
    });
    spans
}

fn infer_call_arg_tys_best_effort(
    source: &str,
    tokens: &[Token],
    ctx: &semantic::Context,
    call_ctx: &CallContext,
    include_receiver_as_arg: bool,
) -> Vec<Option<semantic::Ty>> {
    let Some(lparen_token) = tokens.get(call_ctx.lparen_idx) else {
        return Vec::new();
    };

    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let spans = arg_spans(tokens, call_ctx.lparen_idx, source_len);

    let mut arg_tys: Vec<Option<semantic::Ty>> = Vec::new();

    // If this is a member call, try to include the receiver type as the leading argument.
    if include_receiver_as_arg {
        let parsed = analyzer::analyze_syntax(source);
        if let Some(call_expr) =
            find_call_expr_by_lparen(&parsed.expr, &call_ctx.callee, lparen_token.span.start)
            && let ExprKind::MemberCall { receiver, .. } = &call_expr.kind
        {
            let mut map = analyzer::TypeMap::default();
            let _ = analyzer::infer_expr_with_map(&parsed.expr, ctx, &mut map);
            let mut ty = map
                .get(receiver.id)
                .cloned()
                .unwrap_or(semantic::Ty::Unknown);
            if matches!(ty, semantic::Ty::Unknown)
                && matches!(
                    &receiver.kind,
                    ExprKind::Ident(sym) if sym.text == "true" || sym.text == "false"
                )
            {
                ty = semantic::Ty::Boolean;
            }
            arg_tys.push(Some(ty));
        }
    }

    for span in spans {
        let start = span.start as usize;
        let end = span.end as usize;
        if start > source.len() || end > source.len() || start > end {
            continue;
        }
        let frag = &source[start..end];
        arg_tys.push(infer_one_arg(frag, ctx));
    }

    arg_tys
}

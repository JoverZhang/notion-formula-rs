//! Signature help for calls under the cursor.
//! Uses UTF-8 byte offsets (via tokens/spans) and best-effort type inference.

use super::SignatureHelp;
use super::position::prev_non_trivia_before;
use crate::ast::{Expr, ExprKind};
use crate::ide::completion::SignatureItem;
use crate::ide::display::{ParamSlot, RenderedSignature, build_signature_segments};
use crate::lexer::{Token, TokenKind};
use crate::semantic;

/// Call-site info derived from tokens, using byte offsets for the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CallContext {
    pub(super) callee: String,
    pub(super) lparen_idx: usize,
    pub(super) arg_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepeatShapeInfo {
    repeat_groups: usize,
    tail_start: usize,
}

fn repeat_shape_info(
    sig: &semantic::FunctionSig,
    total_args_for_shape: usize,
) -> Option<RepeatShapeInfo> {
    let shape = semantic::complete_repeat_shape(&sig.params, total_args_for_shape)?;
    Some(RepeatShapeInfo {
        repeat_groups: shape.repeat_groups.max(1),
        tail_start: shape.tail_start,
    })
}

fn ty_contains_generic(ty: &semantic::Ty) -> bool {
    match ty {
        semantic::Ty::Generic(_) => true,
        semantic::Ty::List(inner) => ty_contains_generic(inner),
        semantic::Ty::Union(members) => members.iter().any(ty_contains_generic),
        _ => false,
    }
}

fn format_ty_with_optional(ty: &semantic::Ty, optional: bool) -> String {
    let mut out = ty.to_string();
    if optional {
        out.push('?');
    }
    out
}

fn choose_display_ty<'a>(
    actual: Option<&'a semantic::Ty>,
    declared_template: &'a semantic::Ty,
    instantiated_expected: &'a semantic::Ty,
) -> &'a semantic::Ty {
    if !ty_contains_generic(declared_template) {
        return instantiated_expected;
    }
    actual.unwrap_or(instantiated_expected)
}

fn render_signature(
    sig: &semantic::FunctionSig,
    arg_tys: &[Option<semantic::Ty>],
    total_args_for_shape: usize,
    inst_param_tys: &[semantic::Ty],
    is_method_style: bool,
) -> RenderedSignature {
    let mut receiver: Option<(String, String)> = None;
    let mut slots = Vec::<ParamSlot>::new();
    let mut next_param_index = 0u32;

    fn push_param(
        receiver: &mut Option<(String, String)>,
        slots: &mut Vec<ParamSlot>,
        next_param_index: &mut u32,
        is_method_style: bool,
        name: String,
        ty: String,
    ) {
        if is_method_style && receiver.is_none() {
            *receiver = Some((name, ty));
            return;
        }

        let idx = *next_param_index;
        *next_param_index += 1;
        slots.push(ParamSlot::Param {
            name,
            ty,
            param_index: idx,
        });
    }

    fn push_ellipsis(slots: &mut Vec<ParamSlot>) {
        slots.push(ParamSlot::Ellipsis);
    }

    if sig.params.repeat.is_empty() {
        let mut idx = 0usize;
        for p in sig.params.head.iter().chain(sig.params.tail.iter()) {
            let instantiated_expected = inst_param_tys.get(idx).unwrap_or(&p.ty);
            let actual = arg_tys.get(idx).and_then(|t| t.as_ref());
            let ty = choose_display_ty(actual, &p.ty, instantiated_expected);
            idx += 1;
            push_param(
                &mut receiver,
                &mut slots,
                &mut next_param_index,
                is_method_style,
                p.name.clone(),
                format_ty_with_optional(ty, p.optional),
            );
        }
        return RenderedSignature { receiver, slots };
    }

    for (idx, p) in sig.params.head.iter().enumerate() {
        let ty = inst_param_tys.get(idx).unwrap_or(&p.ty);
        push_param(
            &mut receiver,
            &mut slots,
            &mut next_param_index,
            is_method_style,
            p.name.clone(),
            format_ty_with_optional(ty, p.optional),
        );
    }

    // Show the repeat pattern for each entered repeat group (numbered), then an ellipsis, then the tail.
    let repeat_start = sig.params.head.len();
    let repeat_len = sig.params.repeat.len();

    let shape = repeat_shape_info(sig, total_args_for_shape);
    let repeat_groups_displayed = shape.map(|s| s.repeat_groups).unwrap_or(1);
    let tail_start = shape.map(|s| s.tail_start).unwrap_or(usize::MAX);

    for n in 1..=repeat_groups_displayed {
        for (r_idx, p) in sig.params.repeat.iter().enumerate() {
            let name = format!("{}{}", p.name, n);
            let cycle = n - 1;
            let actual_idx = repeat_start + cycle * repeat_len + r_idx;
            let inst_idx = repeat_start + r_idx;
            let instantiated_expected = inst_param_tys.get(inst_idx).unwrap_or(&p.ty);
            let actual = arg_tys.get(actual_idx).and_then(|t| t.as_ref());
            let ty = choose_display_ty(actual, &p.ty, instantiated_expected);
            push_param(
                &mut receiver,
                &mut slots,
                &mut next_param_index,
                is_method_style,
                name,
                format_ty_with_optional(ty, p.optional),
            );
        }
    }
    push_ellipsis(&mut slots);
    for (t_idx, p) in sig.params.tail.iter().enumerate() {
        let actual_idx = tail_start.saturating_add(t_idx);
        let inst_idx = repeat_start + repeat_len + t_idx;
        let instantiated_expected = inst_param_tys.get(inst_idx).unwrap_or(&p.ty);
        let actual = arg_tys.get(actual_idx).and_then(|t| t.as_ref());
        let ty = choose_display_ty(actual, &p.ty, instantiated_expected);
        push_param(
            &mut receiver,
            &mut slots,
            &mut next_param_index,
            is_method_style,
            p.name.clone(),
            format_ty_with_optional(ty, p.optional),
        );
    }

    RenderedSignature { receiver, slots }
}

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

    fn infer_one_arg(expr_source: &str, ctx: &semantic::Context) -> Option<semantic::Ty> {
        let trimmed = expr_source.trim();
        if trimmed.is_empty() {
            return None;
        }

        match trimmed {
            "true" | "false" => {
                return Some(semantic::Ty::Boolean);
            }
            _ => {}
        }

        let Ok(parsed) = crate::analyze(trimmed) else {
            return Some(semantic::Ty::Unknown);
        };

        let mut map = semantic::TypeMap::default();
        Some(semantic::infer_expr_with_map(&parsed.expr, ctx, &mut map))
    }

    fn arg_spans(tokens: &[Token], lparen_idx: usize, source_len: u32) -> Vec<crate::Span> {
        let mut spans = Vec::<crate::Span>::new();
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
                        spans.push(crate::Span {
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
                    spans.push(crate::Span {
                        start,
                        end: token.span.start,
                    });
                    start = token.span.end;
                }
                _ => {}
            }
        }

        spans.push(crate::Span {
            start,
            end: source_len,
        });
        spans
    }

    let source_len = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let spans = arg_spans(tokens, call_ctx.lparen_idx, source_len);

    let mut arg_tys: Vec<Option<semantic::Ty>> = Vec::new();

    // If this is a member call, try to include the receiver type as the leading argument.
    if include_receiver_as_arg
        && let Ok(parsed) = crate::analyze(source)
        && let Some(call_expr) =
            find_call_expr_by_lparen(&parsed.expr, &call_ctx.callee, lparen_token.span.start)
        && let ExprKind::MemberCall { receiver, .. } = &call_expr.kind
    {
        let mut map = semantic::TypeMap::default();
        let _ = semantic::infer_expr_with_map(&parsed.expr, ctx, &mut map);
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

fn active_param_index(
    sig: &semantic::FunctionSig,
    call_ctx: &CallContext,
    total_args_for_shape: usize,
) -> usize {
    if sig.params.repeat.is_empty() {
        let total_params = sig.params.head.len() + sig.params.tail.len();
        if total_params == 0 {
            return 0;
        }
        return call_ctx.arg_index.min(total_params - 1);
    }

    let head_len = sig.params.head.len();
    let repeat_len = sig.params.repeat.len();
    let tail_len = sig.params.tail.len();

    if repeat_len == 0 {
        return 0;
    }

    let Some(shape) = repeat_shape_info(sig, total_args_for_shape) else {
        return 0;
    };
    let tail_start = shape.tail_start;
    let repeat_groups_displayed = shape.repeat_groups;

    // Display shape: head, repeat groups, "...", tail
    let ellipsis_idx = head_len + repeat_len * repeat_groups_displayed;
    let tail_display_start = ellipsis_idx + 1;

    if call_ctx.arg_index < head_len {
        return call_ctx.arg_index;
    }

    if call_ctx.arg_index >= tail_start {
        let tail_idx = call_ctx.arg_index.saturating_sub(tail_start);
        let max_tail = tail_len.saturating_sub(1);
        return tail_display_start + tail_idx.min(max_tail);
    }

    let idx_in_repeat = call_ctx.arg_index.saturating_sub(head_len);
    let repeat_pos = idx_in_repeat % repeat_len;

    let cycle = idx_in_repeat / repeat_len;
    head_len + cycle * repeat_len + repeat_pos
}

fn nearest_param_index(slots: &[ParamSlot], slot_idx: usize) -> usize {
    let get = |i: usize| match slots.get(i) {
        Some(ParamSlot::Param { param_index, .. }) => Some(*param_index as usize),
        _ => None,
    };

    if let Some(idx) = get(slot_idx) {
        return idx;
    }
    for i in (0..slot_idx).rev() {
        if let Some(idx) = get(i) {
            return idx;
        }
    }
    for i in (slot_idx + 1)..slots.len() {
        if let Some(idx) = get(i) {
            return idx;
        }
    }
    0
}

/// Computes signature help when the cursor is inside a call argument list.
///
/// Returns `None` if the cursor is before the `(`, or if the callee is unknown.
pub(super) fn compute_signature_help_if_in_call(
    source: &str,
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

    let is_postfix_call = || {
        let (callee_idx, callee_token) = prev_non_trivia_before(tokens, call_ctx.lparen_idx)?;
        let TokenKind::Ident(_) = callee_token.kind else {
            return None;
        };
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
    let (inst_param_tys, inst_ret) = semantic::instantiate_sig(func, arg_tys.as_slice());

    let mut full_call_ctx = call_ctx.clone();
    if is_method_style {
        // `receiver.fn(arg1, ...)` is treated as `fn(receiver, arg1, ...)` internally.
        full_call_ctx.arg_index = full_call_ctx.arg_index.saturating_add(1);
    }

    let total_args_for_shape = arg_tys.len().max(full_call_ctx.arg_index + 1);
    let rendered = render_signature(
        func,
        arg_tys.as_slice(),
        total_args_for_shape,
        inst_param_tys.as_slice(),
        is_method_style,
    );

    let active_slot = if rendered.slots.is_empty() {
        0
    } else {
        let full_slot = active_param_index(func, &full_call_ctx, total_args_for_shape);
        let slot = if is_method_style {
            full_slot.saturating_sub(1)
        } else {
            full_slot
        };
        slot.min(rendered.slots.len().saturating_sub(1))
    };

    let active_parameter = nearest_param_index(rendered.slots.as_slice(), active_slot);
    let segments =
        build_signature_segments(func.name.as_str(), &rendered, &inst_ret, is_method_style);

    Some(SignatureHelp {
        signatures: vec![SignatureItem { segments }],
        active_signature: 0,
        active_parameter,
    })
}

/// Finds the innermost call whose `(` starts before `cursor`.
pub(super) fn detect_call_context(tokens: &[Token], cursor: u32) -> Option<CallContext> {
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
pub(super) fn expected_call_arg_ty(
    call_ctx: Option<&CallContext>,
    ctx: Option<&semantic::Context>,
) -> Option<semantic::Ty> {
    let call_ctx = call_ctx?;
    let ctx = ctx?;
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

#[cfg(test)]
mod tests {
    use super::nearest_param_index;
    use crate::ide::display::ParamSlot;

    #[test]
    fn active_parameter_uses_nearest_param_index_when_on_ellipsis() {
        let slots = vec![
            ParamSlot::Param {
                name: "a".into(),
                ty: "number".into(),
                param_index: 3,
            },
            ParamSlot::Ellipsis,
            ParamSlot::Param {
                name: "b".into(),
                ty: "string".into(),
                param_index: 4,
            },
        ];
        assert_eq!(nearest_param_index(&slots, 1), 3);
    }
}

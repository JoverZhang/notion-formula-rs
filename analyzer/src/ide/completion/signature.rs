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
    // If the declared parameter includes generics, prefer the inferred actual type when the
    // argument expression is non-empty. This helps show instantiated generics (incl `unknown`)
    // at the call site.
    if ty_contains_generic(declared_template) {
        return actual.unwrap_or(instantiated_expected);
    }

    let Some(actual) = actual else {
        return instantiated_expected;
    };

    // Avoid "unknown" overriding useful expected types (especially for hard-constrained params).
    if matches!(actual, semantic::Ty::Unknown) {
        return instantiated_expected;
    }

    // For union-typed params (e.g. `number | number[]`), the actual argument type is often more
    // helpful than repeating the full union at every slot.
    if matches!(instantiated_expected, semantic::Ty::Union(_))
        && semantic::ty_accepts(instantiated_expected, actual)
    {
        return actual;
    }

    instantiated_expected
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

    fn repeat_name(base: &str, n: usize) -> String {
        if let Some(prefix) = base.strip_suffix('N') {
            return format!("{prefix}{n}");
        }

        let digits_len = base
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digits_len > 0 {
            let split = base.len().saturating_sub(digits_len);
            let (prefix, suffix) = base.split_at(split);
            if suffix == "1" {
                return format!("{prefix}{n}");
            }
        }

        format!("{base}{n}")
    }

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
        for (idx, p) in sig
            .params
            .head
            .iter()
            .chain(sig.params.tail.iter())
            .enumerate()
        {
            let instantiated_expected = inst_param_tys.get(idx).unwrap_or(&p.ty);
            let actual = arg_tys.get(idx).and_then(|t| t.as_ref());
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

    let (repeat_groups_displayed, tail_start) =
        semantic::complete_repeat_shape(&sig.params, total_args_for_shape)
            .map(|s| (s.repeat_groups, s.tail_start))
            .unwrap_or((1, usize::MAX));

    for n in 1..=repeat_groups_displayed {
        for (r_idx, p) in sig.params.repeat.iter().enumerate() {
            let name = repeat_name(p.name.as_str(), n);
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

fn active_parameter_for_call(
    sig: &semantic::FunctionSig,
    arg_index_full: usize,
    total_args_for_shape: usize,
    is_method_style: bool,
) -> usize {
    let idx = if sig.params.repeat.is_empty() {
        let total_params = sig.params.head.len() + sig.params.tail.len();
        if total_params == 0 {
            0
        } else {
            arg_index_full.min(total_params - 1)
        }
    } else {
        let head_len = sig.params.head.len();
        let repeat_len = sig.params.repeat.len();
        let tail_len = sig.params.tail.len();

        if repeat_len == 0 {
            return 0;
        }

        let Some(shape) = semantic::complete_repeat_shape(&sig.params, total_args_for_shape) else {
            return 0;
        };

        if arg_index_full < head_len {
            arg_index_full
        } else if arg_index_full >= shape.tail_start {
            let tail_idx = arg_index_full.saturating_sub(shape.tail_start);
            let max_tail = tail_len.saturating_sub(1);
            let tail_idx = tail_idx.min(max_tail);
            head_len + repeat_len * shape.repeat_groups + tail_idx
        } else {
            let idx_in_repeat = arg_index_full.saturating_sub(head_len);
            let repeat_pos = idx_in_repeat % repeat_len;

            let cycle = idx_in_repeat / repeat_len;
            head_len + cycle * repeat_len + repeat_pos
        }
    };

    if is_method_style {
        idx.saturating_sub(1)
    } else {
        idx
    }
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
    let (inst_param_tys, inst_ret) = semantic::instantiate_sig(func, arg_tys.as_slice());

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

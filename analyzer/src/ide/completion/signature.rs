use super::SignatureHelp;
use super::position::prev_non_trivia_before;
use crate::ast::{Expr, ExprKind};
use crate::lexer::{Token, TokenKind};
use crate::semantic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CallContext {
    pub(super) callee: String,
    pub(super) lparen_idx: usize,
    pub(super) arg_index: usize,
}

pub(super) fn format_ty(ty: &semantic::Ty) -> String {
    match ty {
        semantic::Ty::Number => "number".into(),
        semantic::Ty::String => "string".into(),
        semantic::Ty::Boolean => "boolean".into(),
        semantic::Ty::Date => "date".into(),
        semantic::Ty::Null => "null".into(),
        semantic::Ty::Unknown => "unknown".into(),
        semantic::Ty::Generic(id) => format!("T{}", id.0),
        semantic::Ty::List(inner) => format!("{}[]", format_ty(inner)),
        semantic::Ty::Union(types) => types.iter().map(format_ty).collect::<Vec<_>>().join(" | "),
    }
}

fn ty_contains_generic(ty: &semantic::Ty) -> bool {
    match ty {
        semantic::Ty::Generic(_) => true,
        semantic::Ty::List(inner) => ty_contains_generic(inner),
        semantic::Ty::Union(members) => members.iter().any(ty_contains_generic),
        _ => false,
    }
}

fn format_param_sig(name: &str, ty: &semantic::Ty, optional: bool) -> String {
    let mut ty = format_ty(ty);
    if optional {
        ty.push('?');
    }
    format!("{name}: {ty}")
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

fn format_signature_display(
    sig: &semantic::FunctionSig,
    arg_tys: &[Option<semantic::Ty>],
    total_args_for_shape: usize,
    inst_param_tys: &[semantic::Ty],
    inst_ret: &semantic::Ty,
) -> (String, Vec<String>) {
    if sig.params.repeat.is_empty() {
        let mut idx = 0usize;
        let params = sig
            .params
            .head
            .iter()
            .chain(sig.params.tail.iter())
            .map(|p| {
                let instantiated_expected = inst_param_tys.get(idx).unwrap_or(&p.ty);
                let actual = arg_tys.get(idx).and_then(|t| t.as_ref());
                let ty = choose_display_ty(actual, &p.ty, instantiated_expected);
                idx += 1;
                format_param_sig(&p.name, ty, p.optional)
            })
            .collect::<Vec<_>>();

        let label_params = params.join(", ");
        let label = format!("{}({}) -> {}", sig.name, label_params, format_ty(inst_ret));
        return (label, params);
    }

    let mut params = Vec::<String>::new();
    for (idx, p) in sig.params.head.iter().enumerate() {
        let ty = inst_param_tys.get(idx).unwrap_or(&p.ty);
        params.push(format_param_sig(&p.name, ty, p.optional));
    }

    // Show the repeat pattern twice with numbering, then an ellipsis, then the tail.
    let repeat_start = sig.params.head.len();
    let repeat_len = sig.params.repeat.len();

    let shape = semantic::complete_repeat_shape(&sig.params, total_args_for_shape);
    let repeat_groups = shape.map(|s| s.repeat_groups).unwrap_or(1).max(1);
    let repeat_groups_displayed = repeat_groups.min(2);
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
            params.push(format_param_sig(&name, ty, p.optional));
        }
    }
    params.push("...".into());
    for (t_idx, p) in sig.params.tail.iter().enumerate() {
        let actual_idx = tail_start.saturating_add(t_idx);
        let inst_idx = repeat_start + repeat_len + t_idx;
        let instantiated_expected = inst_param_tys.get(inst_idx).unwrap_or(&p.ty);
        let actual = arg_tys.get(actual_idx).and_then(|t| t.as_ref());
        let ty = choose_display_ty(actual, &p.ty, instantiated_expected);
        params.push(format_param_sig(&p.name, ty, p.optional));
    }

    let label_params = params.join(", ");
    let label = format!("{}({}) -> {}", sig.name, label_params, format_ty(inst_ret));
    (label, params)
}

fn find_call_expr_by_lparen<'a>(
    root: &'a Expr,
    callee: &str,
    lparen_start: u32,
) -> Option<&'a Expr> {
    fn visit<'a>(
        expr: &'a Expr,
        callee: &str,
        lparen_start: u32,
        best: &mut Option<&'a Expr>,
    ) {
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

        let mut depth = 0i32;
        let mut start = lparen.span.end;

        for token in tokens.iter().skip(lparen_idx + 1) {
            if token.is_trivia() || matches!(token.kind, TokenKind::Eof) {
                continue;
            }

            match token.kind {
                TokenKind::OpenParen => depth += 1,
                TokenKind::CloseParen => {
                    if depth == 0 {
                        spans.push(crate::Span {
                            start,
                            end: token.span.start,
                        });
                        return spans;
                    }
                    depth -= 1;
                }
                TokenKind::Comma if depth == 0 => {
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
    if include_receiver_as_arg {
        if let Ok(parsed) = crate::analyze(source) {
            if let Some(call_expr) =
                find_call_expr_by_lparen(&parsed.expr, &call_ctx.callee, lparen_token.span.start)
            {
                if let ExprKind::MemberCall { receiver, .. } = &call_expr.kind {
                    let mut map = semantic::TypeMap::default();
                    let _ = semantic::infer_expr_with_map(&parsed.expr, ctx, &mut map);
                    let mut ty = map.get(receiver.id).cloned().unwrap_or(semantic::Ty::Unknown);
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

fn active_param_index(sig: &semantic::FunctionSig, call_ctx: &CallContext, total_args_for_shape: usize) -> usize {
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

    let Some(shape) = semantic::complete_repeat_shape(&sig.params, total_args_for_shape) else {
        return 0;
    };
    let tail_start = shape.tail_start;
    let repeat_groups_displayed = shape.repeat_groups.max(1).min(2);

    // Display shape: head, repeat x{1..2}, "...", tail
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

    // Map all cycles >= 2 to the second displayed cycle so we can still highlight
    // condition vs value within the repeat pair.
    let cycle = (idx_in_repeat / repeat_len).min(repeat_groups_displayed.saturating_sub(1));
    head_len + cycle * repeat_len + repeat_pos
}

/// Only compute signature help if the cursor is inside a function call argument context
/// (i.e., after the opening parenthesis).
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
    let can_use_postfix_help = is_postfix_call
        && semantic::postfix_capable_builtin_names().contains(func.name.as_str())
        && semantic::is_postfix_capable(func);

    let arg_tys =
        infer_call_arg_tys_best_effort(source, tokens, ctx, call_ctx, can_use_postfix_help);
    let (inst_param_tys, inst_ret) = semantic::instantiate_sig(func, arg_tys.as_slice());

    let mut full_call_ctx = call_ctx.clone();
    if can_use_postfix_help {
        // `receiver.fn(arg1, ...)` is treated as `fn(receiver, arg1, ...)` internally.
        full_call_ctx.arg_index = full_call_ctx.arg_index.saturating_add(1);
    }

    let total_args_for_shape = arg_tys.len().max(full_call_ctx.arg_index + 1);
    let (full_label, full_params) = format_signature_display(
        func,
        arg_tys.as_slice(),
        total_args_for_shape,
        inst_param_tys.as_slice(),
        &inst_ret,
    );
    let full_active_param = if full_params.is_empty() {
        0
    } else {
        active_param_index(func, &full_call_ctx, total_args_for_shape).min(full_params.len() - 1)
    };

    if !can_use_postfix_help {
        return Some(SignatureHelp {
            receiver: None,
            label: full_label,
            params: full_params,
            active_param: full_active_param,
        });
    }

    // Postfix rendering is a presentation-only transformation: split off the receiver slot.
    let receiver = full_params.first().cloned();
    let params = full_params.into_iter().skip(1).collect::<Vec<_>>();
    let label = format!("{}({}) -> {}", func.name, params.join(", "), format_ty(&inst_ret));

    let active_param = if params.is_empty() {
        0
    } else {
        let shifted = if full_active_param == 0 {
            0
        } else {
            full_active_param - 1
        };
        shifted.min(params.len() - 1)
    };

    Some(SignatureHelp {
        receiver,
        label,
        params,
        active_param,
    })
}

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

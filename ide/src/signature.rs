//! Signature help for calls under the cursor.
//! Uses UTF-8 byte offsets (via tokens/spans) and best-effort type inference.

use crate::context::{CallContext, prev_non_trivia_before};
use crate::display::{ParamSlot, RenderedSignature, build_signature_segments};
use analyzer::ast::{Expr, ExprKind};
use analyzer::semantic;
use analyzer::{Span, Token, TokenKind};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompletedRepeatShape {
    tail_start: usize,
    repeat_groups: usize,
}

fn required_tail_prefix_len(tail: &[semantic::ParamSig]) -> usize {
    let mut required = 0usize;
    for (idx, p) in tail.iter().enumerate() {
        if !p.optional {
            required = idx + 1;
        }
    }
    required
}

fn ceil_to_multiple(n: usize, m: usize) -> usize {
    if m == 0 {
        return n;
    }
    if n == 0 {
        return 0;
    }
    let rem = n % m;
    if rem == 0 { n } else { n + (m - rem) }
}

fn resolve_repeat_tail_used_with_min_groups(
    params: &semantic::ParamShape,
    total: usize,
    repeat_min_groups: usize,
) -> Option<usize> {
    if params.repeat.is_empty() {
        return Some(params.tail.len());
    }

    let head_len = params.head.len();
    if total < head_len {
        return None;
    }

    let repeat_len = params.repeat.len();
    let tail_min = required_tail_prefix_len(&params.tail);
    let min_middle = repeat_len.saturating_mul(repeat_min_groups);

    for tail_used in (tail_min..=params.tail.len()).rev() {
        if total < head_len + tail_used {
            continue;
        }
        let middle = total - head_len - tail_used;
        if middle >= min_middle && middle.is_multiple_of(repeat_len) {
            return Some(tail_used);
        }
    }

    None
}

fn complete_repeat_shape(
    params: &semantic::ParamShape,
    total: usize,
) -> Option<CompletedRepeatShape> {
    const REPEAT_MIN_GROUPS: usize = 1;

    if params.repeat.is_empty() {
        return None;
    }

    let head_len = params.head.len();
    let repeat_len = params.repeat.len();
    if repeat_len == 0 {
        return None;
    }

    if let Some(tail_used) =
        resolve_repeat_tail_used_with_min_groups(params, total, REPEAT_MIN_GROUPS)
    {
        let tail_start = total.saturating_sub(tail_used);
        let middle = total.saturating_sub(head_len + tail_used);
        let repeat_groups = middle / repeat_len;
        return Some(CompletedRepeatShape {
            tail_start,
            repeat_groups,
        });
    }

    let tail_min = required_tail_prefix_len(&params.tail);
    let min_middle = repeat_len.saturating_mul(REPEAT_MIN_GROUPS);

    let mut best: Option<(usize /* total */, usize /* tail_used */)> = None;
    for tail_used in tail_min..=params.tail.len() {
        let min_total_for_tail = head_len.saturating_add(tail_used);
        let min_total_for_middle = head_len
            .saturating_add(tail_used)
            .saturating_add(min_middle);

        let base_total = total.max(min_total_for_tail).max(min_total_for_middle);
        let middle_base = base_total - head_len - tail_used;
        let middle = ceil_to_multiple(middle_base, repeat_len);
        let completed_total = head_len + tail_used + middle;

        match best {
            None => best = Some((completed_total, tail_used)),
            Some((best_total, best_tail_used)) => {
                if completed_total < best_total
                    || (completed_total == best_total && tail_used > best_tail_used)
                {
                    best = Some((completed_total, tail_used));
                }
            }
        }
    }

    let (completed_total, tail_used) = best?;
    let tail_start = completed_total - tail_used;
    let middle = completed_total - head_len - tail_used;
    let repeat_groups = middle / repeat_len;
    Some(CompletedRepeatShape {
        tail_start,
        repeat_groups,
    })
}

type Subst = HashMap<semantic::GenericId, semantic::Ty>;
type GenericRegistry = HashMap<semantic::GenericId, semantic::GenericParamKind>;

fn registry_for(sig: &semantic::FunctionSig) -> GenericRegistry {
    sig.generics.iter().map(|g| (g.id, g.kind)).collect()
}

fn bind_generic(
    subst: &mut Subst,
    registry: &GenericRegistry,
    id: semantic::GenericId,
    actual: &semantic::Ty,
) {
    let kind = registry
        .get(&id)
        .copied()
        .unwrap_or(semantic::GenericParamKind::Plain);

    fn contains_unknown(ty: &semantic::Ty) -> bool {
        match ty {
            semantic::Ty::Unknown => true,
            semantic::Ty::Union(members) => members.iter().any(contains_unknown),
            _ => false,
        }
    }

    match kind {
        semantic::GenericParamKind::Plain => {
            if matches!(actual, semantic::Ty::Unknown) {
                return;
            }

            let to_add = vec![actual.clone()];
            match subst.get(&id).cloned() {
                None => {
                    subst.insert(id, semantic::normalize_union(to_add));
                }
                Some(prev) => {
                    subst.insert(
                        id,
                        semantic::normalize_union(std::iter::once(prev).chain(to_add)),
                    );
                }
            }
        }
        semantic::GenericParamKind::Variant => {
            if contains_unknown(actual) {
                subst.insert(id, semantic::Ty::Unknown);
                return;
            }

            if subst
                .get(&id)
                .is_some_and(|t| matches!(t, semantic::Ty::Unknown))
            {
                return;
            }

            let mut to_add: Vec<semantic::Ty> = Vec::new();
            match actual {
                semantic::Ty::Union(members) => {
                    to_add.extend(members.iter().cloned());
                }
                other => {
                    to_add.push(other.clone());
                }
            }

            if to_add.is_empty() {
                return;
            }

            match subst.get(&id).cloned() {
                None => {
                    subst.insert(id, semantic::normalize_union(to_add));
                }
                Some(prev) => {
                    subst.insert(
                        id,
                        semantic::normalize_union(std::iter::once(prev).chain(to_add)),
                    );
                }
            }
        }
    }
}

fn unify(
    subst: &mut Subst,
    registry: &GenericRegistry,
    expected: &semantic::Ty,
    actual: &semantic::Ty,
) {
    match expected {
        semantic::Ty::Generic(id) => bind_generic(subst, registry, *id, actual),
        semantic::Ty::List(exp_inner) => {
            if let semantic::Ty::List(act_inner) = actual {
                unify(subst, registry, exp_inner, act_inner);
            }
        }
        semantic::Ty::Union(branches) => {
            for branch in branches {
                unify(subst, registry, branch, actual);
            }
        }
        _ => {}
    }
}

fn apply(subst: &Subst, ty_template: &semantic::Ty) -> semantic::Ty {
    match ty_template {
        semantic::Ty::Generic(id) => subst.get(id).cloned().unwrap_or(semantic::Ty::Unknown),
        semantic::Ty::List(inner) => semantic::Ty::List(Box::new(apply(subst, inner))),
        semantic::Ty::Union(members) => {
            semantic::normalize_union(members.iter().map(|m| apply(subst, m)))
        }
        other => other.clone(),
    }
}

fn unify_call_args_present(
    sig: &semantic::FunctionSig,
    arg_tys: &[Option<semantic::Ty>],
    subst: &mut Subst,
) {
    let registry = registry_for(sig);

    if sig.params.repeat.is_empty() {
        let total_params = sig.params.head.len() + sig.params.tail.len();
        for (idx, actual) in arg_tys.iter().enumerate() {
            if idx >= total_params {
                break;
            }
            let Some(actual) = actual else {
                continue;
            };
            let expected = if idx < sig.params.head.len() {
                sig.params.head.get(idx)
            } else {
                sig.params.tail.get(idx - sig.params.head.len())
            };
            if let Some(param) = expected {
                unify(subst, &registry, &param.ty, actual);
            }
        }
        return;
    }

    let head_len = sig.params.head.len();
    let Some(shape) = complete_repeat_shape(&sig.params, arg_tys.len()) else {
        return;
    };

    for (idx, actual) in arg_tys.iter().enumerate() {
        let Some(actual) = actual else {
            continue;
        };

        let expected = if idx < head_len {
            sig.params.head.get(idx)
        } else if idx >= shape.tail_start {
            sig.params.tail.get(idx - shape.tail_start)
        } else {
            let r_idx = (idx - head_len) % sig.params.repeat.len();
            sig.params.repeat.get(r_idx)
        };

        if let Some(param) = expected {
            unify(subst, &registry, &param.ty, actual);
        }
    }
}

fn instantiate_sig(
    sig: &semantic::FunctionSig,
    arg_tys: &[Option<semantic::Ty>],
) -> (Vec<semantic::Ty>, semantic::Ty) {
    let mut subst = Subst::new();
    unify_call_args_present(sig, arg_tys, &mut subst);

    let params = sig
        .display_params()
        .into_iter()
        .map(|p| apply(&subst, &p.ty))
        .collect::<Vec<_>>();
    let ret = apply(&subst, &sig.ret);
    (params, ret)
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
        complete_repeat_shape(&sig.params, total_args_for_shape)
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

        let parsed = analyzer::analyze_syntax(trimmed);

        let mut map = analyzer::TypeMap::default();
        Some(analyzer::infer_expr_with_map(&parsed.expr, ctx, &mut map))
    }

    fn arg_spans(tokens: &[Token], lparen_idx: usize, source_len: u32) -> Vec<Span> {
        let mut spans = Vec::<Span>::new();
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
                        spans.push(Span {
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
                    spans.push(Span {
                        start,
                        end: token.span.start,
                    });
                    start = token.span.end;
                }
                _ => {}
            }
        }

        spans.push(Span {
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

        let Some(shape) = complete_repeat_shape(&sig.params, total_args_for_shape) else {
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

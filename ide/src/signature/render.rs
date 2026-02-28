//! Signature rendering into display slots.
//!
//! Converts a function signature + inferred argument types into a
//! [`RenderedSignature`] that the display layer can format.

use crate::display::{ParamSlot, RenderedSignature};
use analyzer::semantic;

use super::param_shape::complete_repeat_shape;

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

pub(super) fn render_signature(
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

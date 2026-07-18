//! Presentation adapter for a shared resolved call projection.

use crate::display::{ParamSlot, RenderedSignature};
use analyzer::semantic;

fn ty_contains_generic(ty: &semantic::Ty) -> bool {
    match ty {
        semantic::Ty::Generic(_) => true,
        semantic::Ty::List(inner) | semantic::Ty::Ident(inner) => ty_contains_generic(inner),
        semantic::Ty::Union(members) => members.iter().any(ty_contains_generic),
        semantic::Ty::Fn { params, ret } => {
            params.iter().any(|(_, ty)| ty_contains_generic(ty)) || ty_contains_generic(ret)
        }
        _ => false,
    }
}

fn unwrap_for_display(ty: &semantic::Ty) -> &semantic::Ty {
    match ty {
        semantic::Ty::Fn { ret, .. } => ret,
        semantic::Ty::Ident(inner) => inner,
        other => other,
    }
}

fn format_ty_with_optional(ty: &semantic::Ty, optional: bool) -> String {
    let mut output = unwrap_for_display(ty).to_string();
    if optional {
        output.push('?');
    }
    output
}

fn observed_ty(observation: Option<&semantic::ArgumentObservation>) -> Option<&semantic::Ty> {
    match observation {
        Some(semantic::ArgumentObservation::Typed(ty)) => Some(ty),
        Some(semantic::ArgumentObservation::Empty) | None => None,
    }
}

fn choose_display_ty<'a>(
    actual: Option<&'a semantic::Ty>,
    declared_template: &'a semantic::Ty,
    instantiated_expected: &'a semantic::Ty,
) -> &'a semantic::Ty {
    let template = unwrap_for_display(declared_template);
    let expected = unwrap_for_display(instantiated_expected);

    if ty_contains_generic(template) {
        return actual.unwrap_or(expected);
    }
    let Some(actual) = actual else {
        return expected;
    };
    if matches!(actual, semantic::Ty::Unknown) {
        return expected;
    }
    if matches!(expected, semantic::Ty::Union(_)) && semantic::type_accepts(expected, actual) {
        return actual;
    }
    expected
}

pub(super) fn render_signature(
    signature: &semantic::FunctionSig,
    observations: &[semantic::ArgumentObservation],
    resolved: &semantic::ResolvedFunctionSig,
    is_method_style: bool,
) -> RenderedSignature {
    let mut receiver = None;
    let mut slots = Vec::new();
    let mut next_parameter_index = 0u32;
    let mut ellipsis_inserted = false;

    for slot in &resolved.projection {
        if !signature.params.repeat.is_empty()
            && !ellipsis_inserted
            && matches!(slot.logical_param, semantic::ParamRef::Tail(_))
        {
            slots.push(ParamSlot::Ellipsis);
            ellipsis_inserted = true;
        }

        let parameter = semantic::param_for_ref(signature, slot.logical_param);
        let actual = slot
            .argument_index
            .and_then(|index| observed_ty(observations.get(index)));
        let ty = choose_display_ty(actual, &parameter.ty, &slot.expected_ty);
        let name = slot.repeat_group.map_or_else(
            || parameter.name.clone(),
            |group| format!("{}{group}", parameter.name),
        );
        let rendered_ty = format_ty_with_optional(ty, parameter.optional);

        if is_method_style && receiver.is_none() {
            receiver = Some((name, rendered_ty));
        } else {
            slots.push(ParamSlot::Param {
                name,
                ty: rendered_ty,
                param_index: next_parameter_index,
            });
            next_parameter_index += 1;
        }
    }

    if !signature.params.repeat.is_empty() && !ellipsis_inserted {
        slots.push(ParamSlot::Ellipsis);
    }

    RenderedSignature { receiver, slots }
}

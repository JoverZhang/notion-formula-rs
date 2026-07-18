//! Pure call-site projection, generic binding, and return-type refinement.

use std::collections::HashMap;

use crate::{
    FunctionSig, GenericId, GenericParamKind, ParamShape, ParamSig, Ty, normalize_union,
    resolve_repeat_tail_used,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentObservation {
    Empty,
    Typed(Ty),
}

#[derive(Debug, Clone, Copy)]
pub struct CallSignatureInput<'a> {
    pub arguments: &'a [ArgumentObservation],
}

#[derive(Debug, Clone, Copy)]
pub struct ResolverInput<'a> {
    pub arguments: &'a [ArgumentObservation],
    pub default_return_ty: &'a Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFunctionSig {
    pub validity: ShapeValidity,
    pub projection: Vec<ResolvedParamSlot>,
    pub arguments: Vec<ResolvedArgument>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeValidity {
    Valid,
    Invalid(CallShapeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallShapeError {
    TooFew { minimum: usize, actual: usize },
    TooMany { maximum: usize, actual: usize },
    InvalidRepeat { actual: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamRef {
    Head(usize),
    Repeat(usize),
    Tail(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedParamSlot {
    pub logical_param: ParamRef,
    pub repeat_group: Option<usize>,
    pub argument_index: Option<usize>,
    pub expected_ty: Ty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArgument {
    pub parameter: Option<ParamRef>,
    pub repeat_group: Option<usize>,
    pub expected_ty: Option<Ty>,
    pub type_status: ArgumentTypeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentTypeStatus {
    Compatible,
    Mismatch { actual: Ty },
    Indeterminate,
    Unmapped,
}

type Substitution = HashMap<GenericId, Ty>;
type GenericRegistry = HashMap<GenericId, GenericParamKind>;

/// Resolve one immutable call snapshot. Re-running this function carries no hidden state.
pub fn resolve_call_signature(
    signature: &FunctionSig,
    input: CallSignatureInput<'_>,
) -> ResolvedFunctionSig {
    let shape = project_shape(&signature.params, input.arguments.len());
    let registry = signature
        .generics
        .iter()
        .map(|generic| (generic.id, generic.kind))
        .collect::<GenericRegistry>();
    let mut substitution = Substitution::new();

    for (argument_index, observation) in input.arguments.iter().enumerate() {
        let Some(slot) = shape.projection.get(argument_index) else {
            continue;
        };
        let ArgumentObservation::Typed(actual) = observation else {
            continue;
        };
        let template = parameter(signature, slot.parameter);
        unify(&mut substitution, &registry, &template.ty, actual);
    }

    let default_return_ty = instantiate(&substitution, &signature.ret);
    let return_ty = signature.resolver.map_or_else(
        || default_return_ty.clone(),
        |resolver| {
            resolver(&ResolverInput {
                arguments: input.arguments,
                default_return_ty: &default_return_ty,
            })
        },
    );

    let projection = shape
        .projection
        .iter()
        .enumerate()
        .map(|(index, slot)| ResolvedParamSlot {
            logical_param: slot.parameter,
            repeat_group: slot.repeat_group,
            argument_index: (index < input.arguments.len()).then_some(index),
            expected_ty: instantiate(&substitution, &parameter(signature, slot.parameter).ty),
        })
        .collect::<Vec<_>>();

    let arguments = input
        .arguments
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let Some(slot) = projection.get(index) else {
                return ResolvedArgument {
                    parameter: None,
                    repeat_group: None,
                    expected_ty: None,
                    type_status: ArgumentTypeStatus::Unmapped,
                };
            };
            ResolvedArgument {
                parameter: Some(slot.logical_param),
                repeat_group: slot.repeat_group,
                expected_ty: Some(slot.expected_ty.clone()),
                type_status: check_argument_type(&slot.expected_ty, observation),
            }
        })
        .collect();

    ResolvedFunctionSig {
        validity: shape.validity,
        projection,
        arguments,
        return_ty,
    }
}

/// Return whether an inferred or runtime-observed type is accepted by an expected type.
pub fn type_accepts(expected: &Ty, actual: &Ty) -> bool {
    if matches!(expected, Ty::Unknown | Ty::Generic(_)) || matches!(actual, Ty::Unknown) {
        return true;
    }
    match (expected, actual) {
        (Ty::Fn { ret, .. }, actual) => type_accepts(ret, actual),
        (Ty::Ident(_), _) => true,
        (Ty::Union(_), Ty::Union(actual_members)) => actual_members
            .iter()
            .all(|actual| type_accepts(expected, actual)),
        (Ty::Union(expected_members), actual) => expected_members
            .iter()
            .any(|expected| type_accepts(expected, actual)),
        (expected, Ty::Union(actual_members)) => actual_members
            .iter()
            .all(|actual| type_accepts(expected, actual)),
        (Ty::List(expected), Ty::List(actual)) => type_accepts(expected, actual),
        _ => expected == actual,
    }
}

pub fn check_argument_type(expected: &Ty, observation: &ArgumentObservation) -> ArgumentTypeStatus {
    match observation {
        ArgumentObservation::Empty => ArgumentTypeStatus::Indeterminate,
        ArgumentObservation::Typed(actual) if contains_unknown(actual) => {
            ArgumentTypeStatus::Indeterminate
        }
        ArgumentObservation::Typed(actual) if type_accepts(expected, actual) => {
            ArgumentTypeStatus::Compatible
        }
        ArgumentObservation::Typed(actual) => ArgumentTypeStatus::Mismatch {
            actual: actual.clone(),
        },
    }
}

pub fn param_for_ref(signature: &FunctionSig, reference: ParamRef) -> &ParamSig {
    parameter(signature, reference)
}

struct ShapeProjection {
    validity: ShapeValidity,
    projection: Vec<ProjectionSlot>,
}

#[derive(Debug, Clone, Copy)]
struct ProjectionSlot {
    parameter: ParamRef,
    repeat_group: Option<usize>,
}

fn project_shape(params: &ParamShape, observed: usize) -> ShapeProjection {
    if params.repeat.is_empty() {
        return project_fixed_shape(params, observed);
    }

    if let Some(projection) = exact_repeat_projection(params, observed) {
        return ShapeProjection {
            validity: ShapeValidity::Valid,
            projection,
        };
    }

    let minimum = params.head.len()
        + params.repeat.len() * params.repeat_min_groups
        + required_tail_prefix_len(&params.tail);
    let validity = if observed < minimum {
        ShapeValidity::Invalid(CallShapeError::TooFew {
            minimum,
            actual: observed,
        })
    } else {
        ShapeValidity::Invalid(CallShapeError::InvalidRepeat { actual: observed })
    };
    let completed = minimum_completable_repeat_total(params, observed);
    let projection = exact_repeat_projection(params, completed)
        .expect("completion algorithm must produce a valid repeat shape");
    ShapeProjection {
        validity,
        projection,
    }
}

fn project_fixed_shape(params: &ParamShape, observed: usize) -> ShapeProjection {
    let minimum = required_tail_prefix_len(&params.head) + required_tail_prefix_len(&params.tail);
    let maximum = params.head.len() + params.tail.len();
    let validity = if observed < minimum {
        ShapeValidity::Invalid(CallShapeError::TooFew {
            minimum,
            actual: observed,
        })
    } else if observed > maximum {
        ShapeValidity::Invalid(CallShapeError::TooMany {
            maximum,
            actual: observed,
        })
    } else {
        ShapeValidity::Valid
    };
    let projection = (0..params.head.len())
        .map(|index| ProjectionSlot {
            parameter: ParamRef::Head(index),
            repeat_group: None,
        })
        .chain((0..params.tail.len()).map(|index| ProjectionSlot {
            parameter: ParamRef::Tail(index),
            repeat_group: None,
        }))
        .collect();
    ShapeProjection {
        validity,
        projection,
    }
}

fn exact_repeat_projection(params: &ParamShape, total: usize) -> Option<Vec<ProjectionSlot>> {
    let tail_used = resolve_repeat_tail_used(params, total)?;
    let middle = total.checked_sub(params.head.len() + tail_used)?;
    let groups = middle / params.repeat.len();
    let mut projection = Vec::with_capacity(total);
    projection.extend((0..params.head.len()).map(|index| ProjectionSlot {
        parameter: ParamRef::Head(index),
        repeat_group: None,
    }));
    for group in 1..=groups {
        projection.extend((0..params.repeat.len()).map(|index| ProjectionSlot {
            parameter: ParamRef::Repeat(index),
            repeat_group: Some(group),
        }));
    }
    projection.extend((0..tail_used).map(|index| ProjectionSlot {
        parameter: ParamRef::Tail(index),
        repeat_group: None,
    }));
    Some(projection)
}

fn minimum_completable_repeat_total(params: &ParamShape, observed: usize) -> usize {
    let repeat_len = params.repeat.len();
    let tail_min = required_tail_prefix_len(&params.tail);
    let minimum_middle = repeat_len * params.repeat_min_groups;
    let mut best: Option<(usize, usize)> = None;

    for tail_used in tail_min..=params.tail.len() {
        let base = observed
            .max(params.head.len() + tail_used)
            .max(params.head.len() + tail_used + minimum_middle);
        let middle_base = base - params.head.len() - tail_used;
        let middle = ceil_to_multiple(middle_base, repeat_len);
        let completed = params.head.len() + middle + tail_used;
        if best.is_none_or(|(best_total, best_tail)| {
            completed < best_total || (completed == best_total && tail_used > best_tail)
        }) {
            best = Some((completed, tail_used));
        }
    }
    best.expect("repeat shapes always have a completable total")
        .0
}

fn required_tail_prefix_len(params: &[ParamSig]) -> usize {
    params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| (!param.optional).then_some(index + 1))
        .max()
        .unwrap_or(0)
}

fn ceil_to_multiple(value: usize, multiple: usize) -> usize {
    let remainder = value % multiple;
    if remainder == 0 {
        value
    } else {
        value + multiple - remainder
    }
}

fn parameter(signature: &FunctionSig, reference: ParamRef) -> &ParamSig {
    match reference {
        ParamRef::Head(index) => &signature.params.head[index],
        ParamRef::Repeat(index) => &signature.params.repeat[index],
        ParamRef::Tail(index) => &signature.params.tail[index],
    }
}

fn bind_generic(
    substitution: &mut Substitution,
    registry: &GenericRegistry,
    id: GenericId,
    actual: &Ty,
) {
    let kind = registry
        .get(&id)
        .copied()
        .unwrap_or(GenericParamKind::Plain);
    if kind == GenericParamKind::Variant && contains_unknown(actual) {
        substitution.insert(id, Ty::Unknown);
        return;
    }
    if kind == GenericParamKind::Plain && matches!(actual, Ty::Unknown) {
        return;
    }
    if kind == GenericParamKind::Variant
        && substitution
            .get(&id)
            .is_some_and(|current| matches!(current, Ty::Unknown))
    {
        return;
    }

    let additions = match actual {
        Ty::Union(members) if kind == GenericParamKind::Variant => members.clone(),
        actual => vec![actual.clone()],
    };
    let combined = substitution
        .remove(&id)
        .into_iter()
        .chain(additions)
        .collect::<Vec<_>>();
    substitution.insert(id, normalize_union(combined));
}

fn unify(substitution: &mut Substitution, registry: &GenericRegistry, expected: &Ty, actual: &Ty) {
    match expected {
        Ty::Generic(id) => bind_generic(substitution, registry, *id, actual),
        Ty::List(expected) => {
            if let Ty::List(actual) = actual {
                unify(substitution, registry, expected, actual);
            }
        }
        Ty::Union(members) => {
            for member in members {
                unify(substitution, registry, member, actual);
            }
        }
        Ty::Fn { ret, .. } => unify(substitution, registry, ret, actual),
        Ty::Ident(expected) => {
            if let Ty::Ident(actual) = actual {
                unify(substitution, registry, expected, actual);
            }
        }
        Ty::Number | Ty::String | Ty::Boolean | Ty::Date | Ty::Null | Ty::Unknown => {}
    }
}

fn instantiate(substitution: &Substitution, template: &Ty) -> Ty {
    match template {
        Ty::Generic(id) => substitution.get(id).cloned().unwrap_or(Ty::Unknown),
        Ty::List(inner) => Ty::List(Box::new(instantiate(substitution, inner))),
        Ty::Union(members) => normalize_union(
            members
                .iter()
                .map(|member| instantiate(substitution, member)),
        ),
        Ty::Fn { params, ret } => Ty::Fn {
            params: params
                .iter()
                .map(|(parameter, ty)| (parameter.clone(), instantiate(substitution, ty)))
                .collect(),
            ret: Box::new(instantiate(substitution, ret)),
        },
        Ty::Ident(inner) => Ty::Ident(Box::new(instantiate(substitution, inner))),
        other => other.clone(),
    }
}

fn contains_unknown(ty: &Ty) -> bool {
    match ty {
        Ty::Unknown => true,
        Ty::List(inner) | Ty::Ident(inner) => contains_unknown(inner),
        Ty::Union(members) => members.iter().any(contains_unknown),
        Ty::Fn { params, ret } => {
            params.iter().any(|(_, ty)| contains_unknown(ty)) || contains_unknown(ret)
        }
        Ty::Number | Ty::String | Ty::Boolean | Ty::Date | Ty::Null | Ty::Generic(_) => false,
    }
}

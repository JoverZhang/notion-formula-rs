//! Generic substitution and unification for signature help.
//!
//! Given a function signature with generic parameters and a set of actual
//! argument types, this module infers concrete types for each generic and
//! produces an instantiated parameter list + return type.

use analyzer::semantic;
use std::collections::HashMap;

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
    let Some(shape) = super::param_shape::complete_repeat_shape(&sig.params, arg_tys.len()) else {
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

/// Instantiates a function signature given actual argument types.
///
/// Returns `(instantiated_param_tys, instantiated_return_ty)`.
pub(super) fn instantiate_sig(
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

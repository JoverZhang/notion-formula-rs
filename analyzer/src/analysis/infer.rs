use crate::ast::{Expr, ExprKind};
use crate::{LitKind, NodeId};
use std::collections::HashMap;

use super::{Context, FunctionSig, GenericId, GenericParamKind, Ty, normalize_union};

pub type ExprId = NodeId;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypeMap {
    inner: HashMap<ExprId, Ty>,
}

impl TypeMap {
    pub fn insert(&mut self, id: ExprId, ty: Ty) {
        self.inner.insert(id, ty);
    }

    pub fn get(&self, id: ExprId) -> Option<&Ty> {
        self.inner.get(&id)
    }
}

pub(crate) type Subst = HashMap<GenericId, Ty>;
type GenericRegistry = HashMap<GenericId, GenericParamKind>;

fn registry_for(sig: &FunctionSig) -> GenericRegistry {
    sig.generics.iter().map(|g| (g.id, g.kind)).collect()
}

fn bind_generic(subst: &mut Subst, registry: &GenericRegistry, id: GenericId, actual: &Ty) {
    let kind = registry
        .get(&id)
        .copied()
        .unwrap_or(GenericParamKind::Plain);

    fn contains_unknown(ty: &Ty) -> bool {
        match ty {
            Ty::Unknown => true,
            Ty::Union(members) => members.iter().any(contains_unknown),
            _ => false,
        }
    }

    match kind {
        GenericParamKind::Plain => {
            if matches!(actual, Ty::Unknown) {
                return;
            }

            let to_add = vec![actual.clone()];
            match subst.get(&id).cloned() {
                None => {
                    subst.insert(id, normalize_union(to_add));
                }
                Some(prev) => {
                    // Plain generics: permissive accumulation on conflicts.
                    subst.insert(id, normalize_union(std::iter::once(prev).chain(to_add)));
                }
            }
        }
        GenericParamKind::Variant => {
            if contains_unknown(actual) {
                subst.insert(id, Ty::Unknown);
                return;
            }

            // Once a variant generic sees an Unknown, the result stays Unknown.
            if subst.get(&id).is_some_and(|t| matches!(t, Ty::Unknown)) {
                return;
            }

            let mut to_add: Vec<Ty> = Vec::new();
            match actual {
                Ty::Union(members) => {
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
                    subst.insert(id, normalize_union(to_add));
                }
                Some(prev) => {
                    // Variant generics: union-accumulate across all bindings.
                    subst.insert(id, normalize_union(std::iter::once(prev).chain(to_add)));
                }
            }
        }
    }
}

pub(crate) fn unify(subst: &mut Subst, registry: &GenericRegistry, expected: &Ty, actual: &Ty) {
    match expected {
        Ty::Generic(id) => bind_generic(subst, registry, *id, actual),
        Ty::List(exp_inner) => {
            if let Ty::List(act_inner) = actual {
                unify(subst, registry, exp_inner, act_inner);
            }
        }
        Ty::Union(branches) => {
            for branch in branches {
                unify(subst, registry, branch, actual);
            }
        }
        _ => {}
    }
}

pub(crate) fn apply(subst: &Subst, ty_template: &Ty) -> Ty {
    match ty_template {
        Ty::Generic(id) => subst.get(id).cloned().unwrap_or(Ty::Unknown),
        Ty::List(inner) => Ty::List(Box::new(apply(subst, inner))),
        Ty::Union(members) => normalize_union(members.iter().map(|m| apply(subst, m))),
        other => other.clone(),
    }
}

fn unify_call_args(sig: &FunctionSig, arg_tys: &[Ty], subst: &mut Subst) {
    let registry = registry_for(sig);

    if sig.params.repeat.is_empty() {
        let params = sig.params.head.iter().chain(sig.params.tail.iter());
        for (param, actual) in params.zip(arg_tys.iter()) {
            unify(subst, &registry, &param.ty, actual);
        }
        return;
    }

    let head_len = sig.params.head.len();
    let tail_used =
        super::resolve_repeat_tail_used(&sig.params, arg_tys.len()).unwrap_or(sig.params.tail.len());
    let tail_start = arg_tys.len().saturating_sub(tail_used);

    for (idx, actual) in arg_tys.iter().enumerate() {
        let expected = if idx < head_len {
            sig.params.head.get(idx)
        } else if idx >= tail_start {
            sig.params.tail.get(idx - tail_start)
        } else {
            let r_idx = (idx - head_len) % sig.params.repeat.len();
            sig.params.repeat.get(r_idx)
        };

        if let Some(param) = expected {
            unify(subst, &registry, &param.ty, actual);
        }
    }
}

fn unify_call_args_present(sig: &FunctionSig, arg_tys: &[Option<Ty>], subst: &mut Subst) {
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
    let Some(shape) = super::complete_repeat_shape(&sig.params, arg_tys.len()) else {
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

pub(crate) fn instantiate_sig(sig: &FunctionSig, arg_tys: &[Option<Ty>]) -> (Vec<Ty>, Ty) {
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

pub fn infer_expr_with_map(expr: &Expr, ctx: &Context, map: &mut TypeMap) -> Ty {
    infer_expr_inner(expr, ctx, map)
}

fn infer_expr_inner(expr: &Expr, ctx: &Context, map: &mut TypeMap) -> Ty {
    let ty = match &expr.kind {
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Number => Ty::Number,
            LitKind::String => Ty::String,
            LitKind::Bool => Ty::Boolean,
        },
        ExprKind::Ident(_) => Ty::Unknown,
        ExprKind::Group { inner } => infer_expr_with_map(inner, ctx, map),
        ExprKind::Unary { op, expr } => {
            let inner_ty = infer_expr_with_map(expr, ctx, map);
            match op.node {
                crate::ast::UnOpKind::Not => match inner_ty {
                    Ty::Boolean => Ty::Boolean,
                    _ => Ty::Unknown,
                },
                crate::ast::UnOpKind::Neg => match inner_ty {
                    Ty::Number => Ty::Number,
                    _ => Ty::Unknown,
                },
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_ty = infer_expr_with_map(left, ctx, map);
            let right_ty = infer_expr_with_map(right, ctx, map);
            use crate::ast::BinOpKind::*;
            match op.node {
                Plus | Minus | Star | Slash | Percent | Caret => {
                    if left_ty == Ty::Number && right_ty == Ty::Number {
                        Ty::Number
                    } else {
                        Ty::Unknown
                    }
                }
                AndAnd | OrOr => {
                    if left_ty == Ty::Boolean && right_ty == Ty::Boolean {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                Lt | Le | Ge | Gt => {
                    if left_ty != Ty::Unknown && right_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                EqEq | Ne => {
                    if left_ty == right_ty && left_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
            }
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            let _ = infer_expr_with_map(cond, ctx, map);
            let then_ty = infer_expr_with_map(then, ctx, map);
            let otherwise_ty = infer_expr_with_map(otherwise, ctx, map);
            join_types(then_ty, otherwise_ty)
        }
        ExprKind::Call { callee, args } => match callee.text.as_str() {
            "prop" => infer_prop(args, ctx, map),
            name => {
                let sig = ctx.functions.iter().find(|f| f.name == name);
                infer_call(name, sig, args, ctx, map)
            }
        },
        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            let _ = infer_expr_with_map(receiver, ctx, map);
            for arg in args {
                let _ = infer_expr_with_map(arg, ctx, map);
            }

            // Postfix form: `receiver.fn(arg1, ...)` corresponds to `fn(receiver, arg1, ...)`.
            if !super::postfix_capable_builtin_names().contains(method.text.as_str()) {
                Ty::Unknown
            } else {
                let sig = ctx.functions.iter().find(|f| f.name == method.text);
                let sig =
                    sig.filter(|sig| sig.flat_params().is_some_and(|params| params.len() > 1));
                if sig.is_none() {
                    Ty::Unknown
                } else {
                    let mut all_args: Vec<Expr> = Vec::with_capacity(1 + args.len());
                    all_args.push((**receiver).clone());
                    all_args.extend(args.iter().cloned());
                    infer_call(method.text.as_str(), sig, &all_args, ctx, map)
                }
            }
        }
        ExprKind::Error => Ty::Unknown,
    };

    map.insert(expr.id, ty.clone());
    ty
}

fn infer_prop(args: &[Expr], ctx: &Context, map: &mut TypeMap) -> Ty {
    for arg in args {
        let _ = infer_expr_with_map(arg, ctx, map);
    }

    if args.len() != 1 {
        return Ty::Unknown;
    }
    let arg = &args[0];
    let name = match &arg.kind {
        ExprKind::Lit(lit) if lit.kind == LitKind::String => lit.symbol.text.as_str(),
        _ => return Ty::Unknown,
    };
    ctx.lookup(name).unwrap_or(Ty::Unknown)
}

fn infer_call(
    _name: &str,
    sig: Option<&FunctionSig>,
    args: &[Expr],
    ctx: &Context,
    map: &mut TypeMap,
) -> Ty {
    let Some(sig) = sig else {
        for arg in args {
            let _ = infer_expr_with_map(arg, ctx, map);
        }
        return Ty::Unknown;
    };

    let mut arg_tys = Vec::with_capacity(args.len());
    for arg in args {
        arg_tys.push(infer_expr_with_map(arg, ctx, map));
    }

    let mut subst = Subst::new();
    unify_call_args(sig, arg_tys.as_slice(), &mut subst);

    apply(&subst, &sig.ret)
}

fn join_types(a: Ty, b: Ty) -> Ty {
    if a == Ty::Unknown || b == Ty::Unknown {
        Ty::Unknown
    } else if a == b {
        a
    } else {
        Ty::Unknown
    }
}

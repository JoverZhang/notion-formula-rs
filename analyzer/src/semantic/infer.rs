use crate::ast::{Expr, ExprKind};
use crate::token::{LitKind, NodeId};
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

    let mut to_add: Vec<Ty> = Vec::new();
    match (kind, actual) {
        (_, Ty::Unknown) => return,
        (GenericParamKind::Variant, Ty::Union(members)) => {
            to_add.extend(
                members
                    .iter()
                    .filter(|t| !matches!(t, Ty::Unknown))
                    .cloned(),
            );
        }
        (GenericParamKind::Variant, other) => {
            if !matches!(other, Ty::Unknown) {
                to_add.push(other.clone());
            }
        }
        (GenericParamKind::Plain, other) => {
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
            // Plain generics: permissive accumulation on conflicts.
            // Variant generics: union-accumulate on every binding (Unknowns filtered above).
            subst.insert(id, normalize_union(std::iter::once(prev).chain(to_add)));
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

    let registry = registry_for(sig);
    let mut subst = Subst::new();

    if sig.params.repeat.is_empty() {
        let params = sig.params.head.iter().chain(sig.params.tail.iter());
        for (param, actual) in params.zip(arg_tys.iter()) {
            unify(&mut subst, &registry, &param.ty, actual);
        }
    } else {
        let head_len = sig.params.head.len();
        let tail_used =
            resolve_repeat_tail_used(sig, arg_tys.len()).unwrap_or(sig.params.tail.len());
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
                unify(&mut subst, &registry, &param.ty, actual);
            }
        }
    }

    apply(&subst, &sig.ret)
}

fn resolve_repeat_tail_used(sig: &FunctionSig, total: usize) -> Option<usize> {
    if sig.params.repeat.is_empty() {
        return Some(sig.params.tail.len());
    }

    let head_len = sig.params.head.len();
    if total < head_len {
        return None;
    }

    let repeat_len = sig.params.repeat.len();
    let mut tail_min = 0usize;
    for (idx, p) in sig.params.tail.iter().enumerate() {
        if !p.optional {
            tail_min = idx + 1;
        }
    }

    for tail_used in (tail_min..=sig.params.tail.len()).rev() {
        if total < head_len + tail_used {
            continue;
        }
        let middle = total - head_len - tail_used;
        if middle >= repeat_len && middle.is_multiple_of(repeat_len) {
            return Some(tail_used);
        }
    }

    None
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

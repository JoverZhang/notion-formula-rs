//! Type inference and generic instantiation.
//!
//! Best-effort inference: returns [`Ty::Unknown`] when it can't determine a type. Emits no
//! diagnostics.

use crate::ast::{Expr, ExprKind, UnOp};
use crate::{LitKind, NodeId};
use std::collections::HashMap;

use super::{Context, FunctionSig, GenericId, GenericParamKind, LambdaParam, Ty, normalize_union};

/// Identifier for an expression node used as the key in [`TypeMap`].
pub type ExprId = NodeId;

/// Map from expression id to its inferred [`Ty`].
///
/// [`infer_expr_with_map`] records types for all visited [`ExprId`]s, including intermediate nodes,
/// so downstream consumers can look up types for subexpressions.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypeMap {
    inner: HashMap<ExprId, Ty>,
}

impl TypeMap {
    /// Record the inferred type for `id`.
    pub fn insert(&mut self, id: ExprId, ty: Ty) {
        self.inner.insert(id, ty);
    }

    /// Look up the inferred type for `id`, if it was visited.
    pub fn get(&self, id: ExprId) -> Option<&Ty> {
        self.inner.get(&id)
    }
}

/// Inference context carrying the type map and lexical scope stack.
///
/// The scope stack supports implicit lambda bindings: when the inference pass wraps an
/// argument in an [`ExprKind::ImplicitLambda`], it pushes a scope frame with the lambda's
/// parameter bindings before inferring the body, then pops it afterwards.
pub(crate) struct InferCtx {
    map: TypeMap,
    scopes: Vec<HashMap<String, Ty>>,
}

impl InferCtx {
    fn new(map: TypeMap) -> Self {
        Self {
            map,
            scopes: Vec::new(),
        }
    }

    /// Push a new lexical scope frame with the given bindings.
    fn push_scope(&mut self, bindings: HashMap<String, Ty>) {
        self.scopes.push(bindings);
    }

    /// Pop the innermost scope frame.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Look up a name in the scope stack (innermost first).
    fn resolve(&self, name: &str) -> Option<&Ty> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// Record the inferred type for an expression node.
    fn insert(&mut self, id: ExprId, ty: Ty) {
        self.map.insert(id, ty);
    }

    /// Consume the context and return the underlying [`TypeMap`].
    fn into_map(self) -> TypeMap {
        self.map
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
        Ty::Fn { ret, .. } => {
            // For Fn-typed params, unify the return type with the actual arg type.
            // The actual arg has not been wrapped yet — it's the raw expression type.
            unify(subst, registry, ret, actual);
        }
        Ty::Ident(inner) => {
            // Ident params carry the type that will be bound. Unify it with actual
            // (though typically actual is Unknown for bare identifiers).
            unify(subst, registry, inner, actual);
        }
        _ => {}
    }
}

pub(crate) fn apply(subst: &Subst, ty_template: &Ty) -> Ty {
    match ty_template {
        Ty::Generic(id) => subst.get(id).cloned().unwrap_or(Ty::Unknown),
        Ty::List(inner) => Ty::List(Box::new(apply(subst, inner))),
        Ty::Union(members) => normalize_union(members.iter().map(|m| apply(subst, m))),
        Ty::Fn { params, ret } => Ty::Fn {
            params: params
                .iter()
                .map(|(lp, ty)| (lp.clone(), apply(subst, ty)))
                .collect(),
            ret: Box::new(apply(subst, ret)),
        },
        Ty::Ident(inner) => Ty::Ident(Box::new(apply(subst, inner))),
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
    let tail_used = super::resolve_repeat_tail_used(&sig.params, arg_tys.len())
        .unwrap_or(sig.params.tail.len());
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

/// Infer the type of `expr` and populate `map` with types for subexpressions.
///
/// - Identifiers resolve through the lexical scope stack (for implicit lambda bindings)
///   and default to [`Ty::Unknown`] if not found.
/// - List literals infer to `List(Unknown)` if any item is unknown, otherwise `List(Union(items))`.
/// - After the desugar pass, remaining member calls fall back to [`Ty::Unknown`].
/// - Types are recorded in `map` after inferring each expression node.
/// - For `Ty::Fn`-typed parameter positions, the argument expression is wrapped in-place in
///   an [`ExprKind::ImplicitLambda`] node.
pub fn infer_expr_with_map(expr: &mut Expr, ctx: &Context, map: &mut TypeMap) -> Ty {
    let taken_map = std::mem::take(map);
    let mut ictx = InferCtx::new(taken_map);
    let ty = infer_expr(expr, ctx, &mut ictx);
    *map = ictx.into_map();
    ty
}

fn infer_expr(expr: &mut Expr, ctx: &Context, ictx: &mut InferCtx) -> Ty {
    let ty = match &mut expr.kind {
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Number => Ty::Number,
            LitKind::String => Ty::String,
            LitKind::Bool => Ty::Boolean,
        },
        ExprKind::Ident(sym) => ictx.resolve(&sym.text).cloned().unwrap_or(Ty::Unknown),
        ExprKind::Group { inner } => infer_expr(inner, ctx, ictx),
        ExprKind::List { items } => {
            fn contains_unknown(ty: &Ty) -> bool {
                match ty {
                    Ty::Unknown => true,
                    Ty::Union(members) => members.iter().any(contains_unknown),
                    _ => false,
                }
            }

            if items.is_empty() {
                Ty::List(Box::new(Ty::Unknown))
            } else {
                let mut item_tys = Vec::with_capacity(items.len());
                for item in items {
                    item_tys.push(infer_expr(item, ctx, ictx));
                }

                if item_tys.iter().any(contains_unknown) {
                    Ty::List(Box::new(Ty::Unknown))
                } else {
                    Ty::List(Box::new(normalize_union(item_tys)))
                }
            }
        }
        ExprKind::Unary { op, expr } => {
            let inner_ty = infer_expr(expr, ctx, ictx);
            match op {
                UnOp::Not(_) => match inner_ty {
                    Ty::Boolean => Ty::Boolean,
                    _ => Ty::Unknown,
                },
                UnOp::Neg => match inner_ty {
                    Ty::Number => Ty::Number,
                    _ => Ty::Unknown,
                },
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_ty = infer_expr(left, ctx, ictx);
            let right_ty = infer_expr(right, ctx, ictx);
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
            let _ = infer_expr(cond, ctx, ictx);
            let then_ty = infer_expr(then, ctx, ictx);
            let otherwise_ty = infer_expr(otherwise, ctx, ictx);
            join_types(then_ty, otherwise_ty)
        }
        ExprKind::Call { callee, args } => match callee.text.as_str() {
            "prop" => infer_prop(args, ctx, ictx),
            name => {
                let sig = ctx.functions.iter().find(|f| f.name == name);
                infer_call(sig, args, ctx, ictx)
            }
        },
        ExprKind::MemberCall { receiver, args, .. } => {
            // After the desugar pass, only non-postfix-capable member calls remain.
            // We still infer children for TypeMap population, but the result is Unknown.
            let _ = infer_expr(receiver, ctx, ictx);
            for arg in args {
                let _ = infer_expr(arg, ctx, ictx);
            }
            Ty::Unknown
        }
        ExprKind::Error => Ty::Unknown,
        ExprKind::ImplicitLambda { body, .. } => {
            // ImplicitLambda nodes are created by inference itself; if we encounter one
            // during a second pass, just infer the body.
            infer_expr(body, ctx, ictx)
        }
    };

    ictx.insert(expr.id, ty.clone());
    ty
}

fn infer_prop(args: &mut [Expr], ctx: &Context, ictx: &mut InferCtx) -> Ty {
    for arg in args.iter_mut() {
        let _ = infer_expr(arg, ctx, ictx);
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

/// Resolve the parameter at a named position and extract the identifier text.
///
/// Used for `LambdaParam::ParamRef("ident")`: given the resolved params and arguments,
/// find the param named `ref_name`, look at the corresponding argument expression, and
/// return the bare identifier text. Falls back to the ref_name itself if the argument
/// is not an `Ident`.
fn resolve_param_ref(
    ref_name: &str,
    resolved_params: &[&super::ParamSig],
    args: &[Expr],
) -> String {
    for (i, param) in resolved_params.iter().enumerate() {
        if param.name == ref_name {
            if let Some(arg) = args.get(i) {
                if let ExprKind::Ident(sym) = &arg.kind {
                    return sym.text.to_string();
                }
            }
            break;
        }
    }
    // Fallback: use the ref_name itself as the binding name.
    ref_name.to_string()
}

fn infer_call(
    sig: Option<&FunctionSig>,
    args: &mut [Expr],
    ctx: &Context,
    ictx: &mut InferCtx,
) -> Ty {
    let Some(sig) = sig else {
        for arg in args.iter_mut() {
            let _ = infer_expr(arg, ctx, ictx);
        }
        return Ty::Unknown;
    };

    // Resolve the parameter signature for each argument position.
    let resolved_params = sig.params.resolve_params(args.len());

    // Check whether any parameter is Fn-typed (lambda). If not, we can use the
    // simple single-pass path (preserving existing behaviour for non-lambda functions).
    let has_lambda_params = resolved_params
        .iter()
        .any(|p| matches!(p.ty, Ty::Fn { .. }));

    if !has_lambda_params {
        // ── Simple path: no lambdas ──────────────────────────────────────
        let mut arg_tys = Vec::with_capacity(args.len());
        for arg in args.iter_mut() {
            arg_tys.push(infer_expr(arg, ctx, ictx));
        }

        if let Some(resolver) = sig.resolver {
            let resolved = resolver(sig, &arg_tys);
            return resolved.ret;
        }

        let mut subst = Subst::new();
        unify_call_args(sig, arg_tys.as_slice(), &mut subst);
        return apply(&subst, &sig.ret);
    }

    // ── Two-pass path: has lambda params ─────────────────────────────────
    // Custom resolvers are not supported for lambda-bearing signatures.
    // If a future signature needs both, this path must be extended.
    debug_assert!(
        sig.resolver.is_none(),
        "two-pass lambda inference does not support custom SigResolver"
    );
    let registry = registry_for(sig);
    let mut subst = Subst::new();
    let mut arg_tys = vec![Ty::Unknown; args.len()];

    // Pass 1: infer non-Fn, non-Ident arguments to populate generic substitutions.
    for i in 0..args.len() {
        let Some(param) = resolved_params.get(i) else {
            continue;
        };
        match &param.ty {
            Ty::Fn { .. } => continue, // defer to pass 2
            Ty::Ident(_) => continue,  // bare identifier — skip
            _ => {
                arg_tys[i] = infer_expr(&mut args[i], ctx, ictx);
                unify(&mut subst, &registry, &param.ty, &arg_tys[i]);
            }
        }
    }

    // Pass 2: process Ident and Fn-typed arguments with substitutions available.
    for i in 0..args.len() {
        let Some(param) = resolved_params.get(i) else {
            continue;
        };
        match &param.ty {
            Ty::Ident(inner_ty) => {
                // Record the bound type for downstream ParamRef resolution.
                let bound_ty = apply(&subst, inner_ty.as_ref());
                arg_tys[i] = Ty::Ident(Box::new(bound_ty));
                // Also record in ictx for the arg expression itself.
                ictx.insert(args[i].id, arg_tys[i].clone());
            }
            Ty::Fn {
                params: fn_params,
                ret,
            } => {
                // Resolve lambda parameter names and types.
                let mut bindings = HashMap::new();
                let mut param_names = Vec::new();

                for (lp, lp_ty) in fn_params {
                    let (name, ty) = match lp {
                        LambdaParam::Current => ("current".to_string(), apply(&subst, lp_ty)),
                        LambdaParam::ParamRef(ref_name) => {
                            let ident_text = resolve_param_ref(ref_name, &resolved_params, args);
                            let ty = apply(&subst, lp_ty);
                            (ident_text, ty)
                        }
                    };
                    bindings.insert(name.clone(), ty);
                    param_names.push(name);
                }

                // Push scope, infer body, pop scope.
                ictx.push_scope(bindings);
                let body_ty = infer_expr(&mut args[i], ctx, ictx);
                ictx.pop_scope();

                // Unify the body type with the return type of the Fn (raw, not applied).
                // Using the raw ret preserves generic identifiers so bind_generic can
                // accumulate them (e.g. Variant generics building unions across branches).
                unify(&mut subst, &registry, ret, &body_ty);
                arg_tys[i] = body_ty.clone();

                // Wrap the argument in an ImplicitLambda node in-place.
                let original_arg = std::mem::replace(
                    &mut args[i],
                    Expr {
                        id: 0,
                        span: crate::Span { start: 0, end: 0 },
                        kind: ExprKind::Error,
                    },
                );
                args[i] = Expr {
                    id: super::next_synthetic_id(),
                    span: original_arg.span,
                    kind: ExprKind::ImplicitLambda {
                        params: param_names,
                        body: Box::new(original_arg),
                    },
                };

                // Record the body type for the wrapper node too.
                ictx.insert(args[i].id, body_ty);
            }
            _ => {} // already inferred in pass 1
        }
    }

    apply(&subst, &sig.ret)
}

fn join_types(a: Ty, b: Ty) -> Ty {
    if a == Ty::Unknown || b == Ty::Unknown {
        Ty::Unknown
    } else {
        normalize_union([a, b])
    }
}

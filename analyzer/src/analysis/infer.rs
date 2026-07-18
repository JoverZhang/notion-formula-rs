//! Type inference and generic instantiation.
//!
//! Best-effort inference: returns [`Ty::Unknown`] when it can't determine a type. Emits no
//! diagnostics.

use crate::ast::{Expr, ExprKind, UnOp};
use crate::{LitKind, NodeId};
use std::collections::HashMap;

use super::{
    ArgumentObservation, CallSignatureInput, Context, FunctionSig, LambdaParam,
    ResolvedFunctionSig, Ty, normalize_union, param_for_ref, resolve_call_signature,
};

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

/// Semantic facts retained for downstream planning after inference completes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SemanticMap {
    pub expression_types: TypeMap,
    pub builtin_calls: HashMap<ExprId, ResolvedFunctionSig>,
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
    builtin_calls: HashMap<ExprId, ResolvedFunctionSig>,
    scopes: Vec<HashMap<String, Ty>>,
}

impl InferCtx {
    fn new(map: SemanticMap) -> Self {
        Self {
            map: map.expression_types,
            builtin_calls: map.builtin_calls,
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

    fn insert_call(&mut self, id: ExprId, signature: ResolvedFunctionSig) {
        self.builtin_calls.insert(id, signature);
    }

    /// Consume the context and return all retained semantic facts.
    fn into_map(self) -> SemanticMap {
        SemanticMap {
            expression_types: self.map,
            builtin_calls: self.builtin_calls,
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
    let mut semantic_map = SemanticMap {
        expression_types: std::mem::take(map),
        builtin_calls: HashMap::new(),
    };
    let ty = infer_expr_with_semantic_map(expr, ctx, &mut semantic_map);
    *map = semantic_map.expression_types;
    ty
}

/// Infer expression types and retain final resolved contracts for every builtin call.
pub fn infer_expr_with_semantic_map(expr: &mut Expr, ctx: &Context, map: &mut SemanticMap) -> Ty {
    let taken_map = std::mem::take(map);
    let mut ictx = InferCtx::new(taken_map);
    let ty = infer_expr(expr, ctx, &mut ictx);
    *map = ictx.into_map();
    ty
}

fn infer_expr(expr: &mut Expr, ctx: &Context, ictx: &mut InferCtx) -> Ty {
    let expr_id = expr.id;
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
                infer_call(expr_id, sig, args, ctx, ictx)
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
    signature: &FunctionSig,
    resolved: &ResolvedFunctionSig,
    args: &[Expr],
) -> String {
    for (i, argument) in resolved.arguments.iter().enumerate() {
        if argument
            .parameter
            .is_some_and(|reference| param_for_ref(signature, reference).name == ref_name)
        {
            if let Some(arg) = args.get(i)
                && let ExprKind::Ident(sym) = &arg.kind
            {
                return sym.text.to_string();
            }
            break;
        }
    }
    // Fallback: use the ref_name itself as the binding name.
    ref_name.to_string()
}

fn infer_call(
    call_id: ExprId,
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

    let mut observations = vec![ArgumentObservation::Empty; args.len()];
    let initial = resolve_call_signature(
        sig,
        CallSignatureInput {
            arguments: &observations,
        },
    );

    // The shared projection owns positional shape. The Analyzer only interprets the
    // projected semantic parameter kind to decide which expressions need staged inference.
    let has_lambda_params = initial
        .arguments
        .iter()
        .filter_map(|argument| argument.parameter)
        .any(|reference| matches!(param_for_ref(sig, reference).ty, Ty::Fn { .. }));

    if !has_lambda_params {
        for (argument, observation) in args.iter_mut().zip(&mut observations) {
            *observation = ArgumentObservation::Typed(infer_expr(argument, ctx, ictx));
        }
        let resolved = resolve_call_signature(
            sig,
            CallSignatureInput {
                arguments: &observations,
            },
        );
        let return_ty = resolved.return_ty.clone();
        ictx.insert_call(call_id, resolved);
        return return_ty;
    }

    // Phase 1: observe ordinary arguments without entering lambda/binder positions.
    for i in 0..args.len() {
        let Some(reference) = initial
            .arguments
            .get(i)
            .and_then(|argument| argument.parameter)
        else {
            observations[i] = ArgumentObservation::Typed(infer_expr(&mut args[i], ctx, ictx));
            continue;
        };
        let param = param_for_ref(sig, reference);
        match &param.ty {
            Ty::Fn { .. } | Ty::Ident(_) => continue,
            _ => {
                observations[i] = ArgumentObservation::Typed(infer_expr(&mut args[i], ctx, ictx));
            }
        }
    }

    // Phase 2: resolve an immutable partial snapshot, infer each deferred expression with
    // its instantiated lambda parameters, then resolve again. Re-resolving after each
    // deferred argument supports signatures whose later lambda inputs depend on earlier
    // lambda results without retaining mutable generic state in the Analyzer.
    let mut staged = resolve_call_signature(
        sig,
        CallSignatureInput {
            arguments: &observations,
        },
    );
    for i in 0..args.len() {
        let Some(reference) = initial
            .arguments
            .get(i)
            .and_then(|argument| argument.parameter)
        else {
            continue;
        };
        let param = param_for_ref(sig, reference);
        match &param.ty {
            Ty::Ident(_) => {
                let observed = staged
                    .arguments
                    .get(i)
                    .and_then(|argument| argument.expected_ty.as_ref())
                    .cloned()
                    .unwrap_or_else(|| Ty::Ident(Box::new(Ty::Unknown)));
                observations[i] = ArgumentObservation::Typed(observed.clone());
                ictx.insert(args[i].id, observed);
            }
            Ty::Fn { .. } => {
                let expected = staged
                    .arguments
                    .get(i)
                    .and_then(|argument| argument.expected_ty.as_ref());
                let Ty::Fn {
                    params: fn_params, ..
                } = expected.unwrap_or(&param.ty)
                else {
                    unreachable!("Fn template must resolve to Fn expected type")
                };
                let mut bindings = HashMap::new();
                let mut param_names = Vec::new();

                for (lp, lp_ty) in fn_params {
                    let (name, ty) = match lp {
                        LambdaParam::Current => ("current".to_string(), lp_ty.clone()),
                        LambdaParam::ParamRef(ref_name) => {
                            let ident_text = resolve_param_ref(ref_name, sig, &staged, args);
                            (ident_text, lp_ty.clone())
                        }
                    };
                    bindings.insert(name.clone(), ty);
                    param_names.push(name);
                }

                // Push scope, infer body, pop scope.
                ictx.push_scope(bindings);
                let body_ty = infer_expr(&mut args[i], ctx, ictx);
                ictx.pop_scope();
                observations[i] = ArgumentObservation::Typed(body_ty.clone());

                // Wrap the argument in an ImplicitLambda node in-place.
                if !matches!(args[i].kind, ExprKind::ImplicitLambda { .. }) {
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
                }

                // Record the body type for the wrapper node too.
                ictx.insert(args[i].id, body_ty);
            }
            _ => {} // already inferred in pass 1
        }
        staged = resolve_call_signature(
            sig,
            CallSignatureInput {
                arguments: &observations,
            },
        );
    }

    let return_ty = staged.return_ty.clone();
    ictx.insert_call(call_id, staged);
    return_ty
}

fn join_types(a: Ty, b: Ty) -> Ty {
    if a == Ty::Unknown || b == Ty::Unknown {
        Ty::Unknown
    } else {
        normalize_union([a, b])
    }
}

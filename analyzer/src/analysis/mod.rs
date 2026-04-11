//! Semantic analysis and type model for formulas.
//!
//! This layer infers a best-effort [`Ty`] for expressions and validates calls against builtin
//! [`FunctionSig`]s plus the special-cased `prop("Name")` form.

use crate::ast::{Expr, ExprKind};
use crate::diagnostics::{Diagnostic, DiagnosticCode, DiagnosticKind};
use crate::{LitKind, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};

mod desugar;
mod infer;
pub use builtin_fn::{
    FunctionCategory, FunctionSig, GenericId, GenericParam, GenericParamKind, LambdaParam,
    ParamShape, ParamSig, SigResolver, Ty, builtins_functions, normalize_union,
};
pub use infer::{ExprId, TypeMap, infer_expr_with_map};

/// Global counter for synthetic expression IDs created during inference (e.g. `ImplicitLambda`
/// wrapper nodes). Starts at `u32::MAX / 2` to avoid collisions with parser-allocated IDs.
static NEXT_SYNTHETIC_ID: AtomicU32 = AtomicU32::new(u32::MAX / 2);

/// Allocate a fresh [`NodeId`] for a synthetic AST node created outside the parser.
pub fn next_synthetic_id() -> crate::lexer::NodeId {
    NEXT_SYNTHETIC_ID.fetch_add(1, Ordering::Relaxed)
}

static POSTFIX_CAPABLE_BUILTIN_NAMES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    builtins_functions()
        .into_iter()
        .filter(is_postfix_capable)
        .map(|sig| sig.name)
        .collect()
});

/// Return the set of builtin function names that support postfix-call sugar.
///
/// A member call `receiver.name(args...)` is eligible for semantic treatment as
/// `name(receiver, args...)` when:
/// - `name` resolves to a builtin [`FunctionSig`], and
/// - [`is_postfix_capable`] is true for that signature.
pub fn postfix_capable_builtin_names() -> &'static HashSet<String> {
    &POSTFIX_CAPABLE_BUILTIN_NAMES
}

/// Returns true if `receiver.<name>(...)` can be treated as `<name>(receiver, ...)` deterministically.
///
/// This gate is used by:
/// - the postfix-capable builtin allowlist
/// - semantic inference for member calls
/// - signature help postfix rendering
pub fn is_postfix_capable(sig: &FunctionSig) -> bool {
    // Postfix calls must have a deterministic "first parameter slot" and at least one additional
    // parameter to be supplied inside `( ... )`.
    //
    // Deterministic first slot:
    // - `head[0]` if head is non-empty
    // - else `repeat[0]` if repeat is non-empty (repeat_min_groups is 1 in this repo)
    // - else not postfix-capable (tail-only signatures are excluded by design)
    if !sig.params.head.is_empty() {
        return sig.display_params_len() >= 2;
    }
    if !sig.params.repeat.is_empty() {
        return sig.display_params_len() >= 2;
    }
    false
}

/// Returns whether `actual` is accepted by `expected` for semantic validation.
///
/// Currently:
/// - `actual = Unknown` is accepted (avoids cascading mismatch noise when inference is unsure).
/// - `expected = Generic(_)` is treated as a wildcard (the inferred actual being `Generic(_)` does
///   *not* imply wildcarding).
/// - `Union` uses containment semantics (unions are treated as sets for acceptance checks).
/// - `List` is covariant: `List(E)` accepts `List(A)` iff `E` accepts `A`.
pub fn ty_accepts(expected: &Ty, actual: &Ty) -> bool {
    if matches!(actual, Ty::Unknown) {
        return true;
    }
    // Generics wildcard only when the *expected* side is generic.
    // The inferred "actual" type being Generic(...) must not silently pass validation.
    if matches!(expected, Ty::Generic(_)) {
        return true;
    }
    match (expected, actual) {
        // A Fn-typed param accepts any expression — the inference pass handles wrapping.
        // During validation the wrapped ImplicitLambda's body type is checked against ret.
        (
            Ty::Fn {
                ret: expected_ret, ..
            },
            actual,
        ) => ty_accepts(expected_ret, actual),
        // Ident-typed positions are internal annotations for binder names — they should
        // not participate in user-visible type matching. validate_call short-circuits
        // before reaching ty_accepts for Ident params, but IDE callers may hit this path.
        (Ty::Ident(_), _) => true,
        (Ty::Union(_), Ty::Union(actual_members)) => {
            actual_members.iter().all(|a| ty_accepts(expected, a))
        }
        (Ty::Union(branches), actual) => branches.iter().any(|t| ty_accepts(t, actual)),
        (expected, Ty::Union(actual_members)) => {
            actual_members.iter().all(|a| ty_accepts(expected, a))
        }
        (Ty::List(e), Ty::List(a)) => ty_accepts(e, a),
        _ => expected == actual,
    }
}

/// A property available to `prop("Name")` calls and to editor completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    /// Canonical property name as referenced by `prop("...")`.
    pub name: String,
    #[serde(rename = "type")]
    /// Declared property type.
    pub ty: Ty,
    /// If set, editor completions may surface this item as disabled and provide this reason.
    pub disabled_reason: Option<String>,
}

/// Semantic environment used for validation and editor features.
///
/// - `properties` are supplied externally (e.g. by the WASM layer via JSON) and used by `prop(...)`.
/// - `functions` are sourced from Rust builtins at the WASM boundary (JS cannot supply them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub properties: Vec<Property>,
    pub functions: Vec<FunctionSig>,
}

impl Context {
    /// Look up a property type by name.
    ///
    /// Currently this is used for `prop("Name")` resolution.
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.ty.clone())
    }
}

/// Infer the expression type and emit semantic diagnostics.
///
/// Returns `(root_type, diagnostics)`.
///
/// Currently diagnostics are validation-first:
/// - Calls are checked for arity/shape errors first; on a shape error, validation emits exactly one
///   diagnostic for that call and does not emit per-argument type mismatches for the same call.
/// - `prop("Name")` is special-cased (it is not modeled as a [`FunctionSig`]).
/// - Postfix member calls may be treated as calls when the callee is a postfix-capable builtin (see
///   [`is_postfix_capable`]).
pub fn analyze_expr(expr: &mut Expr, ctx: &Context) -> (Ty, Vec<Diagnostic>) {
    desugar::desugar_member_calls(expr);

    let mut map = TypeMap::default();
    let ty = infer_expr_with_map(expr, ctx, &mut map);

    let mut diags = Vec::new();
    validate_expr(expr, ctx, &map, &mut diags);

    (ty, diags)
}

fn lookup_function<'a>(ctx: &'a Context, name: &str) -> Option<&'a FunctionSig> {
    ctx.functions.iter().find(|f| f.name == name)
}

fn validate_expr(expr: &Expr, ctx: &Context, map: &TypeMap, diags: &mut Vec<Diagnostic>) {
    match &expr.kind {
        ExprKind::Lit(_) | ExprKind::Ident(_) | ExprKind::Error => {}
        ExprKind::Group { inner } => validate_expr(inner, ctx, map, diags),
        ExprKind::List { items } => {
            for item in items {
                validate_expr(item, ctx, map, diags);
            }
        }
        ExprKind::Unary { expr, .. } => validate_expr(expr, ctx, map, diags),
        ExprKind::Binary { left, right, .. } => {
            validate_expr(left, ctx, map, diags);
            validate_expr(right, ctx, map, diags);
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            validate_expr(cond, ctx, map, diags);
            validate_expr(then, ctx, map, diags);
            validate_expr(otherwise, ctx, map, diags);
        }
        ExprKind::Call { callee, args } => {
            for arg in args {
                validate_expr(arg, ctx, map, diags);
            }

            match callee.text.as_str() {
                "prop" => validate_prop_call(expr, args, ctx, diags),
                name => {
                    let Some(sig) = lookup_function(ctx, name) else {
                        emit_error(diags, expr.span, format!("unknown function: {}", name));
                        return;
                    };
                    validate_call(expr.span, name, sig, args, ctx, map, diags);
                }
            }
        }
        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            validate_expr(receiver, ctx, map, diags);
            for arg in args {
                validate_expr(arg, ctx, map, diags);
            }

            let Some(sig) = lookup_function(ctx, method.text.as_str()) else {
                return;
            };

            // Postfix form: `receiver.fn(arg1, ...)` is treated like `fn(receiver, arg1, ...)`.
            if !postfix_capable_builtin_names().contains(sig.name.as_str()) {
                return;
            }
            let Some(flat) = sig.flat_params() else {
                return;
            };
            if flat.len() <= 1 {
                return;
            }

            let mut all_args: Vec<Expr> = Vec::with_capacity(1 + args.len());
            all_args.push((**receiver).clone());
            all_args.extend(args.iter().cloned());
            validate_call(
                expr.span,
                method.text.as_str(),
                sig,
                &all_args,
                ctx,
                map,
                diags,
            );
        }
        ExprKind::ImplicitLambda { body, .. } => {
            validate_expr(body, ctx, map, diags);
        }
    }
}

fn validate_prop_call(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) {
    if args.len() != 1 {
        emit_error(diags, expr.span, "prop() expects exactly 1 argument");
        return;
    }

    let arg = &args[0];
    let name = match &arg.kind {
        ExprKind::Lit(lit) if lit.kind == LitKind::String => lit.symbol.text.as_str(),
        _ => {
            emit_error(diags, arg.span, "prop() expects a string literal argument");
            return;
        }
    };

    if ctx.lookup(name).is_none() {
        emit_error(diags, arg.span, format!("Unknown property: {}", name));
    }
}

fn validate_call(
    call_span: Span,
    name: &str,
    sig: &FunctionSig,
    args: &[Expr],
    _ctx: &Context,
    map: &TypeMap,
    diags: &mut Vec<Diagnostic>,
) {
    if !validate_arity(call_span, name, sig, args.len(), diags) {
        return;
    }

    for (idx, arg) in args.iter().enumerate() {
        let Some(param) = param_for_arg_index_with_total(sig, idx, args.len()) else {
            continue;
        };

        // Ident-typed params require a bare identifier expression.
        if matches!(param.ty, Ty::Ident(_)) {
            if !matches!(arg.kind, ExprKind::Ident(_)) {
                emit_error(
                    diags,
                    arg.span,
                    format!("{}() expects a variable name for `{}`", name, param.name),
                );
            }
            continue;
        }

        let actual = map.get(arg.id).cloned().unwrap_or(Ty::Unknown);
        if !ty_accepts(&param.ty, &actual) {
            if name == "sum" {
                emit_error(diags, arg.span, "sum() expects number arguments");
            } else {
                emit_error(
                    diags,
                    arg.span,
                    format!(
                        "argument type mismatch: expected {:?}, got {:?}",
                        param.ty, actual
                    ),
                );
            }
        }
    }
}

/// Returns `true` when the call has a valid arity/shape for `sig`.
///
/// On invalid arity/shape, this function emits exactly one error diagnostic and returns `false`
/// so callers can early-return without producing cascading diagnostics.
fn validate_arity(
    call_span: Span,
    name: &str,
    sig: &FunctionSig,
    arg_len: usize,
    diags: &mut Vec<Diagnostic>,
) -> bool {
    let required = sig.required_min_args();
    let head_len = sig.params.head.len();
    let repeat_len = sig.params.repeat.len();
    let tail_len = sig.params.tail.len();

    // Fixed arity: no repeat group.
    if repeat_len == 0 {
        let max = head_len + tail_len;
        if required == max {
            if arg_len != max {
                let plural = if max == 1 { "" } else { "s" };
                emit_error(
                    diags,
                    call_span,
                    format!("{name}() expects exactly {max} argument{plural}"),
                );
                return false;
            }
            return true;
        }

        if arg_len < required {
            let plural = if required == 1 { "" } else { "s" };
            emit_error(
                diags,
                call_span,
                format!("{name}() expects at least {required} argument{plural}"),
            );
            return false;
        }

        if arg_len > max {
            let plural = if max == 1 { "" } else { "s" };
            emit_error(
                diags,
                call_span,
                format!("{name}() expects at most {max} argument{plural}"),
            );
            return false;
        }

        return true;
    }

    // Repeat-group: head + (repeat group 1+) + tail (tail may be partially present if optional).
    if arg_len < required {
        let plural = if required == 1 { "" } else { "s" };
        emit_error(
            diags,
            call_span,
            format!("{name}() expects at least {required} argument{plural}"),
        );
        return false;
    }

    if resolve_repeat_tail_used(&sig.params, arg_len).is_none() {
        emit_error(
            diags,
            call_span,
            format!("{name}() has an invalid argument shape"),
        );
        return false;
    }

    true
}

fn param_for_arg_index_with_total(
    sig: &FunctionSig,
    idx: usize,
    total: usize,
) -> Option<&ParamSig> {
    if sig.params.repeat.is_empty() {
        if idx < sig.params.head.len() {
            return sig.params.head.get(idx);
        }
        return sig
            .params
            .tail
            .get(idx.saturating_sub(sig.params.head.len()));
    }

    let head_len = sig.params.head.len();
    let tail_used = resolve_repeat_tail_used(&sig.params, total)?;
    let tail_start = total.saturating_sub(tail_used);

    if idx < head_len {
        return sig.params.head.get(idx);
    }
    if idx >= tail_start {
        return sig.params.tail.get(idx - tail_start);
    }

    let idx = idx.saturating_sub(head_len);
    sig.params.repeat.get(idx % sig.params.repeat.len())
}

pub(crate) fn resolve_repeat_tail_used(params: &ParamShape, total: usize) -> Option<usize> {
    builtin_fn::resolve_repeat_tail_used(params, total)
}

fn emit_error(diags: &mut Vec<Diagnostic>, span: Span, message: impl Into<String>) {
    diags.push(Diagnostic {
        kind: DiagnosticKind::Error,
        code: DiagnosticCode::SemanticError,
        message: message.into(),
        span,
        labels: vec![],
        notes: vec![],
        actions: vec![],
    });
}

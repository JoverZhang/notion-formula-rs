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
pub use builtin_fn::type_accepts as ty_accepts;
pub use builtin_fn::{
    ArgumentObservation, ArgumentTypeStatus, CallShapeError, CallSignatureInput, FunctionCategory,
    FunctionSig, GenericId, GenericParam, GenericParamKind, LambdaParam, ParamRef, ParamShape,
    ParamSig, ResolvedFunctionSig, ResolverInput, ShapeValidity, SigResolver, Ty,
    builtins_functions, normalize_union, param_for_ref, resolve_call_signature, type_accepts,
};
pub use infer::{ExprId, SemanticMap, TypeMap, infer_expr_with_map, infer_expr_with_semantic_map};

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
        // A repeat-only declaration has one logical slot but may require multiple
        // physical groups. This keeps `concat` postfix-capable after moving from legacy
        // numbered parameters to `repeat(min = 2)` without making one-group reducers
        // such as `sum` newly postfix-capable.
        return sig.display_params_len() >= 2
            || sig.params.repeat.len() * sig.params.repeat_min_groups >= 2;
    }
    false
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
    let (ty, _, diagnostics) = analyze_expr_with_semantic_map(expr, ctx);
    (ty, diagnostics)
}

/// Analyze an expression and retain the final resolved contract for every builtin call.
pub fn analyze_expr_with_semantic_map(
    expr: &mut Expr,
    ctx: &Context,
) -> (Ty, SemanticMap, Vec<Diagnostic>) {
    desugar::desugar_member_calls(expr);

    let mut map = SemanticMap::default();
    let ty = infer_expr_with_semantic_map(expr, ctx, &mut map);

    let mut diags = Vec::new();
    validate_expr(expr, ctx, &map, &mut diags);

    (ty, map, diags)
}

fn lookup_function<'a>(ctx: &'a Context, name: &str) -> Option<&'a FunctionSig> {
    ctx.functions.iter().find(|f| f.name == name)
}

fn validate_expr(expr: &Expr, ctx: &Context, map: &SemanticMap, diags: &mut Vec<Diagnostic>) {
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
                    validate_call(expr, name, sig, args, map, diags);
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

            // Postfix-capable builtins were desugared into normal calls before inference.
            // Any member call left here is therefore unsupported or unknown.
            let method_name = method.text.as_str();
            let message = if method_name == "prop" || lookup_function(ctx, method_name).is_some() {
                format!("{}() does not support postfix calls", method.text)
            } else {
                format!("unknown function: {}", method.text)
            };
            emit_error(diags, expr.span, message);
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
    call: &Expr,
    name: &str,
    sig: &FunctionSig,
    args: &[Expr],
    map: &SemanticMap,
    diags: &mut Vec<Diagnostic>,
) {
    let Some(resolved) = map.builtin_calls.get(&call.id) else {
        return;
    };
    if let ShapeValidity::Invalid(error) = &resolved.validity {
        emit_shape_error(diags, call.span, name, sig, error);
        return;
    }

    for (arg, argument) in args.iter().zip(&resolved.arguments) {
        let Some(reference) = argument.parameter else {
            continue;
        };
        let param = param_for_ref(sig, reference);

        // Ident-typed params require a bare identifier expression.
        if argument
            .expected_ty
            .as_ref()
            .is_some_and(|expected| matches!(expected, Ty::Ident(_)))
        {
            if !matches!(arg.kind, ExprKind::Ident(_)) {
                emit_error(
                    diags,
                    arg.span,
                    format!("{}() expects a variable name for `{}`", name, param.name),
                );
            }
            continue;
        }

        let ArgumentTypeStatus::Mismatch { actual } = &argument.type_status else {
            continue;
        };
        if name == "sum" {
            emit_error(diags, arg.span, "sum() expects number arguments");
        } else {
            emit_error(
                diags,
                arg.span,
                format!(
                    "argument type mismatch: expected {:?}, got {:?}",
                    argument.expected_ty.as_ref().unwrap_or(&param.ty),
                    actual
                ),
            );
        }
    }
}

fn emit_shape_error(
    diags: &mut Vec<Diagnostic>,
    call_span: Span,
    name: &str,
    sig: &FunctionSig,
    error: &CallShapeError,
) {
    let message = match error {
        CallShapeError::TooFew { minimum, .. }
            if sig.params.repeat.is_empty()
                && *minimum == sig.params.head.len() + sig.params.tail.len() =>
        {
            let plural = if *minimum == 1 { "" } else { "s" };
            format!("{name}() expects exactly {minimum} argument{plural}")
        }
        CallShapeError::TooFew { minimum, .. } => {
            let plural = if *minimum == 1 { "" } else { "s" };
            format!("{name}() expects at least {minimum} argument{plural}")
        }
        CallShapeError::TooMany { maximum, .. } if sig.required_min_args() == *maximum => {
            let plural = if *maximum == 1 { "" } else { "s" };
            format!("{name}() expects exactly {maximum} argument{plural}")
        }
        CallShapeError::TooMany { maximum, .. } => {
            let plural = if *maximum == 1 { "" } else { "s" };
            format!("{name}() expects at most {maximum} argument{plural}")
        }
        CallShapeError::InvalidRepeat { .. } => {
            format!("{name}() has an invalid argument shape")
        }
    };
    emit_error(diags, call_span, message);
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

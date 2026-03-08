//! Tests for SigResolver infrastructure and new builtin signatures.
//!
//! - `flat()` uses a custom resolver for depth-sensitive return types.
//! - `padStart`, `padEnd`, `formatNumber`, `splice` are new builtins added alongside the resolver.

use crate::semantic::{self, builtins_functions, Context, Ty};
use crate::{analyze_syntax, Span};

fn infer_ok(source: &str, ctx: &Context) -> Ty {
    let output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (ty, diags) = semantic::analyze_expr(&output.expr, ctx);
    assert!(
        diags.is_empty(),
        "unexpected semantic diagnostics: {:?}",
        diags
    );
    ty
}

fn infer_with_diags(source: &str, ctx: &Context) -> (Ty, Vec<crate::Diagnostic>) {
    let output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    semantic::analyze_expr(&output.expr, ctx)
}

fn assert_single_diag(source: &str, ctx: &Context, message: &str, span: Span) {
    let output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&output.expr, ctx);
    assert_eq!(diags.len(), 1, "expected 1 diagnostic, got: {:?}", diags);
    assert_eq!(diags[0].message, message);
    assert_eq!(diags[0].span, span);
}

fn builtins_ctx() -> Context {
    Context {
        properties: vec![],
        functions: builtins_functions(),
    }
}

// ---------------------------------------------------------------------------
// flat() -- SigResolver tests
// ---------------------------------------------------------------------------

#[test]
fn flat_nested_list_unwraps_one_level() {
    // flat(number[][]) -> number[]
    // We build a nested list literal: [[1, 2], [3]]
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([[1, 2], [3]])", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn flat_already_flat_list_returns_same_type() {
    // flat(number[]) -> number[] (no deeper nesting)
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([1, 2, 3])", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn flat_triple_nested_deep_flattens() {
    // flat(number[][][]) -> number[] (deep flatten, not just one level)
    // [[[1, 2]]] is number[][][]
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([[[1, 2]]])", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn flat_mixed_types_produces_union() {
    // flat([1, ["hello"]]) -> (number | string)[]
    // The outer list is (number | string[])[], flattening collects leaves: number, string
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([1, [\"hello\"]])", &ctx);
    // normalize_union sorts: Number < String
    assert_eq!(
        ty,
        Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])))
    );
}

#[test]
fn flat_unknown_list_returns_generic_fallback() {
    // flat(unknown[]) -> unknown[] (via generic fallback T0[])
    // Using an identifier `x` gives Ty::Unknown, wrapping in a list gives List(Unknown)
    let ctx = builtins_ctx();
    let (ty, diags) = infer_with_diags("flat([x])", &ctx);
    // [x] infers to List(Unknown); flat(List(Unknown)) -> resolver sees non-List inner, returns
    // the sig.ret fallback which after generic unification would be List(Unknown).
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    assert_eq!(ty, Ty::List(Box::new(Ty::Unknown)));
}

#[test]
fn flat_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "flat()",
        &ctx,
        "flat() expects exactly 1 argument",
        Span { start: 0, end: 6 },
    );
}

#[test]
fn flat_too_many_args() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "flat([1], [2])",
        &ctx,
        "flat() expects exactly 1 argument",
        Span { start: 0, end: 14 },
    );
}

// ---------------------------------------------------------------------------
// padStart / padEnd
// ---------------------------------------------------------------------------

#[test]
fn pad_start_returns_string() {
    let ctx = builtins_ctx();
    let ty = infer_ok("padStart(\"hello\", 10, \" \")", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn pad_start_accepts_number_as_text() {
    let ctx = builtins_ctx();
    let ty = infer_ok("padStart(42, 5, \"0\")", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn pad_end_returns_string() {
    let ctx = builtins_ctx();
    let ty = infer_ok("padEnd(\"hello\", 10, \" \")", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn pad_start_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "padStart(\"x\", 5)",
        &ctx,
        "padStart() expects exactly 3 arguments",
        Span { start: 0, end: 16 },
    );
}

// ---------------------------------------------------------------------------
// formatNumber
// ---------------------------------------------------------------------------

#[test]
fn format_number_returns_string() {
    let ctx = builtins_ctx();
    let ty = infer_ok("formatNumber(3.14, \"percent\", 2)", &ctx);
    assert_eq!(ty, Ty::String);
}

#[test]
fn format_number_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "formatNumber(3.14, \"percent\")",
        &ctx,
        "formatNumber() expects exactly 3 arguments",
        Span { start: 0, end: 29 },
    );
}

// ---------------------------------------------------------------------------
// splice
// ---------------------------------------------------------------------------

#[test]
fn splice_returns_list_of_same_type() {
    let ctx = builtins_ctx();
    let ty = infer_ok("splice([1, 2, 3], 1, 1)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn splice_with_insert_items() {
    let ctx = builtins_ctx();
    let ty = infer_ok("splice([1, 2, 3], 1, 0, 10, 20)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn splice_arity_error_too_few() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "splice([1], 0)",
        &ctx,
        "splice() expects at least 3 arguments",
        Span { start: 0, end: 14 },
    );
}

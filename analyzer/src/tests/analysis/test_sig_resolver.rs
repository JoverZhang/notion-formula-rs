//! Tests for SigResolver infrastructure and new builtin signatures.
//!
//! - `empty()` and `flat()` refine their return types with custom resolvers.
//! - `padStart`, `padEnd`, `formatNumber`, `splice` are new builtins added alongside the resolver.

use crate::semantic::{self, Context, Ty, builtins_functions};
use crate::{Span, analyze_syntax};

fn infer_ok(source: &str, ctx: &Context) -> Ty {
    let mut output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (ty, diags) = semantic::analyze_expr(&mut output.expr, ctx);
    assert!(
        diags.is_empty(),
        "unexpected semantic diagnostics: {:?}",
        diags
    );
    ty
}

fn infer_with_diags(source: &str, ctx: &Context) -> (Ty, Vec<crate::Diagnostic>) {
    let mut output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    semantic::analyze_expr(&mut output.expr, ctx)
}

fn assert_single_diag(source: &str, ctx: &Context, message: &str, span: Span) {
    let mut output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&mut output.expr, ctx);
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
// empty() -- SigResolver tests
// ---------------------------------------------------------------------------

#[test]
fn empty_return_type_depends_on_arity() {
    let ctx = builtins_ctx();
    assert_eq!(infer_ok("empty()", &ctx), Ty::Unknown);
    assert_eq!(infer_ok("empty(0)", &ctx), Ty::Boolean);
    assert_eq!(infer_ok("empty(empty())", &ctx), Ty::Boolean);
    assert_eq!(
        infer_ok("[1, empty(), 2]", &ctx),
        Ty::List(Box::new(Ty::Unknown))
    );
}

#[test]
fn empty_extra_arguments_still_report_an_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "empty(1, 2)",
        &ctx,
        "empty() expects at most 1 argument",
        Span { start: 0, end: 11 },
    );
    assert_eq!(infer_with_diags("empty(1, 2)", &ctx).0, Ty::Number);
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
fn flat_triple_nested_preserves_the_remaining_list_layer() {
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([[[1, 2]]])", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::List(Box::new(Ty::Number)))));
}

#[test]
fn flat_union_of_list_types_flattens_each_member_once() {
    let ctx = builtins_ctx();
    let ty = infer_ok("flat(if(true, [[1]], [[[\"x\"]]]))", &ctx);
    assert_eq!(
        ty,
        Ty::List(Box::new(Ty::Union(vec![
            Ty::Number,
            Ty::List(Box::new(Ty::String)),
        ])))
    );
}

#[test]
fn flat_mixed_types_produces_union() {
    // flat([1, ["hello"]]) -> (number | string)[]
    // The outer list is (number | string[])[]; one-level flattening yields number | string.
    let ctx = builtins_ctx();
    let ty = infer_ok("flat([1, [\"hello\"]])", &ctx);
    // normalize_union sorts: Number < String
    assert_eq!(
        ty,
        Ty::List(Box::new(Ty::Union(vec![Ty::Number, Ty::String])))
    );
}

#[test]
fn flat_unknown_and_unknown_list_preserve_unknown() {
    let ctx = builtins_ctx();
    for source in ["flat([x])", "flat(empty())"] {
        assert_eq!(infer_ok(source, &ctx), Ty::List(Box::new(Ty::Unknown)));
    }
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

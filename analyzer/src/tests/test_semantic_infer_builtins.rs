use crate::semantic::{self, Context, Ty, builtins_functions};
use crate::{Span, analyze};

fn infer_ok(source: &str, ctx: &Context) -> Ty {
    let output = analyze(source).unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (ty, diags) = semantic::analyze_expr(&output.expr, ctx);
    assert!(diags.is_empty(), "unexpected semantic diagnostics: {:?}", diags);
    ty
}

fn assert_single_diag(source: &str, ctx: &Context, message: &str, span: Span) {
    let output = analyze(source).unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&output.expr, ctx);
    assert_eq!(diags.len(), 1, "unexpected diagnostics: {:?}", diags);
    assert_eq!(diags[0].message, message);
    assert_eq!(diags[0].span, span);
}

fn builtins_ctx() -> Context {
    Context {
        properties: vec![],
        functions: builtins_functions(),
    }
}

#[test]
fn semantic_if_inferrs_union_on_plain_generic_conflict() {
    let ctx = builtins_ctx();
    let ty = infer_ok("if(true, 1, \"x\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn semantic_if_inferrs_union_through_nested_if() {
    let ctx = builtins_ctx();
    let ty = infer_ok("if(true, if(true, 1, 2), \"x\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn semantic_ifs_inferrs_union_across_repeat_groups() {
    let ctx = builtins_ctx();
    let ty = infer_ok("ifs(true, 1, false, \"x\", 2)", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn semantic_ifs_variant_generic_skips_unknown_when_accumulating_union() {
    let ctx = builtins_ctx();
    let ty = infer_ok("ifs(true, 1, false, x, \"a\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn diagnostics_if_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "if(true, 1)",
        &ctx,
        "if() expects exactly 3 arguments",
        Span { start: 0, end: 11 },
    );
}

#[test]
fn diagnostics_ifs_missing_default_is_arity_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "ifs(true, 1)",
        &ctx,
        "ifs() expects at least 3 arguments",
        Span { start: 0, end: 12 },
    );
}

#[test]
fn diagnostics_ifs_repeat_shape_error() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "ifs(true, 1, false, 2)",
        &ctx,
        "ifs() has an invalid argument shape",
        Span { start: 0, end: 22 },
    );
}


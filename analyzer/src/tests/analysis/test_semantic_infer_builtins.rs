use crate::semantic::{self, Context, Ty, builtins_functions};
use crate::{Span, analyze_syntax};

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

fn assert_single_diag(source: &str, ctx: &Context, message: &str, span: Span) {
    let output = analyze_syntax(source);
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
fn semantic_if_variant_generic_inferrs_union_on_conflict() {
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
fn semantic_if_variant_generic_propagates_unknown() {
    let ctx = builtins_ctx();
    let ty = infer_ok("if(true, x, 1)", &ctx);
    assert_eq!(ty, Ty::Unknown);
}

#[test]
fn semantic_ternary_inferrs_union_on_conflict() {
    let ctx = builtins_ctx();
    let ty = infer_ok("4 == 4 ? true : \"false\"", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Boolean, Ty::String]));
}

#[test]
fn semantic_ifs_variant_generic_inferrs_union_across_repeat_groups() {
    let ctx = builtins_ctx();
    let ty = infer_ok("ifs(true, 1, false, 2, \"a\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn semantic_ifs_variant_generic_propagates_unknown() {
    let ctx = builtins_ctx();
    let ty = infer_ok("ifs(true, x, false, 1, 2)", &ctx);
    assert_eq!(ty, Ty::Unknown);
}

#[test]
fn semantic_member_call_ifs_is_equivalent_to_prefix_call() {
    let ctx = builtins_ctx();
    let a = infer_ok("(true).ifs(1, 2)", &ctx);
    let b = infer_ok("ifs(true, 1, 2)", &ctx);
    assert_eq!(a, b);
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

#[test]
fn diagnostics_ifs_shape_error_short_circuits_type_mismatches() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "ifs(true, 1, \"x\", 2)",
        &ctx,
        "ifs() has an invalid argument shape",
        Span { start: 0, end: 20 },
    );
}

use crate::semantic::{self, Context, Property, Ty};
use crate::{Span, analyze};

fn run_semantic(source: &str, ctx: Context) -> Vec<crate::Diagnostic> {
    let output = analyze(source).unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&output.expr, &ctx);
    diags
}

fn assert_single_diag(diags: Vec<crate::Diagnostic>, message: &str, span: Span) {
    assert_eq!(diags.len(), 1, "unexpected diagnostics: {:?}", diags);
    assert_eq!(diags[0].message, message);
    assert_eq!(diags[0].span, span);
}

#[test]
fn test_prop_ok() {
    let ctx = Context {
        properties: vec![Property {
            name: "Title".into(),
            ty: Ty::String,
        }],
    };
    let diags = run_semantic("prop(\"Title\")", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_prop_missing() {
    let ctx = Context::default();
    let diags = run_semantic("prop(\"Missing\")", ctx);
    assert_single_diag(
        diags,
        "Unknown property: Missing",
        Span { start: 5, end: 14 },
    );
}

#[test]
fn test_prop_arg_not_string_literal() {
    let ctx = Context::default();
    let diags = run_semantic("prop(123)", ctx);
    assert_single_diag(
        diags,
        "prop() expects a string literal argument",
        Span { start: 5, end: 8 },
    );
}

#[test]
fn test_prop_arity() {
    let ctx = Context::default();
    let diags = run_semantic("prop(\"Title\",\"x\")", ctx);
    assert_single_diag(
        diags,
        "prop() expects exactly 1 argument",
        Span { start: 0, end: 17 },
    );
}

#[test]
fn test_if_ok() {
    let ctx = Context {
        properties: vec![Property {
            name: "Done".into(),
            ty: Ty::Boolean,
        }],
    };
    let diags = run_semantic("if(prop(\"Done\"), 1, 2)", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_if_cond_not_bool() {
    let ctx = Context::default();
    let diags = run_semantic("if(1, 1, 2)", ctx);
    assert_single_diag(
        diags,
        "if() condition must be boolean",
        Span { start: 3, end: 4 },
    );
}

#[test]
fn test_sum_ok() {
    let ctx = Context::default();
    let diags = run_semantic("sum(1,2,3)", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_sum_arg_not_number() {
    let ctx = Context::default();
    let diags = run_semantic("sum(1,\"x\")", ctx);
    assert_single_diag(
        diags,
        "sum() expects number arguments",
        Span { start: 6, end: 9 },
    );
}

#[test]
fn test_sum_arity() {
    let ctx = Context::default();
    let diags = run_semantic("sum()", ctx);
    assert_single_diag(
        diags,
        "sum() expects at least 1 argument",
        Span { start: 0, end: 5 },
    );
}

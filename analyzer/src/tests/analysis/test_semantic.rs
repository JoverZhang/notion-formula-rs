use crate::semantic::{
    self, Context, FunctionCategory, FunctionSig, GenericId, ParamShape, ParamSig, Property, Ty,
};
use crate::{Span, analyze};

fn p(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.into(),
        ty,
        optional: false,
    }
}

fn opt(name: &str, ty: Ty) -> ParamSig {
    ParamSig {
        name: name.into(),
        ty,
        optional: true,
    }
}

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

fn ctx_with_builtins() -> Context {
    Context {
        properties: vec![],
        functions: semantic::builtins_functions(),
    }
}

#[test]
fn test_prop_ok() {
    let mut ctx = ctx_with_builtins();
    ctx.properties.push(Property {
        name: "Title".into(),
        ty: Ty::String,
        disabled_reason: None,
    });
    let diags = run_semantic("prop(\"Title\")", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_prop_missing() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("prop(\"Missing\")", ctx);
    assert_single_diag(
        diags,
        "Unknown property: Missing",
        Span { start: 5, end: 14 },
    );
}

#[test]
fn test_prop_arg_not_string_literal() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("prop(123)", ctx);
    assert_single_diag(
        diags,
        "prop() expects a string literal argument",
        Span { start: 5, end: 8 },
    );
}

#[test]
fn test_prop_arity() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("prop(\"Title\",\"x\")", ctx);
    assert_single_diag(
        diags,
        "prop() expects exactly 1 argument",
        Span { start: 0, end: 17 },
    );
}

#[test]
fn test_if_ok() {
    let mut ctx = ctx_with_builtins();
    ctx.properties.push(Property {
        name: "Done".into(),
        ty: Ty::Boolean,
        disabled_reason: None,
    });
    let diags = run_semantic("if(prop(\"Done\"), 1, 2)", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_if_cond_not_bool() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("if(1, 1, 2)", ctx);
    assert_single_diag(
        diags,
        "argument type mismatch: expected Boolean, got Number",
        Span { start: 3, end: 4 },
    );
}

#[test]
fn test_sum_ok() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("sum(1)", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_sum_variadic_ok() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("sum(1,2,3)", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn test_sum_arg_not_number() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("sum(1,\"x\",2)", ctx);
    assert_single_diag(
        diags,
        "sum() expects number arguments",
        Span { start: 6, end: 9 },
    );
}

#[test]
fn test_sum_arity() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("sum()", ctx);
    assert_single_diag(
        diags,
        "sum() expects at least 1 argument",
        Span { start: 0, end: 5 },
    );
}

#[test]
fn test_unknown_function_does_not_crash() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("noSuchFn(1)", ctx);
    assert_single_diag(
        diags,
        "unknown function: noSuchFn",
        Span { start: 0, end: 11 },
    );
}

#[test]
fn test_sum_type_mismatch_emits_error() {
    let ctx = ctx_with_builtins();
    let diags = run_semantic("sum(\"a\")", ctx);
    assert_single_diag(
        diags,
        "sum() expects number arguments",
        Span { start: 4, end: 7 },
    );
}

#[test]
fn test_sum_accepts_number_list_property() {
    let mut ctx = ctx_with_builtins();
    ctx.properties.push(Property {
        name: "Nums".into(),
        ty: Ty::List(Box::new(Ty::Number)),
        disabled_reason: None,
    });
    let diags = run_semantic("sum(prop(\"Nums\"))", ctx);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn validate_call_does_not_wildcard_inferred_actual_generic() {
    let output = analyze("foo(prop(\"x\"))").unwrap();
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );

    let args = match &output.expr.kind {
        crate::ast::ExprKind::Call { args, .. } => args,
        other => panic!("expected Call expr, got {:?}", other),
    };
    let arg_span = args[0].span;

    let sig = FunctionSig {
        name: "foo".into(),
        params: ParamShape::new(vec![p("x", Ty::Number)], vec![], vec![]),
        ret: Ty::Number,
        category: FunctionCategory::General,
        detail: "foo(x)".into(),
        generics: vec![],
    };

    let ctx = Context {
        properties: vec![Property {
            name: "x".into(),
            ty: Ty::Generic(GenericId(0)),
            disabled_reason: None,
        }],
        functions: vec![sig],
    };

    let (_, diags) = semantic::analyze_expr(&output.expr, &ctx);

    assert_eq!(diags.len(), 1, "unexpected diagnostics: {:?}", diags);
    assert!(diags[0].message.contains("argument type mismatch"));
    assert!(diags[0].message.contains("expected"));
    assert!(diags[0].message.contains("Number"));
    assert!(diags[0].message.contains("Generic"));
    assert_eq!(diags[0].span, arg_span);
}

#[test]
fn required_min_args_repeat_group_counts_all_non_optional_in_head_and_tail() {
    let sig = FunctionSig {
        name: "rg".into(),
        params: ParamShape::new(
            vec![opt("h_opt", Ty::Number), p("h_req", Ty::Number)],
            vec![p("r", Ty::Number)],
            vec![p("t_req", Ty::Number)],
        ),
        ret: Ty::Number,
        category: FunctionCategory::General,
        detail: "rg(...)".into(),
        generics: vec![],
    };

    assert_eq!(sig.required_min_args(), 3);
}

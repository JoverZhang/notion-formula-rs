use crate::ast::ExprKind;
use crate::semantic::{self, Context, LambdaParam, Ty, builtins_functions};
use crate::{Span, analyze_syntax};

fn builtins_ctx() -> Context {
    Context {
        properties: vec![],
        functions: builtins_functions(),
    }
}

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

fn infer_ok_with_ast(source: &str, ctx: &Context) -> (Ty, crate::ast::Expr) {
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
    (ty, output.expr)
}

fn assert_single_diag(source: &str, ctx: &Context, message: &str, span: Span) {
    let mut output = analyze_syntax(source);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&mut output.expr, ctx);
    assert_eq!(diags.len(), 1, "unexpected diagnostics: {:?}", diags);
    assert_eq!(diags[0].message, message);
    assert_eq!(diags[0].span, span);
}

/// Extract the call args from the root expression (must be ExprKind::Call).
fn call_args(expr: &crate::ast::Expr) -> &[crate::ast::Expr] {
    match &expr.kind {
        ExprKind::Call { args, .. } => args,
        other => panic!("expected Call, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// if/ifs: ImplicitLambda wrapping (nullary thunks)
// ---------------------------------------------------------------------------

#[test]
fn if_wraps_then_else_in_implicit_lambda() {
    let ctx = builtins_ctx();
    let (_, expr) = infer_ok_with_ast("if(true, 1, \"x\")", &ctx);
    let args = call_args(&expr);

    // condition is NOT wrapped
    assert!(
        !matches!(&args[0].kind, ExprKind::ImplicitLambda { .. }),
        "condition should not be wrapped"
    );

    // then branch IS wrapped in a nullary lambda
    match &args[1].kind {
        ExprKind::ImplicitLambda { params, body } => {
            assert!(params.is_empty(), "then thunk should have no params");
            assert!(matches!(&body.kind, ExprKind::Lit(_)));
        }
        other => panic!("then arg should be ImplicitLambda, got {:?}", other),
    }

    // else branch IS wrapped in a nullary lambda
    match &args[2].kind {
        ExprKind::ImplicitLambda { params, body } => {
            assert!(params.is_empty(), "else thunk should have no params");
            assert!(matches!(&body.kind, ExprKind::Lit(_)));
        }
        other => panic!("else arg should be ImplicitLambda, got {:?}", other),
    }
}

#[test]
fn if_infers_union_through_implicit_lambda() {
    let ctx = builtins_ctx();
    let ty = infer_ok("if(true, 1, \"x\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn if_nested_infers_through_implicit_lambda() {
    let ctx = builtins_ctx();
    let ty = infer_ok("if(true, if(true, 1, 2), \"x\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn ifs_wraps_value_and_else_in_implicit_lambda() {
    let ctx = builtins_ctx();
    let (_, expr) = infer_ok_with_ast("ifs(true, 1, false, 2, \"default\")", &ctx);
    let args = call_args(&expr);
    // ifs(cond1, value1, cond2, value2, else)
    // args[0] = cond1, not wrapped
    assert!(!matches!(&args[0].kind, ExprKind::ImplicitLambda { .. }));
    // args[1] = value1, wrapped
    assert!(matches!(&args[1].kind, ExprKind::ImplicitLambda { params, .. } if params.is_empty()));
    // args[2] = cond2, not wrapped
    assert!(!matches!(&args[2].kind, ExprKind::ImplicitLambda { .. }));
    // args[3] = value2, wrapped
    assert!(matches!(&args[3].kind, ExprKind::ImplicitLambda { params, .. } if params.is_empty()));
    // args[4] = else, wrapped
    assert!(matches!(&args[4].kind, ExprKind::ImplicitLambda { params, .. } if params.is_empty()));
}

#[test]
fn ifs_infers_union_across_repeat_groups_with_lambdas() {
    let ctx = builtins_ctx();
    let ty = infer_ok("ifs(true, 1, false, 2, \"a\")", &ctx);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

// ---------------------------------------------------------------------------
// let: binder semantics
// ---------------------------------------------------------------------------

#[test]
fn let_basic_number_binding() {
    let ctx = builtins_ctx();
    let ty = infer_ok("let(x, 5, x + 1)", &ctx);
    assert_eq!(ty, Ty::Number);
}

#[test]
fn let_string_binding_with_length() {
    let ctx = builtins_ctx();
    let ty = infer_ok("let(x, \"hello\", length(x))", &ctx);
    assert_eq!(ty, Ty::Number);
}

#[test]
fn let_wraps_body_in_implicit_lambda_with_ident_param() {
    let ctx = builtins_ctx();
    let (_, expr) = infer_ok_with_ast("let(x, 5, x + 1)", &ctx);
    let args = call_args(&expr);

    // args[0] = ident (x) — NOT wrapped
    assert!(
        matches!(&args[0].kind, ExprKind::Ident(_)),
        "first arg should remain a bare ident"
    );

    // args[1] = value (5) — NOT wrapped
    assert!(
        !matches!(&args[1].kind, ExprKind::ImplicitLambda { .. }),
        "value arg should not be wrapped"
    );

    // args[2] = body — wrapped with param "x"
    match &args[2].kind {
        ExprKind::ImplicitLambda { params, .. } => {
            assert_eq!(params, &["x"], "lambda should bind 'x'");
        }
        other => panic!("body arg should be ImplicitLambda, got {:?}", other),
    }
}

#[test]
fn let_propagates_generic_type_through_binder() {
    let ctx = builtins_ctx();
    // The body returns a boolean — different from the bound value type (number).
    let ty = infer_ok("let(x, 5, x > 0)", &ctx);
    assert_eq!(ty, Ty::Boolean);
}

#[test]
fn let_ident_validation_rejects_non_ident() {
    let ctx = builtins_ctx();
    // `123` is not a bare identifier
    assert_single_diag(
        "let(123, 5, 6)",
        &ctx,
        "let() expects a variable name for `ident`",
        // span of `123` is positions 4..7
        Span { start: 4, end: 7 },
    );
}

#[test]
fn let_ident_validation_rejects_string_literal() {
    let ctx = builtins_ctx();
    assert_single_diag(
        "let(\"x\", 5, 6)",
        &ctx,
        "let() expects a variable name for `ident`",
        // span of `"x"` is positions 4..7
        Span { start: 4, end: 7 },
    );
}

// ---------------------------------------------------------------------------
// List lambda builtins: map, filter, find, findIndex, some, every, count
// ---------------------------------------------------------------------------

#[test]
fn map_infers_result_list_type() {
    let ctx = builtins_ctx();
    let ty = infer_ok("map([1, 2, 3], current + 1)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn map_wraps_mapper_in_implicit_lambda_with_current() {
    let ctx = builtins_ctx();
    let (_, expr) = infer_ok_with_ast("map([1, 2], current + 1)", &ctx);
    let args = call_args(&expr);

    // args[0] = list, not wrapped
    assert!(!matches!(&args[0].kind, ExprKind::ImplicitLambda { .. }));

    // args[1] = mapper, wrapped with "current"
    match &args[1].kind {
        ExprKind::ImplicitLambda { params, .. } => {
            assert_eq!(params, &["current"]);
        }
        other => panic!("mapper arg should be ImplicitLambda, got {:?}", other),
    }
}

#[test]
fn map_transforms_element_type() {
    let ctx = builtins_ctx();
    // current is Number, format returns String → result is String[]
    let ty = infer_ok("map([1, 2], format(current))", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::String)));
}

#[test]
fn filter_preserves_element_type() {
    let ctx = builtins_ctx();
    let ty = infer_ok("filter([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn find_returns_element_type() {
    let ctx = builtins_ctx();
    let ty = infer_ok("find([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::Number);
}

#[test]
fn find_index_returns_number() {
    let ctx = builtins_ctx();
    let ty = infer_ok("findIndex([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::Number);
}

#[test]
fn some_returns_boolean() {
    let ctx = builtins_ctx();
    let ty = infer_ok("some([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::Boolean);
}

#[test]
fn every_returns_boolean() {
    let ctx = builtins_ctx();
    let ty = infer_ok("every([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::Boolean);
}

#[test]
fn count_returns_number() {
    let ctx = builtins_ctx();
    let ty = infer_ok("count([1, 2, 3], current > 1)", &ctx);
    assert_eq!(ty, Ty::Number);
}

// ---------------------------------------------------------------------------
// Postfix / member-call desugaring with lambdas
// ---------------------------------------------------------------------------

#[test]
fn postfix_map_desugars_and_infers_correctly() {
    let ctx = builtins_ctx();
    let ty = infer_ok("[1, 2, 3].map(current + 1)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn postfix_filter_desugars_and_infers_correctly() {
    let ctx = builtins_ctx();
    let ty = infer_ok("[1, 2, 3].filter(current > 1)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn postfix_map_is_equivalent_to_prefix() {
    let ctx = builtins_ctx();
    let a = infer_ok("[1, 2].map(current + 1)", &ctx);
    let b = infer_ok("map([1, 2], current + 1)", &ctx);
    assert_eq!(a, b);
}

#[test]
fn let_prefix_call_infers_correctly() {
    let ctx = builtins_ctx();
    // Postfix `let` is not useful: the receiver fills the ident slot, which must be a bare
    // identifier — so we only test the prefix form here.
    let ty = infer_ok("let(x, 5, x + 1)", &ctx);
    assert_eq!(ty, Ty::Number);
}

// ---------------------------------------------------------------------------
// Nested lambdas
// ---------------------------------------------------------------------------

#[test]
fn nested_let_inside_map() {
    let ctx = builtins_ctx();
    // map([1,2], let(x, current, x + 1))
    // current: Number, x bound to current (Number), x + 1 → Number
    let ty = infer_ok("map([1, 2], let(x, current, x + 1))", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn nested_if_inside_map() {
    let ctx = builtins_ctx();
    // map([1,2], if(current > 1, current, 0))
    let ty = infer_ok("map([1, 2], if(current > 1, current, 0))", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn nested_map_inside_let() {
    let ctx = builtins_ctx();
    // let(xs, [1,2,3], map(xs, current + 1))
    let ty = infer_ok("let(xs, [1, 2, 3], map(xs, current + 1))", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
}

#[test]
fn deeply_nested_let_in_let() {
    let ctx = builtins_ctx();
    // let(a, 1, let(b, 2, a + b))
    let ty = infer_ok("let(a, 1, let(b, 2, a + b))", &ctx);
    assert_eq!(ty, Ty::Number);
}

#[test]
fn let_body_sees_outer_scope() {
    let ctx = builtins_ctx();
    // let(a, 1, let(b, a + 1, b + a))
    // a = 1 (Number), b = a + 1 (Number), b + a (Number)
    let ty = infer_ok("let(a, 1, let(b, a + 1, b + a))", &ctx);
    assert_eq!(ty, Ty::Number);
}

// ---------------------------------------------------------------------------
// Scope isolation
// ---------------------------------------------------------------------------

#[test]
fn current_is_unknown_outside_lambda() {
    let ctx = builtins_ctx();
    // `current` at the top level is not in any lambda scope and should resolve to Unknown.
    let ty = infer_ok("current", &ctx);
    assert_eq!(ty, Ty::Unknown);
}

#[test]
fn current_does_not_leak_from_map_into_let_body() {
    let ctx = builtins_ctx();
    // The map lambda binds `current`, but the let body should not see it.
    // `x` is bound to the map result (List<Number>), and the body just returns `x`.
    let (ty, expr) = infer_ok_with_ast("let(x, map([1, 2], current + 1), x)", &ctx);
    assert_eq!(ty, Ty::List(Box::new(Ty::Number)));
    // The let body lambda wraps `x`; verify it has param "x", not "current".
    let args = call_args(&expr);
    match &args[2].kind {
        ExprKind::ImplicitLambda { params, .. } => {
            assert_eq!(params, &["x"]);
            assert!(!params.contains(&"current".to_string()));
        }
        other => panic!("expected ImplicitLambda, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Validation: lambda body type errors
// ---------------------------------------------------------------------------

#[test]
fn filter_rejects_non_boolean_predicate() {
    let ctx = builtins_ctx();
    // filter expects predicate: (current: T) -> Boolean.
    // `current + 1` returns Number, not Boolean — should produce a type mismatch.
    let mut output = analyze_syntax("filter([1, 2], current + 1)");
    assert!(
        output.diagnostics.is_empty(),
        "unexpected parser diagnostics: {:?}",
        output.diagnostics
    );
    let (_, diags) = semantic::analyze_expr(&mut output.expr, &ctx);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly 1 diagnostic, got: {:?}",
        diags
    );
    assert!(
        diags[0].message.contains("argument type mismatch"),
        "expected type mismatch diagnostic, got: {}",
        diags[0].message
    );
}

// ---------------------------------------------------------------------------
// Ty::Fn and Ty::Ident display
// ---------------------------------------------------------------------------

#[test]
fn ty_fn_display_nullary() {
    let ty = Ty::Fn {
        params: vec![],
        ret: Box::new(Ty::Number),
    };
    assert_eq!(ty.to_string(), "() -> number");
}

#[test]
fn ty_fn_display_with_params() {
    let ty = Ty::Fn {
        params: vec![(LambdaParam::Current, Ty::Number)],
        ret: Box::new(Ty::Boolean),
    };
    assert_eq!(ty.to_string(), "(current: number) -> boolean");
}

#[test]
fn ty_ident_display() {
    let ty = Ty::Ident(Box::new(Ty::Number));
    assert_eq!(ty.to_string(), "ident<number>");
}

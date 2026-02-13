use crate::analyze_syntax;
use crate::ast::{Expr, ExprKind};
use crate::lexer::Span;

fn assert_span(expr: &Expr, start: u32, end: u32) {
    assert_eq!(expr.span, Span { start, end });
}

fn assert_contains(parent: &Expr, child: &Expr) {
    assert!(
        parent.span.start <= child.span.start && child.span.end <= parent.span.end,
        "expected parent span {:?} to contain child span {:?}",
        parent.span,
        child.span
    );
}

fn assert_tree_invariants(expr: &Expr) {
    assert!(
        expr.span.start <= expr.span.end,
        "invalid span {:?}",
        expr.span
    );
    match &expr.kind {
        ExprKind::Group { inner } => {
            assert_contains(expr, inner);
            assert_tree_invariants(inner);
        }
        ExprKind::List { items } => {
            for item in items {
                assert_contains(expr, item);
                assert_tree_invariants(item);
            }
        }
        ExprKind::Call { args, .. } => {
            for arg in args {
                assert_contains(expr, arg);
                assert_tree_invariants(arg);
            }
        }
        ExprKind::MemberCall { receiver, args, .. } => {
            assert_contains(expr, receiver);
            assert_tree_invariants(receiver);
            for arg in args {
                assert_contains(expr, arg);
                assert_tree_invariants(arg);
            }
        }
        ExprKind::Unary { expr: inner, .. } => {
            assert_contains(expr, inner);
            assert_tree_invariants(inner);
        }
        ExprKind::Binary { left, right, .. } => {
            assert_contains(expr, left);
            assert_contains(expr, right);
            assert_tree_invariants(left);
            assert_tree_invariants(right);
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            assert_contains(expr, cond);
            assert_contains(expr, then);
            assert_contains(expr, otherwise);
            assert_tree_invariants(cond);
            assert_tree_invariants(then);
            assert_tree_invariants(otherwise);
        }
        ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => {}
    }
}

#[test]
fn test_parser_spans_nested_groups() {
    let src = "( (a) )";
    let out = analyze_syntax(src);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);

    let expr = &out.expr;
    assert_tree_invariants(expr);

    let ExprKind::Group { inner } = &expr.kind else {
        panic!("expected outer group");
    };
    assert_span(expr, 0, 7);

    let ExprKind::Group { inner: inner2 } = &inner.kind else {
        panic!("expected inner group");
    };
    assert_span(inner, 2, 5);
    assert_span(inner2, 3, 4);
}

#[test]
fn test_parser_spans_call_with_comments() {
    let src = "f(a/*c*/,b)";
    let out = analyze_syntax(src);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);

    let expr = &out.expr;
    assert_tree_invariants(expr);

    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call");
    };
    assert_eq!(args.len(), 2);
    assert_span(expr, 0, 11);
    assert_span(&args[0], 2, 3);
    assert_span(&args[1], 9, 10);
}

#[test]
fn test_parser_spans_unary_call_with_comments() {
    let src = "-f(a/*c*/,b)";
    let out = analyze_syntax(src);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);

    let expr = &out.expr;
    assert_tree_invariants(expr);

    let ExprKind::Unary { expr: inner, .. } = &expr.kind else {
        panic!("expected unary");
    };
    assert_span(expr, 0, 12);

    let ExprKind::Call { args, .. } = &inner.kind else {
        panic!("expected call under unary");
    };
    assert_eq!(args.len(), 2);
    assert_span(inner, 1, 12);
    assert_span(&args[0], 3, 4);
    assert_span(&args[1], 10, 11);
}

#[test]
fn test_parser_spans_member_call_with_newline_and_comment() {
    let src = "a\n/*c*/.if(b,c)";
    let out = analyze_syntax(src);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);

    let expr = &out.expr;
    assert_tree_invariants(expr);

    let ExprKind::MemberCall { receiver, args, .. } = &expr.kind else {
        panic!("expected member-call");
    };
    assert_eq!(args.len(), 2);
    assert_span(expr, 0, 15);
    assert_span(receiver, 0, 1);
    assert_span(&args[0], 11, 12);
    assert_span(&args[1], 13, 14);
}

#[test]
fn test_parser_spans_error_recovery_in_call_arg_list() {
    let src = "f(,a)";
    let out = analyze_syntax(src);
    assert!(!out.diagnostics.is_empty());

    let expr = &out.expr;
    assert_tree_invariants(expr);

    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call");
    };
    assert_eq!(args.len(), 2);
    assert_span(expr, 0, 5);
    assert_span(&args[0], 2, 3);
    assert!(
        matches!(args[0].kind, ExprKind::Error),
        "expected recovery to produce an error expression for the missing argument before ','"
    );
    assert_span(&args[1], 3, 4);
}

#[test]
fn test_parser_spans_error_member_call_missing_method_ident() {
    let src = "a.(b)";
    let out = analyze_syntax(src);
    assert!(!out.diagnostics.is_empty());

    let expr = &out.expr;
    assert_tree_invariants(expr);
    assert_span(expr, 0, 2);
    assert!(matches!(expr.kind, ExprKind::Error));
}

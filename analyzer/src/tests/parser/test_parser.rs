use crate::ast::{BinOpKind, ExprKind};
use crate::lexer::LitKind;
use crate::{analyze, tests::common::trim_indent};

#[test]
fn test_pretty() {
    let parsed = analyze(&trim_indent(
        r#"
            if(
                prop("Title"),
                1,
                0
            )"#,
    ))
    .unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (_callee, args) = assert_call!(ast, "if", 3);
    let (_callee, args) = assert_call!(args[0], "prop", 1);
    assert_lit_str!(args[0], "Title");
}

#[test]
fn test_precedence() {
    let parsed = analyze("1 + 2 * 3").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (left, right) = assert_bin!(ast, BinOpKind::Plus);
    assert_lit_num!(left, 1);

    let (left, right) = assert_bin!(right, BinOpKind::Star);
    assert_lit_num!(left, 2);
    assert_lit_num!(right, 3);
}

#[test]
fn test_ternary_parse_shape() {
    let parsed = analyze("1 ? 2 : 3").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    assert_lit_num!(otherwise, 3);

    let parsed = analyze("1 ? 2 : 3 ? 4 : 5").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    let (cond, then, otherwise) = assert_ternary!(otherwise);
    assert_lit_num!(cond, 3);
    assert_lit_num!(then, 4);
    assert_lit_num!(otherwise, 5);
}

#[test]
fn test_caret_is_right_associative() {
    let parsed = analyze("2 ^ 3 ^ 2").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (left, right) = assert_bin!(ast, BinOpKind::Caret);
    assert_lit_num!(left, 2);
    let (left, right) = assert_bin!(right, BinOpKind::Caret);
    assert_lit_num!(left, 3);
    assert_lit_num!(right, 2);
}

#[test]
fn test_ternary_binds_lower_than_or_and_comparisons() {
    let parsed = analyze("1 > 2 || 3 > 4 ? \"x\" : \"y\"").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (cond, then, otherwise) = assert_ternary!(ast);
    let (left, right) = assert_bin!(cond, BinOpKind::OrOr);
    let (_l, _r) = assert_bin!(left, BinOpKind::Gt);
    let (_l, _r) = assert_bin!(right, BinOpKind::Gt);
    assert_lit_str!(then, "x");
    assert_lit_str!(otherwise, "y");
}

#[test]
fn test_ternary_missing_colon_recovers_to_colon() {
    let parsed = analyze("1 ? 2 foo : 3").unwrap();
    assert_eq!(parsed.diagnostics.len(), 1);
    assert!(parsed.diagnostics[0].message.starts_with("expected ':'"));

    let ast = parsed.expr;
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    assert_lit_num!(otherwise, 3);
}

#[test]
fn test_ternary_is_right_associative_with_idents() {
    let parsed = analyze("a ? b : c ? d : e").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (_cond, _then, otherwise) = assert_ternary!(ast);
    let (_c, _d, _e) = assert_ternary!(otherwise);
}

#[test]
fn test_postfix_binds_tighter_than_infix() {
    let parsed = analyze("a.if(b,c) + d").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (lhs, rhs) = assert_bin!(ast, BinOpKind::Plus);
    let ExprKind::MemberCall {
        receiver,
        method,
        args,
    } = &lhs.kind
    else {
        panic!("expected member-call on LHS, got {:?}", lhs.kind);
    };
    assert!(matches!(&receiver.kind, ExprKind::Ident(sym) if sym.text == "a"));
    assert_eq!(method.text, "if");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, ExprKind::Ident(sym) if sym.text == "b"));
    assert!(matches!(&args[1].kind, ExprKind::Ident(sym) if sym.text == "c"));

    assert!(matches!(&rhs.kind, ExprKind::Ident(sym) if sym.text == "d"));
}

#[test]
fn test_unary_applies_to_postfix_completed_expression() {
    let parsed = analyze("-f(1) * 2").unwrap();
    assert!(parsed.diagnostics.is_empty());
    let ast = parsed.expr;

    let (lhs, rhs) = assert_bin!(ast, BinOpKind::Star);
    let ExprKind::Unary { op, expr } = &lhs.kind else {
        panic!("expected unary on LHS, got {:?}", lhs.kind);
    };
    assert!(matches!(op, crate::ast::UnOp::Neg));
    let (_callee, args) = assert_call!(expr.as_ref(), "f", 1);
    assert_lit_num!(&args[0], 1);
    assert_lit_num!(rhs, 2);
}

#[test]
fn test_call_arg_list_missing_comma_recovers_as_two_args() {
    let parsed = analyze("f(1 2)").unwrap();
    assert_eq!(parsed.diagnostics.len(), 1, "diags: {:?}", parsed.diagnostics);

    let ast = parsed.expr;
    let (_callee, args) = assert_call!(ast, "f", 2);
    assert_lit_num!(&args[0], 1);
    assert_lit_num!(&args[1], 2);
}

#[test]
fn test_list_literal_missing_comma_recovers_as_two_items() {
    let parsed = analyze("[1 2]").unwrap();
    assert_eq!(parsed.diagnostics.len(), 1, "diags: {:?}", parsed.diagnostics);

    let ast = parsed.expr;
    let items = assert_list!(ast, 2);
    assert_lit_num!(&items[0], 1);
    assert_lit_num!(&items[1], 2);
}

#[test]
fn test_ternary_missing_then_expr_recovers() {
    let parsed = analyze("1 ? : 3").unwrap();
    assert!(!parsed.diagnostics.is_empty(), "diags: {:?}", parsed.diagnostics);

    let ast = parsed.expr;
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert!(matches!(then.kind, ExprKind::Error));
    assert_lit_num!(otherwise, 3);
}

#[test]
fn test_ternary_missing_else_expr_recovers_without_consuming_close_paren() {
    let parsed = analyze("(1 ? 2 : )").unwrap();
    assert!(!parsed.diagnostics.is_empty(), "diags: {:?}", parsed.diagnostics);

    let ast = parsed.expr;
    let ExprKind::Group { inner } = &ast.kind else {
        panic!("expected group, got {:?}", ast.kind);
    };

    let (cond, then, otherwise) = assert_ternary!(inner.as_ref());
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    assert!(matches!(otherwise.kind, ExprKind::Error));
}

#[test]
fn test_member_call_extra_dot_recovers() {
    let parsed = analyze("a..if(b,c)").unwrap();
    assert!(!parsed.diagnostics.is_empty(), "diags: {:?}", parsed.diagnostics);

    let ast = parsed.expr;
    let ExprKind::MemberCall {
        receiver,
        method,
        args,
    } = &ast.kind
    else {
        panic!("expected member-call, got {:?}", ast.kind);
    };

    assert!(matches!(&receiver.kind, ExprKind::Ident(sym) if sym.text == "a"));
    assert_eq!(method.text, "if");
    assert_eq!(args.len(), 2);
    assert!(matches!(&args[0].kind, ExprKind::Ident(sym) if sym.text == "b"));
    assert!(matches!(&args[1].kind, ExprKind::Ident(sym) if sym.text == "c"));
}

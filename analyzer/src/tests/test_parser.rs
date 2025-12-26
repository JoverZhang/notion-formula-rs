use crate::ast::{BinOpKind, ExprKind};
use crate::token::LitKind;
use crate::{analyze, tests::common::trim_indent};

#[test]
fn test_pretty() {
    let ast = analyze(&trim_indent(
        r#"
            if(
                prop("Title"),
                1,
                0
            )"#,
    ))
    .unwrap();

    let (_callee, args) = assert_call!(ast, "if", 3);
    let (_callee, args) = assert_call!(args[0], "prop", 1);
    assert_lit_str!(args[0], "Title");
}

#[test]
fn test_precedence() {
    let ast = analyze("1 + 2 * 3").unwrap();

    let (left, right) = assert_bin!(ast, BinOpKind::Plus);
    assert_lit_num!(left, 1);

    let (left, right) = assert_bin!(right, BinOpKind::Star);
    assert_lit_num!(left, 2);
    assert_lit_num!(right, 3);
}

#[test]
fn test_ternary_parse_shape() {
    let ast = analyze("1 ? 2 : 3").unwrap();
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    assert_lit_num!(otherwise, 3);

    let ast = analyze("1 ? 2 : 3 ? 4 : 5").unwrap();
    let (cond, then, otherwise) = assert_ternary!(ast);
    assert_lit_num!(cond, 1);
    assert_lit_num!(then, 2);
    let (cond, then, otherwise) = assert_ternary!(otherwise);
    assert_lit_num!(cond, 3);
    assert_lit_num!(then, 4);
    assert_lit_num!(otherwise, 5);
}

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
    assert_eq!(ast.pretty(), r#"if(prop("Title"), 1, 0)"#);
}

#[test]
fn test_precedence(){
    let ast = analyze(&trim_indent(r#"
        1 + 2 * 3
        "#,
    ))
    .unwrap();
    assert_eq!(ast.pretty(), "1 + 2 * 3");
}

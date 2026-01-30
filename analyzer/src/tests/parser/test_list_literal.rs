use crate::ast::ExprKind;
use crate::lexer::LitKind;
use crate::analyze;

#[test]
fn parser_list_empty() {
    let parsed = analyze("[]").unwrap();
    assert!(parsed.diagnostics.is_empty(), "diags: {:?}", parsed.diagnostics);
    let ast = parsed.expr;

    let items = assert_list!(ast, 0);
    assert!(items.is_empty());
}

#[test]
fn parser_list_three_items() {
    let parsed = analyze("[1,2,3]").unwrap();
    assert!(parsed.diagnostics.is_empty(), "diags: {:?}", parsed.diagnostics);
    let ast = parsed.expr;

    let items = assert_list!(ast, 3);
    assert_lit_num!(items[0], 1);
    assert_lit_num!(items[1], 2);
    assert_lit_num!(items[2], 3);
}


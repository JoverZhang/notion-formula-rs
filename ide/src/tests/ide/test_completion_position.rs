use analyzer::semantic::Ty;
use analyzer::Span;
use crate::tests::completion_dsl::{Builtin, Func, Item, Prop, ctx, t};
use crate::CompletionConfig;

#[test]
fn completion_when_expecting_separator_in_call_shows_after_atom_operators() {
    let c = ctx().build();

    t("if(true$0)")
        .ctx(c)
        .expect_sig_active(0)
        .expect_not_empty()
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_not_contains(&[Item::Func(Func::If), Item::Prop(Prop::Title)])
        .expect_not_contains(&[
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
        ])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_strictly_inside_ident_does_not_expect_separator() {
    let c = ctx().props_demo_basic().build();

    // Cursor is strictly inside the identifier token `true`.
    t("if(tr$0ue)")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_builtins(&[Builtin::True])
        .expect_contains_props(&[Prop::Title])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_at_ident_end_allows_extending_func_prefix_completion() {
    let c = ctx().props_demo_basic().build();

    t("if(su$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_funcs(&[Func::Sum])
        .expect_contains_builtins(&[Builtin::True])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_at_ident_end_allows_extending_prop_prefix_completion() {
    let c = ctx().props_demo_basic().build();

    t("if(Ti$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_props(&[Prop::Title])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_items_disabled_inside_prop_string_literal() {
    let c = ctx().props_demo_basic().build();

    t(r#"prop("$0")"#)
        .ctx(c)
        .expect_empty()
        .expect_replace_contains_cursor();
}

#[test]
fn completion_items_disabled_inside_plain_string_literal() {
    let c = ctx().props_demo_basic().build();

    t(r#""abc$0def""#)
        .ctx(c)
        .expect_empty()
        .expect_replace_contains_cursor();
}

#[test]
fn completion_items_disabled_but_signature_help_kept_inside_call_string_arg() {
    let c = ctx().build();

    t(r#"if("a$0", 1, 2)"#)
        .ctx(c)
        .expect_empty()
        .expect_sig_active(0)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_ident_end_prefix_allows_completions() {
    let c = ctx().props_demo_basic().build();

    // Keep the cursor at the end of the identifier token itself (not at the start of `)`).
    t("if(fa$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_builtins(&[Builtin::False])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_ident_end_before_close_paren_treats_ident_as_query() {
    let c = ctx().prop("Date", Ty::Date).build();

    let source = "if(d)";
    let cursor = source.find(')').unwrap();
    let out = crate::completion::complete(source, cursor, Some(&c), CompletionConfig::default());

    assert_eq!(
        out.replace,
        Span { start: 3, end: 4 },
        "expected replace span to cover the identifier prefix"
    );

    let labels: Vec<&str> = out.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"Date"),
        "expected property completion for Date\nactual labels: {labels:?}"
    );
    assert!(
        labels.contains(&"date()"),
        "expected function completion for date\nactual labels: {labels:?}"
    );
}

#[test]
fn completion_inside_call_arg_after_comma_suggests_expr_start_items() {
    let c = ctx().prop("Title", Ty::String).build();

    t("if(true, $0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_items(&[Item::Prop(Prop::Title), Item::Builtin(Builtin::True)])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_empty_before_close_paren_shows_items_and_signature_help() {
    let c = ctx().props_demo_basic().build();

    // Regression: cursor at `)` token start should still be treated as expr-start inside the call.
    t("sum($0)")
        .ctx(c)
        .expect_sig_active(0)
        .expect_not_empty()
        .expect_contains_props(&[Prop::Title])
        .expect_contains_builtins(&[Builtin::True])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_without_close_paren_shows_items_and_signature_help() {
    let c = ctx().props_demo_basic().build();

    t("sum($0")
        .ctx(c)
        .expect_sig_active(0)
        .expect_not_empty()
        .expect_contains_props(&[Prop::Title])
        .expect_contains_builtins(&[Builtin::True])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_empty_before_close_paren_second_arg_shows_items() {
    let c = ctx().props_demo_basic().build();

    // Regression: same as above, but for a later argument position in a call.
    t("if(true, $0)")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_props(&[Prop::Title])
        .expect_replace_contains_cursor();
}

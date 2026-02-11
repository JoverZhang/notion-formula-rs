use crate::completion::CompletionData;
use crate::semantic::Ty;
use crate::tests::completion_dsl::{Builtin, Func, Item, Prop, ctx, t};

#[test]
fn completion_at_document_start() {
    let c = ctx().props_demo_basic().build();

    t("$0")
        .ctx(c)
        .expect_contains_props(&[Prop::Title, Prop::Age, Prop::Flag])
        .expect_contains_builtins(&[Builtin::Not, Builtin::True, Builtin::False])
        .expect_contains_funcs(&[Func::If, Func::Sum])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_at_document_start_without_context_has_no_functions() {
    t("$0")
        .no_ctx()
        .expect_not_contains(&[Item::Func(Func::If), Item::Func(Func::Sum)])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_at_document_start_without_properties_has_no_prop_variables() {
    let c = ctx().build();

    t("$0")
        .ctx(c)
        .expect_not_contains(&[Item::Prop(Prop::Title)])
        .expect_contains_funcs(&[Func::If, Func::Sum])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_atom_shows_operators_and_postfix_methods() {
    let c = ctx().props_demo_basic().build();

    t("(1+1)$0")
        .ctx(c.clone())
        .expect_not_empty()
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_postfix(Func::If)
        .expect_not_contains(&[
            Item::Prop(Prop::Title),
            Item::Func(Func::If),
            Item::Func(Func::Sum),
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
        ])
        .expect_replace_contains_cursor();

    t("sum(1,2,3)$0")
        .ctx(c.clone())
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq)])
        .expect_postfix(Func::If);

    t("if(true,1,2)$0")
        .ctx(c.clone())
        .expect_contains_items(&[Item::Builtin(Builtin::Plus)])
        .expect_postfix(Func::If);

    t("true$0")
        .ctx(c)
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq)])
        .expect_postfix(Func::If);
}

#[test]
fn completion_after_dot_shows_postfix_methods() {
    let c = ctx().props_demo_basic().build();

    t("sum(1,2,3).$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_labels(&[".add()", ".round()"])
        .expect_not_postfix(Func::If)
        .expect_not_contains(&[
            Item::Prop(Prop::Title),
            Item::Func(Func::If),
            Item::Func(Func::Sum),
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
            Item::Builtin(Builtin::EqEq),
            Item::Builtin(Builtin::Plus),
        ])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_dot_offers_postfix_if_and_insert_text_is_if_parens() {
    let c = ctx().build();
    t("true.$0")
        .ctx(c)
        .expect_postfix(Func::If)
        .expect_item_insert_text(".if()", "if()")
        .expect_item_detail(".if()", "(condition).if(then, else)");
}

#[test]
fn completion_after_identifier_shows_after_atom_operators() {
    t("abc$0")
        .no_ctx()
        .expect_not_empty()
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_not_contains(&[Item::Builtin(Builtin::Not), Item::Builtin(Builtin::True)])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_complete_atom_shows_after_atom_operators() {
    t(r#"prop("Title")$0"#)
        .no_ctx()
        .expect_not_empty()
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_not_contains(&[Item::Builtin(Builtin::Not), Item::Builtin(Builtin::True)])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_disabled_property_marking() {
    let c = ctx()
        .prop("Title", Ty::String)
        .disabled_prop("Age", Ty::Number, "cycle")
        .build();

    t("$0")
        .ctx(c)
        .expect_prop(Prop::Title)
        .expect_item_disabled("Age", Some("cycle"))
        .expect_replace_contains_cursor();
}

#[test]
fn completion_apply_function_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().build();
    t("su$0").ctx(c).apply("sum()").expect_text("sum($0)");
}

#[test]
fn completion_apply_function_in_call_callee_position() {
    let c = ctx().build();
    t("$0").ctx(c).apply("if()").expect_text("if($0)");
}

#[test]
fn completion_function_insert_text_contains_lparen() {
    let c = ctx().build();

    t("$0")
        .ctx(c)
        .expect_item_data(
            "if()",
            CompletionData::Function {
                name: "if".to_string(),
            },
        )
        .expect_item_cursor_after_lparen("if()");
}

#[test]
fn completion_apply_property_inserts_full_prop_expression() {
    let c = ctx().prop("Title", Ty::String).build();

    t("$0")
        .ctx(c)
        .apply("Title")
        .expect_text(r#"prop("Title")$0"#);
}

#[test]
fn completion_apply_property_with_prefix() {
    let c = ctx().prop("Title", Ty::String).build();

    t("Ti$0")
        .ctx(c)
        .apply("Title")
        .expect_text(r#"prop("Title")$0"#);
}

#[test]
fn completion_apply_property_before_property() {
    let c = ctx().props_demo_basic().build();

    t(r#"$0prop("Title")"#)
        .ctx(c)
        .apply("Age")
        .expect_text(r#"prop("Age")$0prop("Title")"#);
}

#[test]
fn completion_apply_function_before_call() {
    let c = ctx().build();

    t("$0sum(1,2,3)")
        .ctx(c)
        .apply("if()")
        .expect_text("if($0)sum(1,2,3)");
}

#[test]
fn completion_apply_postfix_if_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().props_demo_basic().build();

    t("(1==1)$0")
        .ctx(c.clone())
        .apply(".if()")
        .expect_text("(1==1).if($0)");

    t("if(true,true,false)$0")
        .ctx(c.clone())
        .apply(".if()")
        .expect_text("if(true,true,false).if($0)");

    t("if(true,true,false).$0")
        .ctx(c)
        .apply(".if()")
        .expect_text("if(true,true,false).if($0)");
}

#[test]
fn completion_disabled_item_has_no_primary_edit() {
    let c = ctx().disabled_prop("Age", Ty::Number, "cycle").build();

    t("$0")
        .ctx(c)
        .expect_item_disabled("Age", Some("cycle"))
        .expect_item_no_primary_edit("Age");
}

#[test]
fn completion_property_data_and_kind() {
    let c = ctx().props_demo_basic().build();

    t("$0")
        .ctx(c)
        .expect_prop(Prop::Title)
        .expect_prop(Prop::Age)
        .expect_prop(Prop::Flag)
        .expect_func("if")
        .expect_func("sum")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_ignore_props_filters_property_items() {
    let c = ctx().props_demo_basic().build();

    t("$0")
        .ctx(c)
        .ignore_props()
        .expect_top_items(&[
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
        ])
        .expect_contains_funcs(&[Func::If, Func::Sum])
        .expect_replace_contains_cursor();
}

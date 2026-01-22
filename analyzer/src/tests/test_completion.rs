use crate::completion::CompletionData;
use crate::semantic::{ParamSig, Ty};
use crate::tests::completion_dsl::{Builtin, Func, Item, Prop, Symbol, ctx, t};

#[test]
fn completion_at_document_start() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t("$0")
        .ctx(c)
        .expect_top_items(&[
            Item::Prop(Prop::Title),
            Item::Prop(Prop::Age),
            Item::Prop(Prop::Flag),
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
            Item::Func(Func::If),
            Item::Func(Func::Sum),
        ])
        .expect_contains_builtins(&[Builtin::Not, Builtin::True, Builtin::False])
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
    let c = ctx().func_if().func_sum().build();

    t("$0")
        .ctx(c)
        .expect_contains_funcs(&[Func::If, Func::Sum])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_atom_shows_postfix_if_only() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t("(1+1)$0")
        .ctx(c.clone())
        .expect_not_empty()
        .expect_contains_items(&[Item::Symbol(Symbol::DotIf)])
        .expect_not_contains(&[
            Item::Prop(Prop::Title),
            Item::Func(Func::If),
            Item::Func(Func::Sum),
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
        ])
        .expect_replace_contains_cursor();

    t("sum(1,2)$0")
        .ctx(c.clone())
        .expect_contains_items(&[Item::Symbol(Symbol::DotIf)]);

    t("if(true,1,2)$0")
        .ctx(c.clone())
        .expect_contains_items(&[Item::Symbol(Symbol::DotIf)]);

    t("true$0")
        .ctx(c)
        .expect_contains_items(&[Item::Symbol(Symbol::DotIf)]);
}

#[test]
fn completion_inside_call_arg_strictly_inside_ident_does_not_expect_separator() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

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
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t("if(su$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_funcs(&[Func::Sum])
        .expect_contains_builtins(&[Builtin::True])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_at_ident_end_allows_extending_prop_prefix_completion() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

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
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t(r#""abc$0def""#)
        .ctx(c)
        .expect_empty()
        .expect_replace_contains_cursor();
}

#[test]
fn completion_items_disabled_but_signature_help_kept_inside_call_string_arg() {
    let c = ctx().func_if().build();

    t(r#"if("a$0", 1, 2)"#)
        .ctx(c)
        .expect_empty()
        .expect_sig_active(0)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_ident_end_prefix_allows_completions() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    // Keep the cursor at the end of the identifier token itself (not at the start of `)`).
    t("if(fa$0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_builtins(&[Builtin::False])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_after_comma_suggests_expr_start_items() {
    let c = ctx().prop("Title", Ty::String).func_if().build();

    t("if(true, $0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_items(&[Item::Prop(Prop::Title), Item::Builtin(Builtin::True)])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_empty_before_close_paren_shows_items_and_signature_help() {
    let c = ctx().props_demo_basic().func_sum().build();

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
    let c = ctx().props_demo_basic().func_sum().build();

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
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    // Regression: same as above, but for a later argument position in a call.
    t("if(true, $0)")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_props(&[Prop::Title])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_with_property_prefix() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    t("Ti$0")
        .ctx(c)
        .expect_top_labels(&["Title"])
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
fn completion_signature_help_active_param_first_arg() {
    let c = ctx().func_if().build();

    t("if($0")
        .ctx(c)
        .expect_sig_active(0)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_signature_help_active_param_second_arg() {
    let c = ctx().func_if().build();

    t("if(true, $0")
        .ctx(c)
        .expect_sig_active(1)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_signature_help_ignores_nested_commas() {
    let c = ctx().func_if().build();

    t("if(true, sum(1,2), $0")
        .ctx(c)
        .expect_sig_active(2)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_number_prefers_number_props() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .func_sum()
        .build();

    t("sum($0")
        .ctx(c)
        .expect_order("Age", "Title")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_handles_nontrivial_property_names() {
    let c = ctx()
        .prop("Title (new)", Ty::String)
        .prop("Age", Ty::Number)
        .func_sum()
        .build();

    t("sum($0")
        .ctx(c)
        .expect_item_data(
            "Title (new)",
            CompletionData::PropExpr {
                property_name: "Title (new)".to_string(),
            },
        )
        .expect_order("Age", "Title (new)")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_boolean_prefers_literals() {
    let c = ctx().props_demo_basic().func_if().build();

    t("if($0")
        .ctx(c)
        .expect_contains_items(&[
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
            Item::Prop(Prop::Flag),
            Item::Prop(Prop::Title),
        ])
        .expect_order_items(Item::Builtin(Builtin::True), Item::Prop(Prop::Title))
        .expect_order_items(Item::Builtin(Builtin::False), Item::Prop(Prop::Title))
        .expect_order_items(Item::Prop(Prop::Flag), Item::Prop(Prop::Title))
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_unknown_argument_does_not_filter_items() {
    let c = ctx()
        .props_demo_basic()
        .func("id")
        .param(ParamSig {
            name: None,
            ty: Ty::Unknown,
            optional: false,
        })
        .ret(Ty::Unknown)
        .finish()
        .build();

    t("id($0")
        .ctx(c)
        .expect_contains_props(&[Prop::Title, Prop::Age])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_apply_function_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().func_sum().build();
    t("su$0").ctx(c).apply("sum").expect_text("sum($0)");
}

#[test]
fn completion_apply_function_in_call_callee_position() {
    let c = ctx().func_if().build();
    t("$0").ctx(c).apply("if").expect_text("if($0)");
}

#[test]
fn completion_function_insert_text_contains_lparen() {
    let c = ctx().func_if().build();

    t("$0")
        .ctx(c)
        .expect_item_data(
            "if",
            CompletionData::Function {
                name: "if".to_string(),
            },
        )
        .expect_item_cursor_after_lparen("if");
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
    let c = ctx().func_if().func_sum().build();

    t("$0sum(1,2)")
        .ctx(c)
        .apply("if")
        .expect_text("if($0)sum(1,2)");
}

#[test]
fn completion_apply_postfix_if_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t("(1+1)$0")
        .ctx(c.clone())
        .apply(".if")
        .expect_text("(1+1).if($0)");

    t("sum(1,2)$0")
        .ctx(c)
        .apply(".if")
        .expect_text("sum(1,2).if($0)");
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
    let c = ctx().props_demo_basic().func_if().func_sum().build();

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
    let c = ctx().props_demo_basic().func_if().func_sum().build();

    t("$0")
        .ctx(c)
        .ignore_props()
        .expect_top_items(&[
            Item::Builtin(Builtin::Not),
            Item::Builtin(Builtin::True),
            Item::Builtin(Builtin::False),
            Item::Func(Func::If),
            Item::Func(Func::Sum),
        ])
        .expect_replace_contains_cursor();
}

// ----- New signature help tests -----

#[test]
fn signature_help_only_inside_call() {
    let c = ctx().func_if().build();

    // No signature help at document start
    t("$0").ctx(c.clone()).expect_no_signature_help();

    // No signature help before opening paren
    t("if$0").ctx(c.clone()).expect_no_signature_help();

    // Signature help appears after opening paren
    t("if($0").ctx(c).expect_sig_active(0);
}

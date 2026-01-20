use crate::completion::CompletionData;
use crate::semantic::{ParamSig, Ty};
use crate::tests::completion_dsl::{
    ctx, DemoFunc, DemoItem, DemoProp, DemoSymbol, t,
};

#[test]
fn completion_at_document_start() {
    let c = ctx()
        .props_demo_basic()
        .func_if()
        .func_sum()
        .build();

    t("$0")
        .ctx(c)
        .expect_top_items(&[
            DemoItem::Prop(DemoProp::Title),
            DemoItem::Prop(DemoProp::Age),
            DemoItem::Prop(DemoProp::Flag),
            DemoItem::Func(DemoFunc::If),
            DemoItem::Func(DemoFunc::Sum),
        ])
        .expect_contains_symbols(&[DemoSymbol::True, DemoSymbol::False, DemoSymbol::LParen])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_at_document_start_without_context_has_no_functions() {
    t("$0")
        .no_ctx()
        .expect_not_contains(["if", "sum"])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_at_document_start_without_properties_has_no_prop_variables() {
    let c = ctx().func_if().func_sum().build();

    t("$0")
        .ctx(c)
        .expect_contains_funcs(&[DemoFunc::If, DemoFunc::Sum])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_identifier_suppresses_expr_start() {
    t("abc$0")
        .no_ctx()
        .expect_not_contains(["if", "sum", "true", "false", "(", "Title"])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_complete_atom_suppresses_expr_start() {
    t(r#"prop("Title")$0"#)
        .no_ctx()
        .expect_not_contains(["if", "sum", "true", "false", "(", "Title"])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_when_expecting_separator_in_call_suppresses_expr_start() {
    let c = ctx().func_if().build();

    t("if(true$0")
        .ctx(c)
        .expect_empty()
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_call_arg_after_comma_suggests_expr_start_items() {
    let c = ctx().prop("Title", Ty::String).func_if().build();

    t("if(true, $0")
        .ctx(c)
        .expect_not_empty()
        .expect_contains_items(&[DemoItem::Prop(DemoProp::Title), DemoItem::Symbol(DemoSymbol::True)])
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
        .expect_prop(DemoProp::Title)
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
    let c = ctx()
        .props_demo_basic()
        .func_if()
        .build();

    t("if($0")
        .ctx(c)
        .expect_contains_items(&[
            DemoItem::Symbol(DemoSymbol::True),
            DemoItem::Symbol(DemoSymbol::False),
            DemoItem::Prop(DemoProp::Flag),
            DemoItem::Prop(DemoProp::Title),
        ])
        .expect_order_items(DemoItem::Symbol(DemoSymbol::True), DemoItem::Prop(DemoProp::Title))
        .expect_order_items(DemoItem::Symbol(DemoSymbol::False), DemoItem::Prop(DemoProp::Title))
        .expect_order_items(DemoItem::Prop(DemoProp::Flag), DemoItem::Prop(DemoProp::Title))
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
        .expect_contains_props(&[DemoProp::Title, DemoProp::Age])
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
        .expect_prop(DemoProp::Title)
        .expect_prop(DemoProp::Age)
        .expect_prop(DemoProp::Flag)
        .expect_func("if")
        .expect_func("sum")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_ignore_props_filters_property_items() {
    let c = ctx()
        .props_demo_basic()
        .func_if()
        .func_sum()
        .build();

    t("$0")
        .ctx(c)
        .ignore_props()
        .expect_top_items(&[
            DemoItem::Func(DemoFunc::If),
            DemoItem::Func(DemoFunc::Sum),
            DemoItem::Symbol(DemoSymbol::True),
            DemoItem::Symbol(DemoSymbol::False),
            DemoItem::Symbol(DemoSymbol::LParen),
        ])
        .expect_replace_contains_cursor();
}

// ----- New signature help tests -----

#[test]
fn signature_help_only_inside_call() {
    let c = ctx().func_if().build();

    // No signature help at document start
    t("$0")
        .ctx(c.clone())
        .expect_no_signature_help();

    // No signature help before opening paren
    t("if$0")
        .ctx(c.clone())
        .expect_no_signature_help();

    // Signature help appears after opening paren
    t("if($0")
        .ctx(c)
        .expect_sig_active(0);
}

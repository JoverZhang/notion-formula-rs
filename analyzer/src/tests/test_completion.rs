use crate::completion::CompletionData;
use crate::semantic::{ParamSig, Ty};
use crate::token::Span;

use crate::tests::completion_dsl::{ctx, prop_label, t};

#[test]
fn completion_at_document_start() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .func_if()
        .func_sum()
        .build();

    t("$0")
        .ctx(c)
        .expect_prefix(&[r#"prop("Title")"#, r#"prop("Age")"#])
        .expect_contains(["if", "sum"])
        .expect_item_data(
            r#"prop("Age")"#,
            CompletionData::PropExpr {
                property_name: "Age".to_string(),
            },
        )
        .expect_item_data(
            "if",
            CompletionData::Function {
                name: "if".to_string(),
            },
        )
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
        .expect_no_label_prefix("prop(\"")
        .expect_contains(["if", "sum"])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_identifier_suppresses_expr_start() {
    t("abc$0")
        .no_ctx()
        .expect_no_label_prefix("prop(\"")
        .expect_not_contains(["if", "sum", "true", "false", "("])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_prop_ident_with_context() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    t("prop$0")
        .ctx(c)
        .expect_prefix(&[r#"prop("Title")"#, r#"prop("Age")"#])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_prop_ident_without_context() {
    t("prop$0")
        .no_ctx()
        .expect_labels(&["("])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_prop_lparen_with_context() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    t("prop($0")
        .ctx(c)
        .expect_labels(&[r#"prop("Title")"#, r#"prop("Age")"#])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_prop_lparen_without_context() {
    t("prop($0")
        .no_ctx()
        .expect_empty()
        .expect_replace_contains_cursor();
}

#[test]
fn completion_inside_prop_string_with_context() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    let text = r#"prop("")"#;
    let quote_start = text.find('"').expect("expected opening quote");
    let quote_end = text[quote_start + 1..]
        .find('"')
        .map(|idx| idx + quote_start + 1)
        .expect("expected closing quote");
    let expected_replace = Span {
        start: (quote_start + 1) as u32,
        end: quote_end as u32,
    };

    t(r#"prop("$0")"#)
        .ctx(c)
        .expect_labels(&["Title", "Age"])
        .expect_item_data(
            "Age",
            CompletionData::PropertyName {
                name: "Age".to_string(),
            },
        )
        .expect_replace(expected_replace)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_after_complete_atom_suppresses_expr_start() {
    t(r#"prop("Title")$0"#)
        .no_ctx()
        .expect_no_label_prefix("prop(\"")
        .expect_not_contains(["if", "sum", "true", "false", "("])
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
        .expect_contains([r#"prop("Title")"#, "true"])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_prefix_prop_identifier_with_context() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    t("pro$0")
        .ctx(c)
        .expect_prefix(&[r#"prop("Title")"#, r#"prop("Age")"#])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_disabled_property_marking() {
    let c = ctx()
        .prop("Title", Ty::String)
        .disabled_prop("Age", Ty::Number, "cycle")
        .build();

    let mut at_start = t("$0").ctx(c.clone());
    assert!(!at_start.item(r#"prop("Title")"#).is_disabled);
    at_start
        .expect_item_disabled(r#"prop("Age")"#, Some("cycle"))
        .expect_replace_contains_cursor();

    let mut in_prop_string = t(r#"prop("$0")"#).ctx(c);
    assert!(!in_prop_string.item("Title").is_disabled);
    in_prop_string
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
        .expect_order(r#"prop("Age")"#, r#"prop("Title")"#)
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
            r#"prop("Title (new)")"#,
            CompletionData::PropExpr {
                property_name: "Title (new)".to_string(),
            },
        )
        .expect_order(r#"prop("Age")"#, r#"prop("Title (new)")"#)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_boolean_prefers_literals() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Flag", Ty::Boolean)
        .prop("Age", Ty::Number)
        .func_if()
        .build();

    t("if($0")
        .ctx(c)
        .expect_contains(["true", "false", r#"prop("Flag")"#, r#"prop("Title")"#])
        .expect_order("true", r#"prop("Title")"#)
        .expect_order("false", r#"prop("Title")"#)
        .expect_order(r#"prop("Flag")"#, r#"prop("Title")"#)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_unknown_argument_does_not_filter_items() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
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
        .expect_contains([r#"prop("Title")"#, r#"prop("Age")"#])
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
fn completion_apply_prop_expr_does_not_add_parens() {
    let c = ctx().prop("Title", Ty::String).build();
    let expected = prop_label("Title");

    t("pr$0")
        .ctx(c)
        .apply(&expected)
        .expect_text(&format!("{expected}$0"));
}

#[test]
fn completion_apply_property_name_inside_prop_string() {
    let c = ctx().prop("Title", Ty::Number).build();

    t(r#"prop("$0")"#)
        .ctx(c)
        .apply("Title")
        .expect_text(r#"prop("Title$0")"#);
}

#[test]
fn completion_disabled_item_has_no_primary_edit() {
    let c = ctx().disabled_prop("Age", Ty::Number, "cycle").build();

    t("$0")
        .ctx(c)
        .expect_item_disabled(r#"prop("Age")"#, Some("cycle"))
        .expect_item_no_primary_edit(r#"prop("Age")"#);
}

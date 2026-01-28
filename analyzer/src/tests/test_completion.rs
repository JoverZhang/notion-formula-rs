use crate::completion::{CompletionData, CompletionKind, complete_with_context};
use crate::semantic::Ty;
use crate::semantic::{Context, builtins_functions};
use crate::tests::completion_dsl::{Builtin, Func, Item, Prop, ctx, t};
use std::collections::HashSet;

fn fixture_source_and_cursor(fixture: &str) -> (String, usize) {
    let cursor = fixture.find("$0").expect("fixture must contain $0 marker");
    assert_eq!(
        fixture.matches("$0").count(),
        1,
        "fixture must contain exactly one $0 marker"
    );
    (fixture.replace("$0", ""), cursor)
}

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
        .expect_contains_items(&[
            Item::Builtin(Builtin::EqEq),
            Item::Builtin(Builtin::Plus),
        ])
        .expect_postfix_if()
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
        .expect_postfix_if();

    t("if(true,1,2)$0")
        .ctx(c.clone())
        .expect_contains_items(&[Item::Builtin(Builtin::Plus)])
        .expect_postfix_if();

    t("true$0")
        .ctx(c)
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq)])
        .expect_postfix_if();
}

#[test]
fn completion_after_atom_postfix_if_requires_if_in_context() {
    let c = ctx().props_demo_basic().without_funcs(&["if"]).build();

    t("(1+1)$0")
        .ctx(c)
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_not_postfix_if();
}

#[test]
fn completion_after_dot_shows_postfix_methods() {
    let c = ctx().props_demo_basic().build();

    t("sum(1,2,3).$0")
        .ctx(c)
        .expect_not_empty()
        .expect_item_data(
            ".if",
            CompletionData::PostfixMethod {
                name: "if".to_string(),
            },
        )
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
    let (source, cursor) = fixture_source_and_cursor("true.$0");
    let out = complete_with_context(&source, cursor, Some(&c));

    let item = out
        .items
        .iter()
        .find(|i| i.label == ".if")
        .expect("missing completion item .if");
    assert_eq!(item.insert_text, "if()");
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
fn completion_inside_call_arg_empty_does_not_apply_fuzzy_ranking() {
    let c = ctx().props_demo_basic().build();

    let out_start = crate::completion::complete_with_context("", 0, Some(&c));

    let input = "empty($0)";
    let cursor = input.find("$0").expect("missing cursor marker");
    let source = input.replace("$0", "");
    let out_call = crate::completion::complete_with_context(&source, cursor, Some(&c));

    assert!(out_call.preferred_indices.is_empty());

    let start_labels: Vec<(crate::completion::CompletionKind, String)> = out_start
        .items
        .iter()
        .map(|i| (i.kind, i.label.clone()))
        .collect();
    let call_labels: Vec<(crate::completion::CompletionKind, String)> = out_call
        .items
        .iter()
        .map(|i| (i.kind, i.label.clone()))
        .collect();
    assert_eq!(call_labels, start_labels);
}

#[test]
fn completion_fuzzy_ranking_orders_matches_and_computes_preferred_indices() {
    let c = ctx().build();

    let input = "rep$0";
    let cursor = input.find("$0").expect("missing cursor marker");
    let source = input.replace("$0", "");
    let out = crate::completion::complete_with_context_config(
        &source,
        cursor,
        Some(&c),
        crate::completion::CompletionConfig { preferred_limit: 3 },
    );

    let idx_to_number = out
        .items
        .iter()
        .position(|i| i.label == "toNumber")
        .unwrap();
    let idx_repeat = out.items.iter().position(|i| i.label == "repeat").unwrap();
    let idx_replace = out.items.iter().position(|i| i.label == "replace").unwrap();
    let idx_replace_all = out
        .items
        .iter()
        .position(|i| i.label == "replaceAll")
        .unwrap();

    assert!(idx_repeat < idx_to_number);
    assert!(idx_replace < idx_to_number);
    assert!(idx_replace_all < idx_to_number);

    assert_eq!(out.preferred_indices, vec![0, 1, 2]);
    assert_eq!(out.items[out.preferred_indices[0]].label, "repeat");
    assert_eq!(out.items[out.preferred_indices[1]].label, "replace");
    assert_eq!(out.items[out.preferred_indices[2]].label, "replaceAll");
}

#[test]
fn completion_preferred_limit_zero_disables_preferred_indices() {
    let c = ctx().build();

    let input = "rep$0";
    let cursor = input.find("$0").expect("missing cursor marker");
    let source = input.replace("$0", "");
    let out = crate::completion::complete_with_context_config(
        &source,
        cursor,
        Some(&c),
        crate::completion::CompletionConfig { preferred_limit: 0 },
    );

    assert!(out.preferred_indices.is_empty());
}

#[test]
fn completion_ranking_contains_beats_fuzzy() {
    let c = ctx()
        .only_funcs(&["mean", "median", "toNumber", "name", "some"])
        .build();

    let input = "me$0";
    let cursor = input.find("$0").expect("missing cursor marker");
    let source = input.replace("$0", "");
    let out = crate::completion::complete_with_context(&source, cursor, Some(&c));

    let idx_mean = out.items.iter().position(|i| i.label == "mean").unwrap();
    let idx_median = out.items.iter().position(|i| i.label == "median").unwrap();
    let idx_name = out.items.iter().position(|i| i.label == "name").unwrap();
    let idx_some = out.items.iter().position(|i| i.label == "some").unwrap();
    let idx_to_number = out
        .items
        .iter()
        .position(|i| i.label == "toNumber")
        .unwrap();

    let max_contains = *[idx_mean, idx_median, idx_name, idx_some]
        .iter()
        .max()
        .unwrap();
    assert!(max_contains < idx_to_number);
}

#[test]
fn completion_ranking_exact_beats_contains() {
    let mut replace = None;
    let mut replace_all = None;
    for f in builtins_functions() {
        match f.name.as_str() {
            "replace" => replace = Some(f),
            "replaceAll" => replace_all = Some(f),
            _ => {}
        }
    }
    let c = Context {
        properties: Vec::new(),
        functions: vec![replace_all.unwrap(), replace.unwrap()],
    };

    let input = "replace$0";
    let cursor = input.find("$0").expect("missing cursor marker");
    let source = input.replace("$0", "");
    let out = crate::completion::complete_with_context(&source, cursor, Some(&c));

    let idx_replace = out.items.iter().position(|i| i.label == "replace").unwrap();
    let idx_replace_all = out
        .items
        .iter()
        .position(|i| i.label == "replaceAll")
        .unwrap();

    assert!(idx_replace < idx_replace_all);
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
    let c = ctx().build();

    t("if($0")
        .ctx(c)
        .expect_sig_active(0)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_signature_help_active_param_second_arg() {
    let c = ctx().build();

    t("if(true, $0")
        .ctx(c)
        .expect_sig_active(1)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_signature_help_ignores_nested_commas() {
    let c = ctx().build();

    t("if(true, sum(1,2,3), $0")
        .ctx(c)
        .expect_sig_active(2)
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_number_prefers_number_props() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .build();

    t("sum($0")
        .ctx(c)
        .expect_order("Age", "Title")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_sum_union_accepts_number_list_props() {
    let c = ctx()
        .prop("Title", Ty::String)
        .prop("Age", Ty::Number)
        .prop("Nums", Ty::List(Box::new(Ty::Number)))
        .build();

    t("sum($0")
        .ctx(c)
        .expect_order("Age", "Title")
        .expect_order("Nums", "Title")
        .expect_replace_contains_cursor();
}

#[test]
fn completion_type_ranking_handles_nontrivial_property_names() {
    let c = ctx()
        .prop("Title (new)", Ty::String)
        .prop("Age", Ty::Number)
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
    let c = ctx().props_demo_basic().build();

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
    let c = ctx().props_demo_basic().build();

    t("id($0")
        .ctx(c)
        .expect_contains_props(&[Prop::Title, Prop::Age])
        .expect_replace_contains_cursor();
}

#[test]
fn completion_apply_function_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().build();
    t("su$0").ctx(c).apply("sum").expect_text("sum($0)");
}

#[test]
fn completion_apply_function_in_call_callee_position() {
    let c = ctx().build();
    t("$0").ctx(c).apply("if").expect_text("if($0)");
}

#[test]
fn completion_function_insert_text_contains_lparen() {
    let c = ctx().build();

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
    let c = ctx().build();

    t("$0sum(1,2,3)")
        .ctx(c)
        .apply("if")
        .expect_text("if($0)sum(1,2,3)");
}

#[test]
fn completion_apply_postfix_if_inserts_parens_and_moves_cursor_inside() {
    let c = ctx().props_demo_basic().build();

    t("(1+1)$0")
        .ctx(c.clone())
        .apply(".if")
        .expect_text("(1+1).if($0)");

    t("sum(1,2,3)$0")
        .ctx(c.clone())
        .apply(".if")
        .expect_text("sum(1,2,3).if($0)");

    t("sum(1,2,3).$0")
        .ctx(c)
        .apply(".if")
        .expect_text("sum(1,2,3).if($0)");
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

// ----- New signature help tests -----

#[test]
fn signature_help_only_inside_call() {
    let c = ctx().build();

    // No signature help at document start
    t("$0").ctx(c.clone()).expect_no_signature_help();

    // No signature help before opening paren
    t("if$0").ctx(c.clone()).expect_no_signature_help();

    // Signature help appears after opening paren
    t("if($0").ctx(c).expect_sig_active(0);
}

#[test]
fn signature_help_label_sum_union_variadic() {
    let c = ctx().build();
    t("sum($0")
        .ctx(c)
        .expect_sig_label("sum(values: number | number[], ...) -> number");
}

#[test]
fn signature_help_postfix_if_label_format() {
    let c = ctx().build();
    let (source, cursor) = fixture_source_and_cursor("true.if($0, 1)");
    let out = complete_with_context(&source, cursor, Some(&c));
    let sig = out
        .signature_help
        .as_ref()
        .expect("expected signature help");

    assert_eq!(sig.receiver.as_deref(), Some("condition: boolean"));
    assert_eq!(
        sig.params,
        vec!["then: unknown".to_string(), "else: unknown".to_string()]
    );
    assert_eq!(sig.active_param, 0);

    assert!(
        sig.label.starts_with("if("),
        "expected function signature label, got: {}",
        sig.label
    );
    assert_eq!(sig.label, "if(then: unknown, else: unknown) -> unknown");
}

#[test]
fn signature_help_postfix_if_active_param_then_else() {
    let c = ctx().build();

    t("true.if($0, 1)").ctx(c.clone()).expect_sig_active(0);
    t("true.if(1, $0)").ctx(c).expect_sig_active(1);
}

#[test]
fn signature_help_normal_if_has_no_receiver_and_includes_all_params() {
    let c = ctx().build();
    let (source, cursor) = fixture_source_and_cursor("if($0");
    let out = complete_with_context(&source, cursor, Some(&c));
    let sig = out
        .signature_help
        .as_ref()
        .expect("expected signature help");

    assert_eq!(sig.receiver, None);
    assert_eq!(
        sig.params,
        vec![
            "condition: boolean".to_string(),
            "then: unknown".to_string(),
            "else: unknown".to_string(),
        ]
    );
}

#[test]
fn completion_after_dot_only_offers_postfix_capable_functions() {
    let c = ctx().build();
    let (source, cursor) = fixture_source_and_cursor("true.$0");
    let out = complete_with_context(&source, cursor, Some(&c));

    assert!(
        out.items.iter().any(|i| i.label == ".if"),
        "expected postfix .if completion"
    );
    assert!(
        out.items.iter().all(|i| i.label != ".sum"),
        "did not expect .sum to be offered as a postfix method"
    );
}

#[test]
fn signature_help_postfix_non_postfix_capable_function_is_not_method_style() {
    let c = ctx().build();
    let (source, cursor) = fixture_source_and_cursor("true.sum($0");
    let out = complete_with_context(&source, cursor, Some(&c));
    let sig = out
        .signature_help
        .as_ref()
        .expect("expected signature help");

    assert_eq!(sig.receiver, None);
    assert_eq!(sig.label, "sum(values: number | number[], ...) -> number");
    assert!(
        !sig.label.contains(").sum("),
        "did not expect method-style signature label, got: {}",
        sig.label
    );
}

#[test]
fn completion_day_arg_kinds_are_single_run_each_for_ui_grouping() {
    let c = ctx().props_demo_basic().build();

    let (source, cursor) = fixture_source_and_cursor("day($0)");
    let out = complete_with_context(&source, cursor, Some(&c));

    assert!(
        out.preferred_indices.is_empty(),
        "expected preferred_indices to be empty (no fuzzy) inside `day(...)`\nactual preferred_indices: {:?}",
        out.preferred_indices
    );

    let mut runs = Vec::<CompletionKind>::new();
    let mut last = None;
    for item in &out.items {
        if last != Some(item.kind) {
            runs.push(item.kind);
            last = Some(item.kind);
        }
    }

    let mut seen = HashSet::<std::mem::Discriminant<CompletionKind>>::new();
    for kind in runs {
        let disc = std::mem::discriminant(&kind);
        assert!(
            seen.insert(disc),
            "completion kind run is fragmented; kind {:?} re-appeared after a different kind",
            kind
        );
    }
}

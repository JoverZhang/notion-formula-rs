use crate::completion::{CompletionConfig, CompletionData, complete};
use analyzer::semantic::Ty;
use analyzer::semantic::{Context, builtins_functions};
use crate::tests::completion_dsl::{Builtin, Func, Item, Prop, ctx, t};

#[test]
fn completion_after_atom_postfix_if_requires_if_in_context() {
    let c = ctx().props_demo_basic().without_funcs(&["if"]).build();

    t("(1+1)$0")
        .ctx(c)
        .expect_contains_items(&[Item::Builtin(Builtin::EqEq), Item::Builtin(Builtin::Plus)])
        .expect_not_postfix(Func::If);
}

#[test]
fn completion_member_access_prefix_filters_to_query_matches() {
    let c = ctx().build();

    t(r#""abc".rep$0"#)
        .ctx(c)
        .expect_contains_labels(&[".repeat()", ".replace()", ".replaceAll()"])
        .expect_not_contains_labels(&[".if()", ".test()", ".match()"]);
}

#[test]
fn completion_member_access_filters_postfix_items_strictly() {
    let c = ctx().build();

    t("true.rep$0").ctx(c.clone()).expect_not_postfix(Func::If);
    t("true.i$0").ctx(c).expect_postfix(Func::If);
}

#[test]
fn completion_member_access_filters_by_receiver_type() {
    let c = ctx().build();

    t("true.rep$0").ctx(c.clone()).expect_empty();
    t(r#""x".rep$0"#)
        .ctx(c)
        .expect_contains_labels(&[".repeat()", ".replace()", ".replaceAll()"])
        .expect_not_contains_labels(&[".if()", ".ifs()"]);
}

#[test]
fn completion_inside_call_arg_empty_does_not_apply_fuzzy_ranking() {
    let c = ctx().props_demo_basic().build();

    let start = t("$0").ctx(c.clone()).items_kinds_labels();
    t("empty($0)")
        .ctx(c)
        .expect_preferred_indices_empty()
        .expect_items_kinds_labels(&start);
}

#[test]
fn completion_non_ascii_query_disables_ranking_and_preserves_order() {
    let c = ctx().build();

    let baseline = complete("", 0, Some(&c), CompletionConfig::default());
    let baseline_items = baseline
        .items
        .iter()
        .map(|i| (i.kind, i.label.clone()))
        .collect::<Vec<_>>();

    // Cursor is inside the identifier, so the replace span is non-empty. Query ranking should be
    // disabled because the replace text contains non-ASCII characters.
    let source = "αβ";
    let cursor = "α".len();
    let out = complete(source, cursor, Some(&c), CompletionConfig::default());

    assert_eq!(out.replace.start, 0);
    assert_eq!(out.replace.end, u32::try_from(source.len()).unwrap());
    assert!(out.preferred_indices.is_empty());
    assert_eq!(
        out.items
            .iter()
            .map(|i| (i.kind, i.label.clone()))
            .collect::<Vec<_>>(),
        baseline_items
    );
}

#[test]
fn completion_fuzzy_ranking_orders_matches_and_computes_preferred_indices() {
    let c = ctx().build();

    t("rep$0")
        .ctx(c)
        .preferred_limit(3)
        .expect_order("repeat()", "toNumber()")
        .expect_order("replace()", "toNumber()")
        .expect_order("replaceAll()", "toNumber()")
        .expect_top_labels(&["repeat()", "replace()", "replaceAll()"])
        .expect_preferred_indices(&[0, 1, 2]);
}

#[test]
fn completion_preferred_limit_zero_disables_preferred_indices() {
    let c = ctx().build();

    t("rep$0")
        .ctx(c)
        .preferred_limit(0)
        .expect_preferred_indices_empty();
}

#[test]
fn completion_ranking_contains_beats_fuzzy() {
    let c = ctx()
        .only_funcs(&["mean", "median", "toNumber", "name"])
        .build();

    t("me$0")
        .ctx(c)
        .expect_order("mean()", "toNumber()")
        .expect_order("median()", "toNumber()")
        .expect_order("name()", "toNumber()");
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

    t("replace$0")
        .ctx(c)
        .expect_order("replace()", "replaceAll()");
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
fn completion_single_char_query_prefers_prefix_prop_over_short_contains() {
    let c = ctx().prop("Title", Ty::String).build();

    t("t$0")
        .ctx(c.clone())
        .expect_contains_labels(&["Title", "at()"])
        .expect_order("Title", "at()");

    let out = complete("t", 1, Some(&c), CompletionConfig::default());
    let top = out
        .items
        .iter()
        .take(5)
        .map(|i| i.label.as_str())
        .collect::<Vec<_>>();
    assert!(
        top.contains(&"Title"),
        "expected Title to be in the top 5 ranked items, got: {top:?}"
    );
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
fn completion_type_ranking_sum_number_prefers_number_list_props() {
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
fn completion_after_dot_only_offers_postfix_capable_functions() {
    let c = ctx().build();
    t("true.$0")
        .ctx(c)
        .expect_postfix(Func::If)
        .expect_not_contains_labels(&[".repeat()", ".replace()"])
        .expect_not_postfix(Func::Sum);
}

#[test]
fn completion_after_dot_unknown_receiver_keeps_full_postfix_set_for_now() {
    let c = ctx().build();
    t("foo.$0")
        .ctx(c)
        .expect_postfix(Func::If)
        .expect_contains_labels(&[".repeat()"]);
}

#[test]
fn completion_day_arg_kinds_are_single_run_each_for_ui_grouping() {
    let c = ctx().props_demo_basic().build();

    t("day($0)")
        .ctx(c)
        .expect_preferred_indices_empty()
        .expect_kind_runs_not_fragmented();
}

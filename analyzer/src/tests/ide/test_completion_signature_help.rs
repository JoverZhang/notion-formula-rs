use crate::tests::completion_dsl::{ctx, t};

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
fn signature_help_sum_variadic_number_only_case_1_empty_first_arg() {
    let c = ctx().build();
    t("sum($0")
        .ctx(c)
        .expect_sig_active(0)
        .expect_sig_label("sum(values1: number | number[], ...) -> number");
}

#[test]
fn signature_help_sum_variadic_number_only_case_2_single_number() {
    let c = ctx().build();
    t("sum(42$0)")
        .ctx(c)
        .expect_sig_active(0)
        .expect_sig_label("sum(values1: number, ...) -> number");
}

#[test]
fn signature_help_sum_variadic_number_only_case_2_list_literal() {
    let c = ctx().build();
    t("sum([1,2,3]$0)")
        .ctx(c)
        .expect_sig_active(0)
        .expect_sig_label("sum(values1: number[], ...) -> number");
}

#[test]
fn signature_help_sum_variadic_number_only_case_3_second_arg_empty() {
    let c = ctx().build();
    t("sum(42, $0)")
        .ctx(c)
        .expect_sig_active(1)
        .expect_sig_label(
            "sum(values1: number, values2: number | number[], ...) -> number",
        );
}

#[test]
fn signature_help_sum_variadic_number_only_case_4_two_numbers() {
    let c = ctx().build();
    t("sum(42, 42$0)")
        .ctx(c)
        .expect_sig_active(1)
        .expect_sig_label(
            "sum(values1: number, values2: number, ...) -> number",
        );
}

#[test]
fn signature_help_sum_prefers_known_actual_types_for_union_slots() {
    let c = ctx().prop("Number", crate::semantic::Ty::Number).build();
    t(r#"sum(prop("Number"), [1, 2, 3]$0)"#)
        .ctx(c)
        .expect_sig_label("sum(values1: number, values2: number[], ...) -> number");
}

#[test]
fn signature_help_postfix_if_label_format() {
    let c = ctx().build();
    t("true.if($0, 1)")
        .ctx(c)
        .expect_sig_active(0)
        .expect_sig_label("(condition: boolean).if(then: number, else: number) -> number");
}

#[test]
fn signature_help_postfix_if_receiver_is_not_overridden_by_ill_typed_receiver() {
    let c = ctx().build();
    t("(1).if(42, \"42\"$0)")
        .ctx(c.clone())
        .expect_sig_label("(condition: boolean).if(then: number, else: string) -> number | string");

    t("(1 == 1).if(42, \"42\"$0)")
        .ctx(c)
        .expect_sig_label("(condition: boolean).if(then: number, else: string) -> number | string");
}

#[test]
fn signature_help_postfix_ifs_uses_method_style_and_boolean_receiver() {
    let c = ctx().build();

    t("(1 == 1).ifs(42, \"42\"$0)")
        .ctx(c.clone())
        .expect_sig_label(
            "(condition1: boolean).ifs(value1: number, ..., default: string) -> number | string",
        );

    // Ill-typed receiver should not override the hard-constrained boolean receiver slot.
    t("(1).ifs(42, \"42\"$0)").ctx(c).expect_sig_label(
        "(condition1: boolean).ifs(value1: number, ..., default: string) -> number | string",
    );
}

#[test]
fn signature_help_postfix_ifs_third_condition_highlights_condition3() {
    let c = ctx()
        .prop("Number", crate::semantic::Ty::Number)
        .prop("Title", crate::semantic::Ty::String)
        .prop("Date", crate::semantic::Ty::Date)
        .build();

    t(
        r#"
(prop("Number") < 1).ifs(
  prop("Title"),
  prop("Number") < 2,
  [prop("Number")],
  prop("Number") < 3$0,
  prop("Date"),
  4
)"#,
    )
    .ctx(c)
    .expect_sig_active(3)
    .expect_sig_active_param_name("condition3");
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
    t("if($0")
        .ctx(c)
        .expect_sig_label("if(condition: boolean, then: unknown, else: unknown) -> unknown");
}

#[test]
fn signature_help_ifs_repeat_group_label_format() {
    let c = ctx().build();
    t("ifs(true, 1, false, 2, 3$0)")
        .ctx(c)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: number, condition2: boolean, value2: number, ..., default: number) -> number",
        );
}

#[test]
fn signature_help_if_shows_instantiated_union_return_type() {
    let c = ctx().build();
    t("if(true, 1, \"x\"$0)")
        .ctx(c)
        .expect_sig_active(2)
        .expect_sig_label("if(condition: boolean, then: number, else: string) -> number | string");
}

#[test]
fn signature_help_if_propagates_unknown() {
    let c = ctx().build();
    t("if(true, x, 1$0)")
        .ctx(c)
        .expect_sig_active(2)
        .expect_sig_label("if(condition: boolean, then: unknown, else: number) -> unknown");
}

#[test]
fn signature_help_ifs_shows_instantiated_union_return_type() {
    let c = ctx().build();
    t("ifs(true, 1, false, 2, \"a\"$0)")
        .ctx(c)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: number, condition2: boolean, value2: number, ..., default: string) -> number | string",
        );
}

#[test]
fn signature_help_ifs_propagates_unknown() {
    let c = ctx().build();
    t("ifs(true, x, false, 1, 2$0)")
        .ctx(c)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: unknown, condition2: boolean, value2: number, ..., default: number) -> unknown",
        );
}

#[test]
fn signature_help_ifs_single_group_highlights_default_and_omits_second_group() {
    let c = ctx().build();
    t("ifs(true, \"42\", $0)")
        .ctx(c)
        .expect_sig_active(2)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: string, ..., default: string) -> string",
        );
}

#[test]
fn signature_help_ifs_invalid_total_4_guides_to_value2() {
    let c = ctx().build();
    t("ifs(true, \"42\", false, $0)")
        .ctx(c)
        .expect_sig_active(3)
        .expect_sig_label("ifs(condition1: boolean, value1: string, condition2: boolean, value2: string, ..., default: string) -> string");
}

#[test]
fn signature_help_ifs_does_not_override_hard_constrained_condition_types() {
    let c = ctx().build();
    t("ifs(true, \"42\", 42, $0)")
        .ctx(c)
        .expect_sig_active(3)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: string, condition2: boolean, value2: string, ..., default: string) -> string",
        );
}

#[test]
fn signature_help_ifs_total_5_highlights_default() {
    let c = ctx().build();
    t("ifs(true, \"42\", false, 7, $0)")
        .ctx(c)
        .expect_sig_active(4)
        .expect_sig_label("ifs(condition1: boolean, value1: string, condition2: boolean, value2: number, ..., default: number | string) -> number | string");
}

#[test]
fn signature_help_ifs_long_call_highlights_repeat_cycle_preserving_position() {
    let c = ctx().build();
    t("ifs(true, \"a\", false, \"b\", true, $0)")
        .ctx(c)
        .expect_sig_active(5);
}

#[test]
fn signature_help_ifs_active_param_empty_default_highlights_tail() {
    let c = ctx().build();
    t("ifs(true, \"123\", true, \"123\", $0)")
        .ctx(c)
        .expect_sig_active(4);
}

#[test]
fn signature_help_ifs_active_param_repeat_value_highlights_value() {
    let c = ctx().build();
    t("ifs(true, \"123\", true, \"123\", true, $0)")
        .ctx(c)
        .expect_sig_active(5);
}

#[test]
fn signature_help_postfix_non_postfix_capable_function_is_not_method_style() {
    let c = ctx().build();
    t("true.sum($0")
        .ctx(c)
        .expect_sig_label("sum(values1: number | number[], ...) -> number")
        .expect_sig_label_not_contains(").sum(");
}

#[test]
fn signature_help_if_list_of_union_is_parenthesized() {
    let c = ctx().build();
    t("if(true, 42, [42, \"42\"]$0)")
        .ctx(c)
        .expect_sig_active(2)
        .expect_sig_label(
            "if(condition: boolean, then: number, else: (number | string)[]) -> number | (number | string)[]",
        );
}

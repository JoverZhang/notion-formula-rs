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
fn signature_help_label_sum_union_variadic() {
    let c = ctx().build();
    t("sum($0")
        .ctx(c)
        .expect_sig_label("sum(values: number | number[], ...) -> number");
}

#[test]
fn signature_help_postfix_if_label_format() {
    let c = ctx().build();
    t("true.if($0, 1)")
        .ctx(c)
        .expect_sig_receiver(Some("condition: boolean"))
        .expect_sig_params(&["then: T0", "else: T0"])
        .expect_sig_active(0)
        .expect_sig_label("if(then: T0, else: T0) -> T0");
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
        .expect_sig_receiver(None)
        .expect_sig_params(&["condition: boolean", "then: T0", "else: T0"]);
}

#[test]
fn signature_help_ifs_repeat_group_label_format() {
    let c = ctx().build();
    t("ifs(true, 1, false, 2, 3$0)")
        .ctx(c)
        .expect_sig_label(
            "ifs(condition1: boolean, value1: T0, condition2: boolean, value2: T0, ..., default: T0) -> T0",
        );
}

#[test]
fn signature_help_postfix_non_postfix_capable_function_is_not_method_style() {
    let c = ctx().build();
    t("true.sum($0")
        .ctx(c)
        .expect_sig_receiver(None)
        .expect_sig_label("sum(values: number | number[], ...) -> number")
        .expect_sig_label_not_contains(").sum(");
}

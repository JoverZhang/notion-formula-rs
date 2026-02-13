use crate::analyze_syntax;
use crate::ast::ExprKind;
use crate::diagnostics::DiagnosticKind;

#[test]
fn test_trailing_tokens_error() {
    let result = analyze_syntax("1 2");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].kind, DiagnosticKind::Error);
    assert!(
        result.diagnostics[0].message.contains("unexpected token"),
        "unexpected message: {}",
        result.diagnostics[0].message
    );
}

#[test]
fn test_multiple_errors_collected() {
    // Missing operand before ')' and an unmatched ')'
    let result = analyze_syntax("(1 + ) 3");
    assert!(
        result.diagnostics.len() >= 2,
        "expected at least two diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn diagnostics_list_trailing_comma_recovers() {
    let result = analyze_syntax("[1,2,]");
    let trailing = result
        .diagnostics
        .iter()
        .find(|d| d.kind == DiagnosticKind::Error && d.message.contains("trailing comma"))
        .unwrap_or_else(|| panic!("unexpected diagnostics: {:?}", result.diagnostics));
    assert!(
        trailing
            .labels
            .iter()
            .any(|l| l.message.as_deref() == Some("remove this comma")),
        "expected trailing-comma diagnostic to include a removal hint label, got {:?}",
        trailing
    );
    assert!(
        trailing.actions.iter().any(|action| {
            action.title == "Remove trailing comma"
                && action
                    .edits
                    .iter()
                    .any(|e| e.range == trailing.span && e.new_text.is_empty())
        }),
        "expected trailing-comma diagnostic to include a structured action, got {:?}",
        trailing.actions
    );

    match &result.expr.kind {
        ExprKind::List { items } => assert_eq!(items.len(), 2),
        other => panic!("expected List, got {:?}", other),
    }
}

#[test]
fn diagnostics_call_missing_close_paren_has_insert_label() {
    let result = analyze_syntax("f(1");
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.message.starts_with("expected ')',"))
        .unwrap_or_else(|| panic!("unexpected diagnostics: {:?}", result.diagnostics));

    assert!(
        diag.labels
            .iter()
            .any(|l| l.message.as_deref() == Some("this '(' is not closed")),
        "expected missing-close-paren diagnostic to label the opening delimiter, got {:?}",
        diag
    );
    assert!(
        diag.labels
            .iter()
            .any(|l| l.message.as_deref() == Some("insert ')'")),
        "expected missing-close-paren diagnostic to include an insertion hint, got {:?}",
        diag
    );
    assert!(
        diag.actions.iter().any(|action| {
            action.title == "Insert `)`"
                && action
                    .edits
                    .iter()
                    .any(|e| e.range.start == 3 && e.range.end == 3 && e.new_text == ")")
        }),
        "expected diagnostic to include missing-close-paren action, got {:?}",
        diag.actions
    );
}

#[test]
fn diagnostics_missing_comma_between_call_args_has_insert_label() {
    let result = analyze_syntax("f(1 2)");
    let diag = result
        .diagnostics
        .iter()
        .find(|d| {
            d.labels
                .iter()
                .any(|l| l.message.as_deref() == Some("insert ','"))
        })
        .unwrap_or_else(|| panic!("unexpected diagnostics: {:?}", result.diagnostics));
    assert!(
        diag.message.contains("expected ','"),
        "expected missing-comma diagnostic message, got {:?}",
        diag
    );
    assert!(
        diag.actions.iter().any(|action| {
            action.title == "Insert `,`"
                && action
                    .edits
                    .iter()
                    .any(|e| e.range.start == 4 && e.range.end == 4 && e.new_text == ",")
        }),
        "expected diagnostic to include missing-comma action, got {:?}",
        diag.actions
    );
}

#[test]
fn diagnostics_eof_deconflict_prefers_missing_close_delimiter() {
    // `f(1,` can lead to both "missing expr after comma" and "missing ')'" at EOF.
    // Ensure we only emit the delimiter diagnostic at that insertion point.
    let result = analyze_syntax("f(1,");
    assert_eq!(
        result.diagnostics.len(),
        1,
        "expected a single EOF diagnostic, got {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics[0].message.starts_with("expected ')',"),
        "unexpected message: {:?}",
        result.diagnostics[0]
    );
}

#[test]
fn diagnostics_mismatched_delimiter_suggests_replacement() {
    let result = analyze_syntax("(1]");
    assert_eq!(
        result.diagnostics.len(),
        1,
        "expected a single mismatched-delimiter diagnostic, got {:?}",
        result.diagnostics
    );
    let diag = &result.diagnostics[0];
    assert!(
        diag.message.starts_with("expected ')',"),
        "unexpected message: {diag:?}"
    );
    assert!(
        diag.labels
            .iter()
            .any(|l| l.message.as_deref() == Some("this '(' is not closed")),
        "expected opening delimiter label, got {diag:?}"
    );
    assert!(
        diag.labels
            .iter()
            .any(|l| l.message.as_deref() == Some("replace `]` with ')'")),
        "expected replacement suggestion label, got {diag:?}"
    );
    assert!(
        diag.actions.iter().any(|action| {
            action.title.contains("Replace")
                && action
                    .edits
                    .iter()
                    .any(|e| e.range.start == 2 && e.range.end == 3 && e.new_text == ")")
        }),
        "expected diagnostic to include mismatched-delimiter replacement action, got {:?}",
        diag.actions
    );
}

#[test]
fn diagnostics_lex_error_has_no_actions() {
    let result = analyze_syntax("1 @");
    assert!(
        !result.diagnostics.is_empty(),
        "expected lex diagnostics, got {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.iter().all(|d| d.actions.is_empty()),
        "lex diagnostics must not carry code actions, got {:?}",
        result.diagnostics
    );
}

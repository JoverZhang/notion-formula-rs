use crate::analyze;
use crate::ast::ExprKind;
use crate::diagnostics::DiagnosticKind;

#[test]
fn test_trailing_tokens_error() {
    let result = analyze("1 2").unwrap();
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
    let result = analyze("(1 + ) 3").unwrap();
    assert!(
        result.diagnostics.len() >= 2,
        "expected at least two diagnostics, got {:?}",
        result.diagnostics
    );
}

#[test]
fn diagnostics_list_trailing_comma_recovers() {
    let result = analyze("[1,2,]").unwrap();
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
        trailing.labels.iter().any(|l| {
            l.quick_fix
                .as_ref()
                .is_some_and(|fix| fix.new_text.is_empty() && fix.title == "Remove trailing comma")
        }),
        "expected trailing-comma diagnostic to include structured quick-fix metadata, got {:?}",
        trailing
    );

    match &result.expr.kind {
        ExprKind::List { items } => assert_eq!(items.len(), 2),
        other => panic!("expected List, got {:?}", other),
    }
}

#[test]
fn diagnostics_call_missing_close_paren_has_insert_label() {
    let result = analyze("f(1").unwrap();
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
        diag.labels.iter().any(|l| {
            l.quick_fix
                .as_ref()
                .is_some_and(|fix| fix.new_text == ")" && fix.title == "Insert `)`")
        }),
        "expected missing-close-paren diagnostic to include structured quick-fix metadata, got {:?}",
        diag
    );
}

#[test]
fn diagnostics_missing_comma_between_call_args_has_insert_label() {
    let result = analyze("f(1 2)").unwrap();
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
        diag.labels.iter().any(|l| {
            l.quick_fix
                .as_ref()
                .is_some_and(|fix| fix.new_text == "," && fix.title == "Insert `,`")
        }),
        "expected missing-comma diagnostic to include structured quick-fix metadata, got {:?}",
        diag
    );
}

#[test]
fn diagnostics_eof_deconflict_prefers_missing_close_delimiter() {
    // `f(1,` can lead to both "missing expr after comma" and "missing ')'" at EOF.
    // Ensure we only emit the delimiter diagnostic at that insertion point.
    let result = analyze("f(1,").unwrap();
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
    let result = analyze("(1]").unwrap();
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
        diag.labels.iter().any(|l| {
            l.quick_fix
                .as_ref()
                .is_some_and(|fix| fix.new_text == ")" && fix.title.contains("Replace"))
        }),
        "expected mismatched-delimiter diagnostic to include structured replacement quick-fix metadata, got {diag:?}"
    );
}

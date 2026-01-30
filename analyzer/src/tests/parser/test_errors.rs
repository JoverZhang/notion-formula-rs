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
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::Error && d.message.contains("trailing comma")),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    match &result.expr.kind {
        ExprKind::List { items } => assert_eq!(items.len(), 2),
        other => panic!("expected List, got {:?}", other),
    }
}

use crate::analyze;
use crate::diagnostics::DiagnosticKind;

#[test]
fn test_trailing_tokens_error() {
    let result = analyze("1 2").unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].kind, DiagnosticKind::Error);
    assert!(
        result.diagnostics[0].message.contains("Unexpected token"),
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

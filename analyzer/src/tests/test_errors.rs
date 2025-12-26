use crate::parser::ParseError;
use crate::analyze;

#[test]
fn test_trailing_tokens_error() {
    let result = analyze("1 2").unwrap();
    assert_eq!(result.errors.len(), 1);
    match &result.errors[0] {
        ParseError::UnexpectedToken { expected, .. } => assert_eq!(expected, "EOF"),
        other => panic!("unexpected error: {:?}", other),
    };
}

#[test]
fn test_multiple_errors_collected() {
    // Missing operand before ')' and an unmatched ')'
    let result = analyze("(1 + ) 3").unwrap();
    assert!(result.errors.len() >= 2);
    assert!(matches!(
        result.errors[0],
        ParseError::UnexpectedToken { .. }
    ));
    assert!(matches!(
        result.errors[1],
        ParseError::UnexpectedToken { .. }
    ));
}

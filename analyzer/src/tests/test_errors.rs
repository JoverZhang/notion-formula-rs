use crate::parser::ParseError;
use crate::analyze;

#[test]
fn test_trailing_tokens_error() {
    let err = analyze("1 2").unwrap_err();
    match err {
        ParseError::UnexpectedToken { expected, .. } => {
            assert_eq!(expected, "EOF");
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

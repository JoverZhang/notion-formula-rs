use crate::{analyze, format_expr};

fn assert_format_idempotent(input: &str) {
    let a1 = analyze(input).unwrap();
    assert!(
        a1.diagnostics.is_empty(),
        "expected no parse errors for input {input}, got {:?}",
        a1.diagnostics
    );
    let f1 = format_expr(&a1.expr, input, &a1.tokens);
    let a2 = analyze(&f1).unwrap();
    assert!(
        a2.diagnostics.is_empty(),
        "format-produced input should parse cleanly: {f1}, errors: {:?}",
        a2.diagnostics
    );
    let f2 = format_expr(&a2.expr, &f1, &a2.tokens);
    assert_eq!(f1, f2, "input: {input}");
}

#[test]
fn test_format_idempotence_cases() {
    let cases = [
        "1+2*3",
        "(1+2)*3",
        "2^3^4",
        "(2^3)^4",
        "a&&b||c",
        "a==b||c==d",
        "!a&&-b",
        "1 ? 2 : 3",
        "1 ? 2 : 3 ? 4 : 5",
        r#"prop("Title",1+2*3)"#,
        "f()",
        "f(1,2,3)",
    ];

    for input in cases {
        assert_format_idempotent(input);
    }
}

#[test]
fn format_preserves_not_and_bang_notation() {
    let cases = [
        ("not not a", "not not a\n"),
        ("!!a", "!!a\n"),
        ("not !a", "not !a\n"),
    ];

    for (input, expected) in cases {
        let out = analyze(input).unwrap();
        assert!(
            out.diagnostics.is_empty(),
            "expected no parse errors for input {input}, got {:?}",
            out.diagnostics
        );
        let formatted = format_expr(&out.expr, input, &out.tokens);
        assert_eq!(formatted, expected, "input: {input}");
    }
}

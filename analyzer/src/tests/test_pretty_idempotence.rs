use crate::analyze;

fn assert_pretty_idempotent(input: &str) {
    let a1 = analyze(input).unwrap();
    assert!(
        a1.errors.is_empty(),
        "expected no parse errors for input {input}, got {:?}",
        a1.errors
    );
    let p1 = a1.expr.pretty();
    let a2 = analyze(&p1).unwrap();
    assert!(
        a2.errors.is_empty(),
        "pretty-produced input should parse cleanly: {p1}, errors: {:?}",
        a2.errors
    );
    let p2 = a2.expr.pretty();
    assert_eq!(p1, p2, "input: {input}");
}

#[test]
fn test_pretty_idempotence_cases() {
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
        assert_pretty_idempotent(input);
    }
}

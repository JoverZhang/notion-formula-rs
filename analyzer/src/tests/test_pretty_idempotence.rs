use crate::analyze;

fn assert_pretty_idempotent(input: &str) {
    let a1 = analyze(input).unwrap();
    let p1 = a1.pretty();
    let a2 = analyze(&p1).unwrap();
    let p2 = a2.pretty();
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
        r#"prop("Title",1+2*3)"#,
        "f()",
        "f(1,2,3)",
    ];

    for input in cases {
        assert_pretty_idempotent(input);
    }
}

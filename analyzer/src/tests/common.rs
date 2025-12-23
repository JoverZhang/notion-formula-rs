
pub fn trim_indent(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        // Skip the first line (which is the empty line)
        .skip(1)
        .map(|l| {
            if l.len() >= min_indent {
                &l[min_indent..]
            } else {
                *l
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

#[test]
fn test_trim_indent() {
    let s = r#"
        if(
            prop("Title"),
            1,
            0
        )"#;
    let expected = "if(\n    prop(\"Title\"),\n    1,\n    0\n)";
    assert_eq!(expected, trim_indent(s));
}

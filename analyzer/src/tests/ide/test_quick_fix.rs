use crate::{analyze, formatted_if_syntax_valid, quick_fixes};

#[test]
fn formatted_is_empty_for_syntax_errors() {
    let source = r#"123 "456""#;
    let output = analyze(source).expect("expected parse output with diagnostics");
    assert!(!output.diagnostics.is_empty());

    let formatted =
        formatted_if_syntax_valid(&output.expr, source, &output.tokens, &output.diagnostics);
    assert_eq!(formatted, "");
    assert!(quick_fixes(&output.diagnostics).is_empty());
}

#[test]
fn formatted_still_available_for_semantic_errors() {
    let source = r#"prop("Missing")"#;
    let mut output = analyze(source).expect("expected parse output");
    assert!(output.diagnostics.is_empty());

    let ctx = crate::semantic::Context {
        properties: vec![],
        functions: crate::semantic::builtins_functions(),
    };
    let (_, semantic_diags) = crate::semantic::analyze_expr(&output.expr, &ctx);
    assert!(!semantic_diags.is_empty());
    output.diagnostics.extend(semantic_diags);

    let formatted =
        formatted_if_syntax_valid(&output.expr, source, &output.tokens, &output.diagnostics);
    assert!(!formatted.is_empty());
    assert!(quick_fixes(&output.diagnostics).is_empty());
}

#[test]
fn quick_fix_missing_close_paren() {
    let source = "(123";
    let output = analyze(source).expect("expected parse output with diagnostics");
    let fixes = quick_fixes(&output.diagnostics);

    assert!(fixes.iter().all(|f| f.edits.len() == 1));
    assert!(fixes.iter().any(|f| {
        f.edits.iter().any(|e| {
            e.range.start == source.len() as u32
                && e.range.end == source.len() as u32
                && e.new_text == ")"
        })
    }));
}

#[test]
fn quick_fix_missing_comma_between_args() {
    let source = "f(1 2)";
    let output = analyze(source).expect("expected parse output with diagnostics");
    let fixes = quick_fixes(&output.diagnostics);

    assert!(fixes.iter().any(|f| {
        f.edits
            .iter()
            .any(|e| e.range.start == 4 && e.range.end == 4 && e.new_text == ",")
    }));
}

#[test]
fn quick_fix_trailing_comma() {
    let source = "[1,2,]";
    let output = analyze(source).expect("expected parse output with diagnostics");
    let fixes = quick_fixes(&output.diagnostics);

    assert!(fixes.iter().any(|f| {
        f.edits
            .iter()
            .any(|e| e.range.start == 4 && e.range.end == 5 && e.new_text.is_empty())
    }));
}

#[test]
fn quick_fix_mismatched_delimiter() {
    let source = "(1]";
    let output = analyze(source).expect("expected parse output with diagnostics");
    let fixes = quick_fixes(&output.diagnostics);

    assert!(fixes.iter().any(|f| {
        f.edits
            .iter()
            .any(|e| e.range.start == 2 && e.range.end == 3 && e.new_text == ")")
    }));
}

#[test]
fn quick_fix_lex_error_returns_no_fixes() {
    let source = "1 @";
    let output = analyze(source).expect("expected parse output with diagnostics");
    let fixes = quick_fixes(&output.diagnostics);
    assert!(fixes.is_empty());
}

use analyzer::semantic::{Context, builtins_functions};
use analyzer::Span;
use crate::{CompletionConfig, IdeError, TextEdit, apply_edits, format, help};

#[test]
fn ide_format_reports_error_on_syntax_errors() {
    let err = format("1 +", 0).expect_err("expected format error");
    assert_eq!(err, IdeError::FormatError);
}

#[test]
fn ide_format_rebases_cursor_through_full_replace() {
    let out = format("1+2", 1).expect("expected formatted output");
    assert_eq!(out.cursor, 0);
}

#[test]
fn ide_apply_edits_rejects_overlapping_ranges() {
    let edits = vec![
        TextEdit {
            range: Span { start: 1, end: 3 },
            new_text: "X".to_string(),
        },
        TextEdit {
            range: Span { start: 2, end: 4 },
            new_text: "Y".to_string(),
        },
    ];

    let err = apply_edits("abcd", edits, 0).expect_err("expected overlap error");
    assert_eq!(err, IdeError::OverlappingEdits);
}

#[test]
fn ide_apply_edits_applies_and_rebases_cursor() {
    let edits = vec![TextEdit {
        range: Span { start: 1, end: 2 },
        new_text: "XYZ".to_string(),
    }];

    let out = apply_edits("abcd", edits, 3).expect("expected edits to apply");
    assert_eq!(out.source, "aXYZcd");
    assert_eq!(out.cursor, 5);
}

#[test]
fn ide_help_splits_completion_and_signature_help() {
    let ctx = Context {
        properties: Vec::new(),
        functions: builtins_functions(),
    };
    let out = help("if(", 3, &ctx, CompletionConfig::default());

    assert!(
        out.signature_help.is_some(),
        "expected signature help inside call"
    );
    assert!(out.completion.replace.start <= 3);
    assert!(out.completion.replace.end >= out.completion.replace.start);
}

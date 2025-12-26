use std::fs;

use analyzer::{analyze, format_expr};

#[test]
fn format_snapshots() {
    insta::glob!("format/*.formula", |path| {
        let source = fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("failed to read {:?}: {}", path, e);
        });

        let out = analyze(&source).unwrap_or_else(|e| {
            panic!("parse failed for {:?}: {:?}", path, e);
        });

        assert!(
            out.diagnostics.is_empty(),
            "expected no diagnostics for {:?}, got {:?}",
            path,
            out.diagnostics
        );

        let formatted = format_expr(&out.expr, &source, &out.tokens);

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("snapshot");

        insta::assert_snapshot!(name, formatted);
    });
}

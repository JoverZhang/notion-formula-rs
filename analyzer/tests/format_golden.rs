mod common;

use std::path::Path;

use analyzer::{analyze, format_expr};
use common::golden::run_golden_dir;

#[test]
fn format_golden() {
    run_golden_dir(
        "format_golden",
        Path::new("tests/format"),
        "snap",
        |path, source| {
            let out = analyze(source).unwrap_or_else(|e| {
                panic!("analyze() returned Err for {:?}: {:?}", path, e);
            });

            assert!(
                out.diagnostics.is_empty(),
                "expected no diagnostics for {:?}, got {:?}",
                path,
                out.diagnostics
            );

            let mut formatted = format_expr(&out.expr, source, &out.tokens);
            if !formatted.ends_with('\n') {
                formatted.push('\n');
            }
            formatted
        },
    );
}

mod common;

use std::path::Path;

use analyzer::analyze_syntax;
use common::golden::run_golden_dir;

#[test]
fn format_golden() {
    run_golden_dir(
        "format_golden",
        Path::new("tests/format"),
        "snap",
        |path, source| {
            let out = analyze_syntax(source);

            assert!(
                out.diagnostics.is_empty(),
                "expected no diagnostics for {:?}, got {:?}",
                path,
                out.diagnostics
            );

            ide::format(source, 0)
                .unwrap_or_else(|err| {
                    panic!("expected format success for {:?}, got {:?}", path, err)
                })
                .source
        },
    );
}

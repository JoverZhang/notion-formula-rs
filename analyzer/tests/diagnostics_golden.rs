mod common;

use std::path::Path;

use analyzer::{analyze, format_diagnostics};
use common::golden::run_golden_dir;

#[test]
fn diagnostics_golden() {
    run_golden_dir(
        "diagnostics_golden",
        Path::new("tests/diagnostics"),
        "snap",
        |path, source| {
            let out = analyze(source).unwrap_or_else(|e| {
                panic!(
                    "analyze() returned Err for {:?}: {:?}\n\
                     (If lexer errors should be golden-tested too, change analyze() \
                     to return ParseOutput with diagnostics even on fatal errors.)",
                    path, e
                )
            });

            format_diagnostics(source, out.diagnostics)
        },
    );
}

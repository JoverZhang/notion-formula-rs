mod common;

use std::path::Path;

use analyzer::{analyze, analyze_with_context, format_diagnostics};
use analyzer::semantic::{Context, Property, Ty};
use common::golden::run_golden_dir;

#[test]
fn diagnostics_golden() {
    run_golden_dir(
        "diagnostics_golden",
        Path::new("tests/diagnostics"),
        "snap",
        |path, source| {
            let is_semantic = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s == "semantic_basic")
                .unwrap_or(false);

            let out = if is_semantic {
                let ctx = Context {
                    properties: vec![Property {
                        name: "Title".into(),
                        ty: Ty::String,
                    }],
                };
                analyze_with_context(source, ctx).unwrap_or_else(|e| {
                    panic!(
                        "analyze_with_context() returned Err for {:?}: {:?}\n\
                         (If lexer errors should be golden-tested too, change analyze() \
                         to return ParseOutput with diagnostics even on fatal errors.)",
                        path, e
                    )
                })
            } else {
                analyze(source).unwrap_or_else(|e| {
                    panic!(
                        "analyze() returned Err for {:?}: {:?}\n\
                         (If lexer errors should be golden-tested too, change analyze() \
                         to return ParseOutput with diagnostics even on fatal errors.)",
                        path, e
                    )
                })
            };

            format_diagnostics(source, out.diagnostics)
        },
    );
}

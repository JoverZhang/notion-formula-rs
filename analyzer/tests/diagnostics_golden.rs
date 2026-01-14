mod common;

use std::path::Path;

use analyzer::semantic::{Context, Property, Ty};
use analyzer::{analyze, analyze_with_context, format_diagnostics};
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
                        disabled_reason: None,
                    }],
                    functions: vec![],
                };
                analyze_with_context(source, ctx)
                    .expect("analyze_with_context() should return ParseOutput")
            } else {
                analyze(source).expect("analyze() should return ParseOutput")
            };

            format_diagnostics(source, out.diagnostics)
        },
    );
}

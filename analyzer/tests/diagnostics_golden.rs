mod common;

use std::path::Path;

use analyzer::semantic::{Context, Property, Ty, builtins_functions};
use analyzer::{analyze, analyze_syntax, format_diagnostics};
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
                .map(|s| s.starts_with("semantic_"))
                .unwrap_or(false);

            let diagnostics = if is_semantic {
                let ctx = Context {
                    properties: vec![Property {
                        name: "Title".into(),
                        ty: Ty::String,
                        disabled_reason: None,
                    }],
                    functions: builtins_functions(),
                };
                analyze(source, &ctx).diagnostics
            } else {
                analyze_syntax(source).diagnostics
            };

            format_diagnostics(source, diagnostics)
        },
    );
}

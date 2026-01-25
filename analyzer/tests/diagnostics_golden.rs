mod common;

use std::path::Path;

use analyzer::semantic::{Context, Property, Ty, builtins_functions};
use analyzer::{analyze, format_diagnostics};
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

            let out = if is_semantic {
                let ctx = Context {
                    properties: vec![Property {
                        name: "Title".into(),
                        ty: Ty::String,
                        disabled_reason: None,
                    }],
                    functions: builtins_functions(),
                };
                let mut out = analyze(source).expect("analyze() should return ParseOutput");
                let (_, diags) = analyzer::semantic::analyze_expr(&out.expr, &ctx);
                out.diagnostics.extend(diags);
                out
            } else {
                analyze(source).expect("analyze() should return ParseOutput")
            };

            format_diagnostics(source, out.diagnostics)
        },
    );
}

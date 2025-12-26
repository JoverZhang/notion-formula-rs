use std::fs;
use std::path::{Path, PathBuf};

use analyzer::{analyze, format_diagnostics};

#[test]
fn diagnostics_golden() {
    let dir = Path::new("tests/diagnostics");

    // Collect all *.formula files and sort them for deterministic order.
    let mut inputs: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read diagnostics dir {:?}: {}", dir, e))
        .filter_map(|ent| ent.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("formula"))
        .collect();

    inputs.sort();

    let update = std::env::var("UPDATE_GOLDEN").is_ok();

    for input in inputs {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>");

        let source = fs::read_to_string(&input)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", input, e));

        let out = analyze(&source).unwrap_or_else(|e| {
            panic!(
                "analyze() returned Err for {:?}: {:?}\n\
                 (If lexer errors should be golden-tested too, change analyze() \
                 to return ParseOutput with diagnostics even on fatal errors.)",
                input, e
            )
        });

        let rendered = format_diagnostics(&source, out.diagnostics);

        let golden_path = input.with_extension("snap");

        if update {
            fs::write(&golden_path, &rendered)
                .unwrap_or_else(|e| panic!("failed to write golden file {:?}: {}", golden_path, e));
            continue;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|_| {
            panic!(
                "missing golden file: {:?}\n\
                 To create/update golden files, run:\n\
                   UPDATE_GOLDEN=1 cargo test -p analyzer diagnostics_golden\n\
                 or simply:\n\
                   UPDATE_GOLDEN=1 cargo test diagnostics_golden\n\
                 (then re-run tests without UPDATE_GOLDEN)\n\
                 Test case: {}",
                golden_path, stem
            )
        });

        // Compare after normalizing trailing whitespace differences (optional but helpful).
        let expected_norm = normalize(&expected);
        let rendered_norm = normalize(&rendered);

        assert_eq!(
            expected_norm, rendered_norm,
            "golden mismatch for {}\ninput: {:?}\ngolden: {:?}\n\
             To update:\n  UPDATE_GOLDEN=1 cargo test diagnostics_golden\n",
            stem, input, golden_path
        );
    }
}

// Keep normalization minimal; avoid changing semantic content.
// This only trims trailing whitespace on each line and ensures a trailing newline.
fn normalize(s: &str) -> String {
    let mut out = String::new();
    for line in s.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

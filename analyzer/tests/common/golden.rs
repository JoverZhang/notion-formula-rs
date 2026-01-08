use std::fs;
use std::path::{Path, PathBuf};

/// Run golden-file comparisons for every `*.formula` file in `dir`, using `golden_ext` for expected files.
/// The `render` callback receives the input path and source contents and should return the actual output.
pub fn run_golden_dir<F>(test_name: &str, dir: &Path, golden_ext: &str, mut render: F)
where
    F: FnMut(&Path, &str) -> String,
{
    let mut inputs: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read test dir {:?}: {}", dir, e))
        .filter_map(|ent| ent.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("formula"))
        .collect();

    inputs.sort();

    let bless = std::env::var("BLESS").is_ok();

    for input in inputs {
        let source = fs::read_to_string(&input)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", input, e));

        let actual = render(&input, &source);
        let golden_path = input.with_extension(golden_ext);

        if bless {
            write_golden(&golden_path, &source, &actual);
            continue;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|_| {
            write_golden(&golden_path, &source, &actual);
            panic!(
                "generated missing golden file for {:?}\n\
                 golden path: {:?}\n\
                 Please review, git add it, then re-run tests (or run with BLESS=1 cargo test {}).",
                input, golden_path, test_name
            )
        });

        let expected_norm = normalize_output(&extract_output(&expected));
        let actual_norm = normalize_output(&actual);

        assert_eq!(
            expected_norm, actual_norm,
            "golden mismatch\ninput: {:?}\ngolden: {:?}\nTo update: BLESS=1 cargo test {}",
            input, golden_path, test_name
        );
    }
}

const OUTPUT_MARKER: &str = "=== OUTPUT ===";

fn write_golden(path: &Path, source: &str, output: &str) {
    let mut contents = String::new();
    contents.push_str("=== INPUT ===\n");
    contents.push_str(source);
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str("=== OUTPUT ===\n");
    contents.push_str(&normalize_output(output));

    fs::write(path, contents)
        .unwrap_or_else(|e| panic!("failed to write golden file {:?}: {}", path, e));
}

fn extract_output(contents: &str) -> String {
    if let Some(idx) = contents.find(OUTPUT_MARKER) {
        let after_marker = &contents[idx + OUTPUT_MARKER.len()..];
        let after_newline = after_marker
            .strip_prefix("\r\n")
            .or_else(|| after_marker.strip_prefix('\n'))
            .unwrap_or(after_marker);
        after_newline.to_string()
    } else {
        contents.to_string()
    }
}

// Keep normalization minimal; avoid changing semantic content.
// This only trims trailing whitespace on each line and ensures a trailing newline.
fn normalize_output(s: &str) -> String {
    let mut out = String::new();
    for line in s.lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

use std::fs;

use analyzer::{analyze, format_diagnostics};
use insta::assert_snapshot;

#[test]
fn diagnostics_snapshots() {
    insta::glob!("diagnostics", "*.formula", |path| {
        let source = fs::read_to_string(path).unwrap();
        let out = analyze(&source).unwrap();
        let rendered = format_diagnostics(&source, out.diagnostics);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        assert_snapshot!(name, rendered);
    });
}

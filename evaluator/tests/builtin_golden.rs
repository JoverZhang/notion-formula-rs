mod support;

use std::path::Path;

#[test]
fn builtin_goldens() {
    support::builtin_golden::run_builtin_goldens(Path::new("tests/builtins"));
}

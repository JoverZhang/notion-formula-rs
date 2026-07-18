use std::path::PathBuf;

use builtin_fn::{builtin_categories, render_builtin_readme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--check".to_string());
    if mode != "--check" && mode != "--write" {
        return Err(format!("expected `--check` or `--write`, got `{mode}`").into());
    }

    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../docs/builtin_functions/README.md");
    let current = std::fs::read_to_string(&path)?;
    let rendered = render_builtin_readme(&current, &builtin_categories())?;

    if mode == "--write" {
        if rendered != current {
            std::fs::write(path, rendered)?;
        }
        return Ok(());
    }

    if rendered != current {
        return Err(
            "docs/builtin_functions/README.md catalog is stale; run `cargo run -p builtin_fn --bin builtin_catalog -- --write`"
                .into(),
        );
    }
    Ok(())
}

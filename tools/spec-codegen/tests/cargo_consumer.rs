use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SPEC: &str = r#"<details>
<summary>Imports</summary>

~~~rust header=formula_engine
use crate::Schema;
~~~

</details>

~~~rust header=formula_engine
pub struct FormulaEngine {
    pub label: String,
}
~~~

~~~rust header=formula_engine
impl FormulaEngine {
    pub fn new(schema: Schema, label: String) -> Self;
    pub fn schema(&self) -> &Schema;
    pub fn bump(&mut self, by: usize);
    pub fn into_rows(self) -> usize;
    pub fn zero() -> usize;
}
~~~
"#;

const BUILD: &str = r#"fn main() {
    println!("cargo:rerun-if-changed=spec/api.md");
    spec_codegen::generate(
        &["spec/api.md"],
        std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"),
    ).expect("generate headers");
}
"#;

const MAIN: &str = r#"pub struct Schema {
    rows: usize,
}

mod engine {
    use spec_header::include_header;

    include_header!(formula_engine);

    struct FormulaEngineInner {
        schema: Schema,
    }

    impl FormulaEngine {
        fn new_impl(schema: Schema, label: String) -> Self {
            Self { label, inner: FormulaEngineInner { schema } }
        }

        #[cfg(not(feature = "wrong-return"))]
        fn schema_impl(&self) -> &Schema {
            &self.inner.schema
        }

        #[cfg(feature = "wrong-return")]
        fn schema_impl(&self) -> u64 { 0 }

        #[cfg(not(feature = "missing-hook"))]
        fn bump_impl(&mut self, by: usize) {
            self.inner.schema.rows += by;
        }

        fn into_rows_impl(self) -> usize {
            self.inner.schema.rows
        }

        fn zero_impl() -> usize { 0 }
    }
}

fn main() {
    let mut engine = engine::FormulaEngine::new(Schema { rows: 1 }, "Initial".into());
    assert_eq!(engine.label, "Initial");
    engine.label = "Renamed".into();
    assert_eq!(engine.label, "Renamed");
    assert_eq!(engine.schema().rows, 1);
    engine.bump(2);
    #[cfg(feature = "private-access")]
    let _ = &engine.inner;
    assert_eq!(engine.into_rows(), 3);
    assert_eq!(engine::FormulaEngine::zero(), 0);
}
"#;

fn cargo(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO"))
        .args(args)
        .arg("--offline")
        .current_dir(directory)
        .env("CARGO_TARGET_DIR", directory.join("target"))
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("run consumer Cargo command")
}

fn assert_success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn assert_failure(output: Output, expected: &str) {
    assert!(!output.status.success(), "consumer unexpectedly compiled");
    let diagnostics = String::from_utf8_lossy(&output.stderr);
    assert!(
        diagnostics.contains(expected),
        "expected {expected}\n{diagnostics}"
    );
}

#[test]
fn cargo_build_links_headers_rebuilds_and_exposes_only_the_support_dependency() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("spec")).unwrap();
    let generator = Path::new(env!("CARGO_MANIFEST_DIR"));
    let support = generator.join("header-support");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "header-consumer"
version = "0.1.0"
edition = "2024"
publish = false
include = ["src/**", "spec/api.md", "build.rs", "Cargo.toml"]

[workspace]

[dependencies]
spec_header = {{ path = {support:?}, version = "0.1.0" }}

[build-dependencies]
spec-codegen = {{ path = {generator:?}, version = "0.1.0" }}

[features]
missing-hook = []
wrong-return = []
private-access = []
"#,
            support = support.to_string_lossy(),
            generator = generator.to_string_lossy(),
        ),
    )
    .unwrap();
    fs::write(root.join("build.rs"), BUILD).unwrap();
    fs::write(root.join("src/main.rs"), MAIN).unwrap();
    let source = root.join("spec/api.md");
    fs::write(&source, SPEC).unwrap();
    assert_success(cargo(root, &["run", "--quiet"]));

    let tree = assert_success(cargo(root, &["tree", "--edges", "normal"]));
    let tree = String::from_utf8_lossy(&tree.stdout);
    assert!(tree.contains("spec_header"), "{tree}");
    assert!(
        !tree.contains("spec-codegen v") && !tree.contains("syn v"),
        "{tree}"
    );

    let package = assert_success(cargo(root, &["package", "--list", "--allow-dirty"]));
    assert!(String::from_utf8_lossy(&package.stdout).contains("spec/api.md"));

    for (feature, diagnostic) in [
        ("missing-hook", "bump_impl"),
        ("wrong-return", "E0308"),
        ("private-access", "E0616"),
    ] {
        assert_failure(cargo(root, &["check", "--features", feature]), diagnostic);
    }

    // The source is the only changed input: Cargo must regenerate the interface.
    fs::write(
        &source,
        SPEC.replace(
            "pub fn zero()",
            "pub fn new_requirement();\n    pub fn zero()",
        ),
    )
    .unwrap();
    assert_failure(cargo(root, &["run", "--quiet"]), "new_requirement_impl");

    // A renamed key must not leave the previous include usable from stale output.
    fs::write(
        &source,
        SPEC.replace("header=formula_engine", "header=renamed_engine"),
    )
    .unwrap();
    assert_failure(cargo(root, &["run", "--quiet"]), "couldn't read");
    fs::write(&source, SPEC).unwrap();
    assert_success(cargo(root, &["run", "--quiet"]));

    // Invalid generation input fails the build even though an earlier header exists.
    fs::write(
        &source,
        SPEC.replace("pub fn zero()", "pub async fn zero()"),
    )
    .unwrap();
    let failed = cargo(root, &["run", "--quiet"]);
    let diagnostics = String::from_utf8_lossy(&failed.stderr);
    assert!(diagnostics.contains("spec/api.md:"), "{diagnostics}");
    assert_failure(failed, "failed to run custom build command");
}

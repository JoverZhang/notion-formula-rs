use std::fs;
use std::path::PathBuf;
use std::process::Command;

use spec_codegen::generate;

struct Fixture {
    directory: tempfile::TempDir,
    source: PathBuf,
    output: PathBuf,
}

impl Fixture {
    fn new(markdown: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("api.md");
        fs::write(&source, markdown).unwrap();
        let output = directory.path().join("out");
        Self {
            directory,
            source,
            output,
        }
    }

    fn generate(&self) -> Result<Vec<PathBuf>, spec_codegen::Error> {
        generate(&[&self.source], &self.output)
    }

    fn header(&self, key: &str) -> String {
        fs::read_to_string(self.output.join(format!("{key}.h.rs"))).unwrap()
    }
}

fn marked(code: &str) -> String {
    format!("~~~rust header=engine\n{code}\n~~~\n")
}

#[test]
fn aggregates_before_classification_and_preserves_visibility() {
    let fixture = Fixture::new(
        r#"~~~rust header=engine
impl Engine {
    pub fn new(label: String) -> Self;
    pub fn value(&self) -> &u64;
}
~~~

~~~rust header=data
#[derive(Clone)]
pub struct Column<T> { pub values: Vec<T> }
pub enum Value { Number(u64), Empty }
pub type Columns<T> = Vec<Column<T>>;
~~~

~~~rust header=engine
/// User-facing engine.
pub struct Engine { pub label: String, pub(crate) enabled: bool }
~~~

~~~rust header=engine
impl Engine {
    pub fn add(&mut self, value: u64);
    pub fn into_value(self) -> u64;
    pub fn zero() -> u64;
}
~~~
"#,
    );
    let paths = fixture.generate().unwrap();
    assert_eq!(
        paths,
        [
            fixture.output.join("data.h.rs"),
            fixture.output.join("engine.h.rs")
        ]
    );
    let header = fixture.header("engine");
    for expected in [
        "pub label: String",
        "pub(crate) enabled: bool",
        "inner: EngineInner",
        "Self::new_impl(label)",
        "self.value_impl()",
        "self.add_impl(value)",
        "self.into_value_impl()",
        "Self::zero_impl()",
        "/// User-facing engine.",
        "// Source: api.md:",
        "// Method: api.md:",
    ] {
        assert!(header.contains(expected), "{expected}\n{header}");
    }
    assert!(header.find("impl Engine").unwrap() < header.find("pub struct Engine").unwrap());
    assert!(!header.contains(&fixture.directory.path().display().to_string()));
    assert!(!fixture.header("data").contains("Inner"));
    syn::parse_file(&header).unwrap();
    syn::parse_file(&fixture.header("data")).unwrap();
}

#[test]
fn extracts_details_blockquotes_lists_and_backtick_fences() {
    for markdown in [
        "<details>\n<summary>Imports</summary>\n\n~~~rust header=engine\nuse crate::Schema;\n~~~\n\n</details>\n",
        "> ~~~rust header=engine\n> use crate::Schema;\n> ~~~\n",
        "> > ~~~rust header=engine\n> > use crate::Schema;\n> > ~~~\n",
        "- ~~~rust header=engine\n  use crate::Schema;\n  ~~~\n",
    ] {
        for source in [markdown.to_owned(), markdown.replace('~', "\x60")] {
            let fixture = Fixture::new(&source);
            fixture.generate().unwrap();
            assert!(fixture.header("engine").contains("use crate::Schema;"));
        }
    }
}

#[test]
fn ignores_unmarked_and_nested_examples() {
    let fixture = Fixture::new(
        "~~~rust\nnot Rust\n~~~\n\n~~~~markdown\n~~~rust header=example\nstruct Example;\n~~~\n~~~~\n",
    );
    assert!(fixture.generate().unwrap().is_empty());
    assert!(!fixture.output.join("example.h.rs").exists());
}

#[test]
fn reports_markdown_line_inside_a_blockquote() {
    let fixture = Fixture::new(
        "# Example\n\n> ~~~rust header=engine\n> struct Engine {}\n> impl Engine {\n>     pub async fn run(&self);\n> }\n> ~~~\n",
    );
    let error = fixture.generate().unwrap_err().to_string();
    assert!(error.contains("api.md:6:"), "{error}");
}

#[test]
fn rejects_unclosed_fences_including_container_termination() {
    for source in [
        "~~~rust header=engine\nstruct Engine;",
        "~~~rust header=engine\n",
        "> ~~~rust header=engine\n> struct Engine;\n\nOutside\n",
        "- ~~~rust header=engine\n  struct Engine;\n\nOutside\n",
        "~~~~rust header=engine\nstruct Engine;\n~~~\n",
    ] {
        let error = Fixture::new(source).generate().unwrap_err().to_string();
        assert!(error.contains("explicitly closed"), "{error}");
    }
}

#[test]
fn rejects_invalid_metadata() {
    for info in [
        "ts header=engine",
        "rust header",
        "rust header=",
        "rust header=../engine",
        "rust header=engine.h.rs",
        "rust header=Engine",
        "rust header=engine__api",
        "rust header=engine header=other",
        "rust header=engine extra=yes",
    ] {
        let source = format!("~~~{info}\nstruct Engine;\n~~~\n");
        let error = Fixture::new(&source).generate().unwrap_err().to_string();
        assert!(error.contains("api.md:1:"), "{info}: {error}");
    }
}

#[test]
fn rejects_unsupported_syntax_duplicates_and_reserved_names() {
    for code in [
        "struct Engine {} struct Engine {}",
        "struct Engine {} impl Engine { fn x(); fn x(); }",
        "struct Engine {} impl Engine { fn x(); } impl Engine { fn x(); }",
        "struct Engine { inner: u64 } impl Engine { fn x(); }",
        "struct Engine {} struct EngineInner; impl Engine { fn x(); }",
        "use crate::State as EngineInner; struct Engine {} impl Engine { fn x(); }",
        "struct Engine {} impl Engine { fn x_impl(); }",
        "struct Engine {} impl Engine { fn x() {} }",
        "struct Engine {} impl Engine { fn x<T>(); }",
        "struct Engine<T> { x: T } impl Engine { fn x(); }",
        "struct Engine {} impl<T> Engine { fn x(); }",
        "struct Engine {} impl Clone for Engine {}",
        "struct Engine {} impl Engine { async fn x(); }",
        "struct Engine {} impl Engine { const fn x(); }",
        "struct Engine {} impl Engine { unsafe fn x(); }",
        "struct Engine {} impl Engine { extern \"C\" fn x(); }",
        "struct Engine {} impl Engine { fn x((a, b): (u64, u64)); }",
        "struct Engine {} impl Engine { fn x(_: u64); }",
        "struct Engine {} impl Engine { fn x(self: Box<Self>); }",
        "struct Engine {} impl Engine { #[cfg(test)] fn x(); }",
        "#[repr(C)] struct Engine { value: u64 }",
        "struct Engine { #[cfg(test)] value: u64 }",
        "struct Engine<#[cfg(test)] T> { value: T }",
        "enum Value { #[cfg(test)] Empty }",
        "impl Missing { fn x(); }",
        "struct Engine; impl Engine { fn x(); }",
        "fn free_function() {}",
        "const VALUE: u64 = 0;",
        "mod nested {}",
        "include!(\"elsewhere.rs\");",
    ] {
        let fixture = Fixture::new(&marked(code));
        let error = fixture.generate().expect_err(code).to_string();
        assert!(error.contains("api.md:"), "{code}: {error}");
        assert!(
            !fixture.output.exists(),
            "invalid syntax must not publish output"
        );
    }
}

#[test]
fn source_ownership_is_explicit_and_all_inputs_validate_before_writing() {
    let fixture = Fixture::new(&marked("struct Engine;"));
    let other = fixture.directory.path().join("translated.md");
    fs::write(&other, marked("struct Other;")).unwrap();
    let error = generate(&[&fixture.source, &other], &fixture.output)
        .unwrap_err()
        .to_string();
    assert!(error.contains("already belongs"), "{error}");
    assert!(!fixture.output.exists());
    assert!(generate(&[&fixture.source, &fixture.source], &fixture.output).is_err());
    assert!(generate::<PathBuf>(&[], &fixture.output).is_err());
}

#[test]
fn removes_only_owned_headers_when_keys_disappear() {
    let fixture = Fixture::new(&marked("struct Engine;"));
    fixture.generate().unwrap();
    let unrelated = fixture.output.join("other_build_output.rs");
    fs::write(&unrelated, "handwritten").unwrap();
    fs::write(&fixture.source, "No marked blocks remain.\n").unwrap();
    assert!(fixture.generate().unwrap().is_empty());
    assert!(!fixture.output.join("engine.h.rs").exists());
    assert_eq!(fs::read_to_string(unrelated).unwrap(), "handwritten");
    fs::write(
        &fixture.source,
        marked("struct Replacement;").replace("header=engine", "header=new_key"),
    )
    .unwrap();
    fixture.generate().unwrap();
    assert!(fixture.output.join("new_key.h.rs").exists());
}

#[test]
fn preserves_valid_outputs_when_new_input_is_invalid() {
    let fixture = Fixture::new(&marked("struct Engine;"));
    fixture.generate().unwrap();
    let previous = fixture.header("engine");
    fs::write(&fixture.source, marked("not valid Rust")).unwrap();
    assert!(fixture.generate().is_err()); // The caller must propagate this failure.
    assert_eq!(fixture.header("engine"), previous);
}

#[test]
fn output_is_deterministic_and_unchanged_files_keep_their_mtime() {
    let first = Fixture::new(&marked("pub struct Engine;"));
    let second = Fixture::new(&marked("pub struct Engine;"));
    let paths = first.generate().unwrap();
    second.generate().unwrap();
    let before = fs::metadata(&paths[0]).unwrap().modified().unwrap();
    first.generate().unwrap();
    assert_eq!(fs::metadata(&paths[0]).unwrap().modified().unwrap(), before);
    assert_eq!(first.header("engine"), second.header("engine"));
}

#[test]
fn refuses_unowned_files_and_invalid_ownership_manifests() {
    let fixture = Fixture::new(&marked("struct Engine;"));
    fs::create_dir(&fixture.output).unwrap();
    let target = fixture.output.join("engine.h.rs");
    fs::write(&target, "handwritten").unwrap();
    assert!(
        fixture
            .generate()
            .unwrap_err()
            .to_string()
            .contains("not owned")
    );
    assert_eq!(fs::read_to_string(&target).unwrap(), "handwritten");
    fs::write(
        fixture.output.join(".spec-codegen-manifest"),
        "spec-codegen-v1\n../outside.h.rs\n",
    )
    .unwrap();
    assert!(
        fixture
            .generate()
            .unwrap_err()
            .to_string()
            .contains("invalid header filename")
    );
}

#[cfg(unix)]
#[test]
fn never_follows_header_or_manifest_symlinks() {
    use std::os::unix::fs::symlink;

    for name in ["engine.h.rs", ".spec-codegen-manifest"] {
        let fixture = Fixture::new(&marked("struct Engine;"));
        fs::create_dir(&fixture.output).unwrap();
        let victim = fixture.directory.path().join("untouched");
        fs::write(&victim, "untouched").unwrap();
        symlink(&victim, fixture.output.join(name)).unwrap();
        assert!(
            fixture
                .generate()
                .unwrap_err()
                .to_string()
                .contains("symlink")
        );
        assert_eq!(fs::read_to_string(victim).unwrap(), "untouched");
    }
}

#[test]
fn cli_uses_the_library_and_returns_failures() {
    let fixture = Fixture::new(&marked("pub struct Engine;"));
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_spec-codegen"))
            .arg("--out-dir")
            .arg(&fixture.output)
            .arg(&fixture.source)
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    assert!(fixture.header("engine").contains("pub struct Engine;"));
    fs::write(&fixture.source, marked("invalid Rust")).unwrap();
    let failed = run();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("api.md:"));
    assert!(
        Command::new(env!("CARGO_BIN_EXE_spec-codegen"))
            .arg("--help")
            .output()
            .unwrap()
            .status
            .success()
    );
}

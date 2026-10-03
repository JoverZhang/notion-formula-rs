use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TemporaryOutput(PathBuf);

impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn compile_fixture(
    rustc: &OsString,
    dependencies: &Path,
    library: &Path,
    fixture: &Path,
    output: &Path,
) -> Output {
    Command::new(rustc)
        .arg("--edition=2024")
        .arg("--crate-name=draft_borrow_fixture")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("--cap-lints=allow")
        .arg("--extern")
        .arg(format!("formula_engine={}", library.display()))
        .arg("-L")
        .arg(format!("dependency={}", dependencies.display()))
        .arg("--out-dir")
        .arg(output)
        .arg(fixture)
        .output()
        .expect("rustc must be available to compile the public borrow fixtures")
}

#[test]
fn public_drafts_share_an_engine_borrow_and_cannot_escape_or_mutate_it() {
    // Use this test executable's dependency directory so custom Cargo target dirs
    // and debug/release profiles use the same library the test was built against.
    let executable = std::env::current_exe().unwrap();
    let dependencies = executable.parent().unwrap();
    let library = fs::read_dir(dependencies)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("libformula_engine-") && name.ends_with(".rlib")
        })
        .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap())
        .expect("Cargo must build the formula_engine library beside the test")
        .path();
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output = TemporaryOutput(std::env::temp_dir().join(format!(
        "formula-draft-borrow-{}-{suffix}",
        std::process::id()
    )));
    fs::create_dir(&output.0).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/draft_borrow");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());

    // A successful fixture checks that the compiler/linking harness works before
    // the failures are interpreted as evidence of the public lifetime contract.
    let pass = compile_fixture(
        &rustc,
        dependencies,
        &library,
        &fixtures.join("multiple.rs"),
        &output.0,
    );
    assert!(
        pass.status.success(),
        "multiple shared Drafts must compile:\n{}",
        String::from_utf8_lossy(&pass.stderr)
    );

    for (file, code) in [
        ("upsert.rs", "E0502"),
        ("remove.rs", "E0502"),
        ("escape.rs", "E0515"),
    ] {
        let result = compile_fixture(
            &rustc,
            dependencies,
            &library,
            &fixtures.join(file),
            &output.0,
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{file} must fail to compile");
        assert!(
            stderr.contains(&format!("error[{code}]")),
            "{file} must fail for the borrow contract ({code}), got:\n{stderr}"
        );
    }
}

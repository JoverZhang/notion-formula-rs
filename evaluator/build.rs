#[path = "build_support.rs"]
mod build_support;

fn main() {
    println!("cargo:rerun-if-changed=../builtin_fn/src");
    println!("cargo:rerun-if-changed=build_support.rs");

    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
        .join("builtin_contract.rs");
    let generated = build_support::generate_contract(&builtin_fn::builtin_categories());
    std::fs::write(output, generated).expect("write generated builtin contract");
}

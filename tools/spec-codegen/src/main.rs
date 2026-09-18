use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: spec-codegen --out-dir <directory> <source.md>...";

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else {
        return Err(USAGE.into());
    };
    if first == "--help" || first == "-h" {
        println!("{USAGE}");
        return Ok(());
    }
    if first != "--out-dir" {
        return Err(USAGE.into());
    }
    let directory = args.next().ok_or(USAGE)?;
    let sources: Vec<PathBuf> = args.map(PathBuf::from).collect();
    spec_codegen::generate(&sources, directory).map_err(|error| error.to_string())?;
    Ok(())
}

fn main() -> ExitCode {
    // Generate one complete set of headers; failures must stop the calling build.
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("spec-codegen: {error}");
            ExitCode::FAILURE
        }
    }
}

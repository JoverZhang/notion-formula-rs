use std::path::PathBuf;

use md_first_core::Dispatcher;
use md_first_preprocessor::rust_header::RustHeader;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = None;
    let mut check = false;
    for argument in std::env::args_os().skip(1) {
        if argument == "--check" && !check {
            check = true;
        } else if argument.to_string_lossy().starts_with('-') || root.is_some() {
            return Err("usage: md-first [--check] [project-root]".into());
        } else {
            root = Some(PathBuf::from(argument));
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    // This project-side entry point chooses the adapters; Dispatcher has none
    // built in. Consumers register their own suffixes through the same interface.
    let mut dispatcher = Dispatcher::new();
    dispatcher.register(".h.rs", RustHeader)?;
    if check {
        dispatcher.check(root)?;
        println!("Generated files are up to date.");
    } else {
        for path in dispatcher.generate(root)? {
            println!("{}", path.display());
        }
    }
    Ok(())
}

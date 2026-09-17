#![no_std]

//! Compile-time inclusion only; generation belongs in the consumer's build script.

/// Include a named header from the consuming package's Cargo output directory.
///
/// Invoke once per module. The included declarations and handwritten implementation
/// share the same module, including access to private fields and methods.
#[macro_export]
macro_rules! include_header {
    ($key:ident) => {
        ::core::include!(::core::concat!(
            ::core::env!("OUT_DIR"),
            "/",
            ::core::stringify!($key),
            ".h.rs"
        ));
    };
}

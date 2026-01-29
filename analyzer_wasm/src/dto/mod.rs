//! Versioned DTO surface for WASM exports.
//!
//! All exported views are designed for the JS/editor boundary:
//! - Offsets and spans are in **UTF-16 code units**.
//! - All ranges are **half-open** `[start, end)` (inclusive start, exclusive end).
//!
//! **Entry points**
//! - [`v1`]: current stable DTO version used by the WASM exports.
pub mod v1;

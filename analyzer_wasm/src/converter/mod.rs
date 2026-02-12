//! Conversion utilities for the WASM/JS boundary.
//!
//! This module is intentionally stateless and centralizes:
//! - Input conversion (e.g. parsing context JSON).
//! - UTF-16 ↔ byte offset bridging for editor-facing positions.
//! - DTO conversion (internal analyzer types → `dto::v1::*`).

mod analyze;
mod completion;
mod context;
mod shared;

pub struct Converter;

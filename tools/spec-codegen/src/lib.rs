//! Generate Rust interface headers from explicitly selected Markdown documents.
//!
//! Call generate once with all canonical sources for an output directory. In Cargo,
//! the caller selects OUT_DIR, registers rerun-if-changed for every source, and
//! propagates any error to fail the build. Translations are not additional inputs.

mod markdown;
mod output;
mod syntax;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A source-located declaration error or a contextual filesystem error.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    fn io(path: &Path, error: std::io::Error) -> Self {
        Self(format!("{}: {error}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Generate all headers, returning their paths in key order.
///
/// All inputs are parsed before writing. Only files owned by the output manifest
/// may be replaced or removed. Unmarked documents are valid: removing the last
/// marked block removes its obsolete header on the next successful invocation.
/// Concurrent invocations must use different output directories.
pub fn generate<P: AsRef<Path>>(
    sources: &[P],
    output_dir: impl AsRef<Path>,
) -> Result<Vec<PathBuf>, Error> {
    if sources.is_empty() {
        return Err(Error::new(
            "at least one canonical Markdown source is required",
        ));
    }
    let mut seen_sources = BTreeSet::new();
    let mut groups: BTreeMap<String, Vec<markdown::Block>> = BTreeMap::new();
    for source in sources {
        let path = source.as_ref();
        let canonical = fs::canonicalize(path).map_err(|error| Error::io(path, error))?;
        if !seen_sources.insert(canonical) {
            return Err(Error::new(format!(
                "{}: duplicate input document",
                path.display()
            )));
        }
        let text = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
        for block in markdown::extract(path, &text)? {
            let group = groups.entry(block.key.clone()).or_default();
            if let Some(owner) = group.first()
                && owner.path != block.path
            {
                return Err(block.error(
                    block.line,
                    format!(
                        "header {} already belongs to {}",
                        block.key,
                        owner.path.display()
                    ),
                ));
            }
            group.push(block);
        }
    }
    let headers = groups
        .into_iter()
        .map(|(key, blocks)| syntax::render(&blocks).map(|code| (key, code)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    output::write(output_dir.as_ref(), &headers)
}

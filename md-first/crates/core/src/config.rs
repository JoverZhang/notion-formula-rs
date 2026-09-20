use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use glob::{MatchOptions, Pattern};

use crate::Error;

pub(crate) fn sources(root: &Path) -> Result<BTreeSet<PathBuf>, Error> {
    let manifest = root.join("Cargo.toml");
    let source = fs::read_to_string(&manifest).map_err(|error| Error::io(&manifest, error))?;
    let value: toml::Value = toml::from_str(&source)
        .map_err(|error| Error::new(format!("{}: {error}", manifest.display())))?;
    let config = value
        .get("workspace")
        .and_then(|value| value.get("metadata"))
        .and_then(|value| value.get("md-first"))
        .ok_or_else(|| Error::new("Cargo.toml: missing [workspace.metadata.md-first]"))?;
    let include = patterns(config, "include", true)?;
    let exclude = patterns(config, "exclude", false)?;
    let options = MatchOptions {
        require_literal_separator: true,
        ..MatchOptions::new()
    };
    let root_pattern = Pattern::escape(&root.to_string_lossy());
    let mut sources = BTreeSet::new();
    for pattern in include {
        let absolute = format!("{root_pattern}/{pattern}");
        let entries = glob::glob_with(&absolute, options)
            .map_err(|error| Error::new(format!("Cargo.toml: {error}")))?;
        for entry in entries {
            let path = entry.map_err(|error| Error::new(error.to_string()))?;
            if !path.is_file() || path.extension().is_none_or(|extension| extension != "md") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|error| Error::new(error.to_string()))?;
            if exclude
                .iter()
                .any(|pattern| pattern.matches_path_with(relative, options))
            {
                continue;
            }
            let canonical = fs::canonicalize(&path).map_err(|error| Error::io(&path, error))?;
            if !canonical.starts_with(root) {
                return Err(Error::new(format!(
                    "{}: source escapes project root",
                    relative.display()
                )));
            }
            sources.insert(relative.to_owned());
        }
    }
    Ok(sources)
}

fn patterns(config: &toml::Value, key: &str, required: bool) -> Result<Vec<Pattern>, Error> {
    let Some(value) = config.get(key) else {
        return if required {
            Err(Error::new(format!(
                "Cargo.toml: md-first.{key} is required"
            )))
        } else {
            Ok(Vec::new())
        };
    };
    let values = value
        .as_array()
        .ok_or_else(|| Error::new(format!("Cargo.toml: md-first.{key} must be an array")))?;
    values
        .iter()
        .map(|value| {
            let pattern = value.as_str().ok_or_else(|| {
                Error::new(format!("Cargo.toml: md-first.{key} must contain strings"))
            })?;
            relative_path(pattern)
                .map_err(|error| Error::new(format!("Cargo.toml: md-first.{key}: {error}")))?;
            Pattern::new(pattern)
                .map_err(|error| Error::new(format!("Cargo.toml: md-first.{key}: {error}")))
        })
        .collect()
}

pub(crate) fn relative_path(value: &str) -> Result<PathBuf, &'static str> {
    let path = Path::new(value);
    if value.is_empty() || value.contains(['\\', '\n', '\r']) || path.is_absolute() {
        return Err("expected a project-relative path using / separators");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir if normalized.pop() => {}
            _ => return Err("path escapes project root"),
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err("expected a file path");
    }
    Ok(normalized)
}

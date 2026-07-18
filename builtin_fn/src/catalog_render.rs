//! Deterministic presentation of the declaration catalog.

use std::fmt;

use crate::{BuiltinCategory, FunctionCategory};

pub const CATALOG_BEGIN_MARKER: &str = "<!-- BEGIN GENERATED BUILTIN CATALOG -->";
pub const CATALOG_END_MARKER: &str = "<!-- END GENERATED BUILTIN CATALOG -->";

/// Render only the generated builtin inventory, including category headings.
pub fn render_builtin_catalog(categories: &[BuiltinCategory]) -> String {
    let mut output = String::new();
    for (category_index, category) in categories.iter().enumerate() {
        if category_index > 0 {
            output.push('\n');
        }
        output.push_str("## ");
        output.push_str(category_title(category.category));
        output.push_str(" (");
        output.push_str(&category.entries.len().to_string());
        output.push_str(")\n\n```rust\n");

        for entry in &category.entries {
            if !entry.is_supported() {
                for doc in &entry.docs {
                    output.push_str("// Unsupported: ");
                    output.push_str(doc);
                    output.push('\n');
                }
            }
            output.push_str(&entry.signature);
            output.push('\n');
        }
        output.push_str("```\n");
    }
    output
}

/// Replace exactly the marked catalog region while preserving hand-maintained prose.
pub fn render_builtin_readme(
    current: &str,
    categories: &[BuiltinCategory],
) -> Result<String, CatalogRegionError> {
    let mut begin_markers = current.match_indices(CATALOG_BEGIN_MARKER);
    let begin = begin_markers
        .next()
        .map(|(index, _)| index)
        .ok_or(CatalogRegionError::MissingBeginMarker)?;
    if begin_markers.next().is_some() {
        return Err(CatalogRegionError::DuplicateBeginMarker);
    }

    let mut end_markers = current.match_indices(CATALOG_END_MARKER);
    let end_marker_start = end_markers
        .next()
        .map(|(index, _)| index)
        .ok_or(CatalogRegionError::MissingEndMarker)?;
    if end_markers.next().is_some() {
        return Err(CatalogRegionError::DuplicateEndMarker);
    }

    let content_start = begin + CATALOG_BEGIN_MARKER.len();
    if end_marker_start < content_start {
        return Err(CatalogRegionError::EndMarkerBeforeBeginMarker);
    }
    let end = end_marker_start + CATALOG_END_MARKER.len();

    let mut output = String::with_capacity(current.len());
    output.push_str(&current[..begin]);
    output.push_str(CATALOG_BEGIN_MARKER);
    output.push_str("\n\n");
    output.push_str(&render_builtin_catalog(categories));
    output.push_str(CATALOG_END_MARKER);
    output.push_str(&current[end..]);
    Ok(output)
}

fn category_title(category: FunctionCategory) -> &'static str {
    match category {
        FunctionCategory::General => "General",
        FunctionCategory::Text => "Text",
        FunctionCategory::Number => "Number",
        FunctionCategory::Date => "Date",
        FunctionCategory::People => "People",
        FunctionCategory::List => "List",
        FunctionCategory::Special => "Special",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogRegionError {
    MissingBeginMarker,
    MissingEndMarker,
    DuplicateBeginMarker,
    DuplicateEndMarker,
    EndMarkerBeforeBeginMarker,
}

impl fmt::Display for CatalogRegionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBeginMarker => write!(
                formatter,
                "builtin README is missing `{CATALOG_BEGIN_MARKER}`"
            ),
            Self::MissingEndMarker => write!(
                formatter,
                "builtin README is missing `{CATALOG_END_MARKER}`"
            ),
            Self::DuplicateBeginMarker => write!(
                formatter,
                "builtin README contains more than one `{CATALOG_BEGIN_MARKER}`"
            ),
            Self::DuplicateEndMarker => write!(
                formatter,
                "builtin README contains more than one `{CATALOG_END_MARKER}`"
            ),
            Self::EndMarkerBeforeBeginMarker => write!(
                formatter,
                "builtin README contains `{CATALOG_END_MARKER}` before `{CATALOG_BEGIN_MARKER}`"
            ),
        }
    }
}

impl std::error::Error for CatalogRegionError {}

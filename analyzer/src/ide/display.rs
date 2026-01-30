//! Shared display formatting helpers for IDE/UI surfaces.
//!
//! This module is intentionally small and deterministic. It is the single
//! canonical place for formatting analyzer types into UI strings.

use crate::semantic::Ty;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeFormatOptions {
    /// When true, formats `List(Union(...))` as `(A | B)[]` to preserve precedence.
    pub paren_union_in_list: bool,
}

impl Default for TypeFormatOptions {
    fn default() -> Self {
        Self {
            paren_union_in_list: true,
        }
    }
}

/// Formats a type for UI display (e.g. `number[]` or `number | string`).
pub fn format_ty(ty: &Ty, opts: &TypeFormatOptions) -> String {
    match ty {
        Ty::Number => "number".into(),
        Ty::String => "string".into(),
        Ty::Boolean => "boolean".into(),
        Ty::Date => "date".into(),
        Ty::Null => "null".into(),
        Ty::Unknown => "unknown".into(),
        Ty::Generic(id) => format!("T{}", id.0),
        Ty::List(inner) => {
            let inner = &**inner;
            if opts.paren_union_in_list && matches!(inner, Ty::Union(_)) {
                format!("({})[]", format_ty(inner, opts))
            } else {
                format!("{}[]", format_ty(inner, opts))
            }
        }
        Ty::Union(members) => members
            .iter()
            .map(|m| format_ty(m, opts))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum DisplaySegmentKind {
    Name,
    Punct,
    ParamName,
    Type,
    Separator,
    Ellipsis,
    Arrow,
    ReturnType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplaySegment {
    pub kind: DisplaySegmentKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedParam {
    Param {
        name: String,
        ty: String,
        param_index: Option<u32>,
    },
    Ellipsis,
}

/// Build signature display segments for a single signature.
///
/// Output shape: `name(p1: ty1, p2: ty2, ...) -> ret`, but with punctuation split into segments.
pub fn build_signature_segments(
    func_name: &str,
    params: &[RenderedParam],
    ret: &Ty,
    opts: &TypeFormatOptions,
) -> Vec<DisplaySegment> {
    let mut segments = Vec::<DisplaySegment>::with_capacity(4 + params.len() * 4);

    segments.push(DisplaySegment {
        kind: DisplaySegmentKind::Name,
        text: func_name.to_string(),
        param_index: None,
    });
    segments.push(DisplaySegment {
        kind: DisplaySegmentKind::Punct,
        text: "(".to_string(),
        param_index: None,
    });

    for (idx, param) in params.iter().enumerate() {
        if idx > 0 {
            segments.push(DisplaySegment {
                kind: DisplaySegmentKind::Separator,
                text: ", ".to_string(),
                param_index: None,
            });
        }

        match param {
            RenderedParam::Ellipsis => segments.push(DisplaySegment {
                kind: DisplaySegmentKind::Ellipsis,
                text: "...".to_string(),
                param_index: None,
            }),
            RenderedParam::Param {
                name,
                ty,
                param_index,
            } => {
                segments.push(DisplaySegment {
                    kind: DisplaySegmentKind::ParamName,
                    text: name.clone(),
                    param_index: *param_index,
                });
                segments.push(DisplaySegment {
                    kind: DisplaySegmentKind::Punct,
                    text: ": ".to_string(),
                    param_index: *param_index,
                });
                segments.push(DisplaySegment {
                    kind: DisplaySegmentKind::Type,
                    text: ty.clone(),
                    param_index: *param_index,
                });
            }
        }
    }

    segments.push(DisplaySegment {
        kind: DisplaySegmentKind::Punct,
        text: ")".to_string(),
        param_index: None,
    });
    segments.push(DisplaySegment {
        kind: DisplaySegmentKind::Arrow,
        text: " -> ".to_string(),
        param_index: None,
    });
    segments.push(DisplaySegment {
        kind: DisplaySegmentKind::ReturnType,
        text: format_ty(ret, opts),
        param_index: None,
    });

    segments
}

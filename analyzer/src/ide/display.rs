//! Shared display formatting helpers for IDE/UI surfaces.
//!
//! This module is intentionally small and deterministic. It is the single
//! canonical place for formatting UI-facing signature help segments.

use crate::semantic::Ty;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
pub enum DisplaySegment {
    Name { text: String },
    Punct { text: String },
    Separator { text: String },
    Ellipsis,
    Arrow { text: String },
    Param {
        name: String,
        ty: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        param_index: Option<u32>,
    },
    ReturnType { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamSlot {
    Param {
        name: String,
        ty: String,
        param_index: u32,
    },
    Ellipsis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSignature {
    pub receiver: Option<(String, String)>,
    pub slots: Vec<ParamSlot>,
}

/// Builds signature segments for a single signature candidate.
///
/// Output shape: `name(p1: ty1, p2: ty2, ...) -> ret`, with punctuation split into segments.
pub fn build_signature_segments(
    func_name: &str,
    rendered: &RenderedSignature,
    ret: &Ty,
    is_method_style: bool,
) -> Vec<DisplaySegment> {
    let mut out = Vec::<DisplaySegment>::new();

    if is_method_style {
        if let Some((name, ty)) = &rendered.receiver {
            out.push(DisplaySegment::Punct {
                text: "(".to_string(),
            });
            out.push(DisplaySegment::Param {
                name: name.clone(),
                ty: ty.clone(),
                param_index: None,
            });
            out.push(DisplaySegment::Punct {
                text: ")".to_string(),
            });
            out.push(DisplaySegment::Punct {
                text: ".".to_string(),
            });
        }
    }

    out.push(DisplaySegment::Name {
        text: func_name.to_string(),
    });
    out.push(DisplaySegment::Punct {
        text: "(".to_string(),
    });

    for (idx, slot) in rendered.slots.iter().enumerate() {
        if idx > 0 {
            out.push(DisplaySegment::Separator {
                text: ", ".to_string(),
            });
        }
        match slot {
            ParamSlot::Ellipsis => out.push(DisplaySegment::Ellipsis),
            ParamSlot::Param {
                name,
                ty,
                param_index,
            } => out.push(DisplaySegment::Param {
                name: name.clone(),
                ty: ty.clone(),
                param_index: Some(*param_index),
            }),
        }
    }

    out.push(DisplaySegment::Punct {
        text: ")".to_string(),
    });
    out.push(DisplaySegment::Arrow {
        text: " -> ".to_string(),
    });
    out.push(DisplaySegment::ReturnType {
        text: ret.to_string(),
    });

    out
}

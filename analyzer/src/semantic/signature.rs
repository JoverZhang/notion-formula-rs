use super::{FunctionCategory, GenericId, Ty};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GenericParamKind {
    Plain,
    Variant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericParam {
    pub id: GenericId,
    pub name: String,
    pub kind: GenericParamKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSig {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ParamLayout {
    Flat(Vec<ParamSig>),
    RepeatGroup {
        head: Vec<ParamSig>,
        repeat: Vec<ParamSig>,
        tail: Vec<ParamSig>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSig {
    pub name: String,
    pub layout: ParamLayout,
    pub ret: Ty,
    pub detail: Option<String>,
    pub category: FunctionCategory,
    pub generics: Vec<GenericParam>,
}

impl FunctionSig {
    pub fn flat_params(&self) -> Option<&[ParamSig]> {
        match &self.layout {
            ParamLayout::Flat(params) => Some(params),
            ParamLayout::RepeatGroup { .. } => None,
        }
    }

    pub fn display_params_len(&self) -> usize {
        match &self.layout {
            ParamLayout::Flat(params) => params.len(),
            ParamLayout::RepeatGroup { head, repeat, tail } => {
                head.len() + repeat.len() + tail.len()
            }
        }
    }

    pub fn display_params(&self) -> Vec<&ParamSig> {
        match &self.layout {
            ParamLayout::Flat(params) => params.iter().collect(),
            ParamLayout::RepeatGroup { head, repeat, tail } => head
                .iter()
                .chain(repeat.iter())
                .chain(tail.iter())
                .collect(),
        }
    }

    pub fn is_variadic(&self) -> bool {
        match &self.layout {
            ParamLayout::Flat(params) => params.last().is_some_and(|p| p.variadic),
            ParamLayout::RepeatGroup { .. } => true,
        }
    }

    pub fn required_min_args(&self) -> usize {
        match &self.layout {
            ParamLayout::Flat(params) => {
                let mut required = 0usize;
                for (idx, p) in params.iter().enumerate() {
                    if p.variadic {
                        if !p.optional {
                            required += 1;
                        }
                        // Variadic always ends the list.
                        break;
                    }
                    if !p.optional {
                        required = idx + 1;
                    }
                }
                required
            }
            ParamLayout::RepeatGroup { head, repeat, tail } => {
                // By design, RepeatGroup layouts represent "at least one repeat" plus the fixed
                // head/tail.
                let head_required = head.iter().take_while(|p| !p.optional).count();
                let tail_required = tail.iter().take_while(|p| !p.optional).count();
                head_required + repeat.len() + tail_required
            }
        }
    }

    pub fn param_for_arg_index(&self, idx: usize) -> Option<&ParamSig> {
        match &self.layout {
            ParamLayout::Flat(params) => {
                if idx < params.len() {
                    return params.get(idx);
                }
                if self.is_variadic() {
                    return params.last();
                }
                None
            }
            ParamLayout::RepeatGroup {
                head,
                repeat,
                tail: _,
            } => {
                // Best-effort mapping without knowing total arg count (completion/sighelp).
                if idx < head.len() {
                    return head.get(idx);
                }
                let idx = idx.saturating_sub(head.len());
                if repeat.is_empty() {
                    return None;
                }
                repeat.get(idx % repeat.len())
            }
        }
    }
}

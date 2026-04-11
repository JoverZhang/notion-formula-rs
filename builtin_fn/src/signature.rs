//! Function signature model used by semantic analysis and editor tooling.

use crate::{FunctionCategory, GenericId, Ty, resolve_repeat_tail_used};
use std::collections::HashSet;

/// How a generic parameter binds during inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericParamKind {
    Plain,
    Variant,
}

/// Declaration of a generic parameter used by a [`FunctionSig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParam {
    pub id: GenericId,
    pub kind: GenericParamKind,
}

/// A single parameter slot in a function signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSig {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
}

/// Parameter shape for a signature: `head`, optional repeating `repeat` group, and `tail`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamShape {
    pub head: Vec<ParamSig>,
    pub repeat: Vec<ParamSig>,
    pub tail: Vec<ParamSig>,
    pub repeat_min_groups: usize,
}

impl ParamShape {
    pub fn new(head: Vec<ParamSig>, repeat: Vec<ParamSig>, tail: Vec<ParamSig>) -> Self {
        if let Some(param) = repeat.iter().find(|p| p.optional) {
            panic!(
                "ParamShape invariant violated: repeat params must not be optional (found: {:?})",
                param
            );
        }

        if !repeat.is_empty()
            && !tail.is_empty()
            && let Some(param) = tail.iter().find(|p| p.optional)
        {
            panic!(
                "ParamShape invariant violated: when repeat params exist, tail params must be required for determinism (found optional: {:?})",
                param
            );
        }

        let mut seen_optional = false;
        for param in &tail {
            if seen_optional && !param.optional {
                panic!(
                    "ParamShape invariant violated: tail params must be suffix-only optional; found required param after optional: {:?}",
                    param
                );
            }
            if param.optional {
                seen_optional = true;
            }
        }

        Self {
            head,
            repeat,
            tail,
            repeat_min_groups: 1,
        }
    }

    pub fn resolve_params(&self, total: usize) -> Vec<&ParamSig> {
        let tail_used = resolve_repeat_tail_used(self, total);

        let mut out = Vec::with_capacity(total);
        let head_len = self.head.len();
        let tail_used = tail_used.unwrap_or(self.tail.len());
        let tail_start = total.saturating_sub(tail_used);

        for idx in 0..total {
            let param = if idx < head_len {
                self.head.get(idx)
            } else if idx >= tail_start && tail_used > 0 {
                self.tail.get(idx - tail_start)
            } else if !self.repeat.is_empty() {
                let repeat_idx = (idx - head_len) % self.repeat.len();
                self.repeat.get(repeat_idx)
            } else {
                self.tail.get(idx.saturating_sub(head_len))
            };

            if let Some(param) = param {
                out.push(param);
            }
        }

        out
    }

    pub fn with_repeat_min_groups(mut self, min: usize) -> Self {
        self.repeat_min_groups = min;
        self
    }
}

/// Custom type resolution function for builtins whose return type cannot be expressed by the
/// standard generic unification system.
pub type SigResolver = fn(&FunctionSig, &[Ty]) -> FunctionSig;

/// A function signature used for semantic validation and editor tooling.
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: ParamShape,
    pub ret: Ty,
    pub category: FunctionCategory,
    pub detail: String,
    pub generics: Vec<GenericParam>,
    pub resolver: Option<SigResolver>,
}

impl PartialEq for FunctionSig {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.ret == other.ret
            && self.category == other.category
            && self.detail == other.detail
            && self.generics == other.generics
    }
}

impl Eq for FunctionSig {}

impl FunctionSig {
    pub fn new(
        category: FunctionCategory,
        detail: impl Into<String>,
        name: impl Into<String>,
        params: ParamShape,
        ret: Ty,
        generics: Vec<GenericParam>,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            ret,
            detail: detail.into(),
            category,
            generics,
            resolver: None,
        }
    }

    pub fn new_builtin(
        category: FunctionCategory,
        detail: impl Into<String>,
        name: impl Into<String>,
        params: ParamShape,
        ret: Ty,
        generics: Vec<GenericParam>,
    ) -> Self {
        let sig = Self::new(category, detail, name, params, ret, generics);
        sig.validate_builtin();
        sig
    }

    pub fn new_builtin_with_resolver(
        category: FunctionCategory,
        detail: impl Into<String>,
        name: impl Into<String>,
        params: ParamShape,
        ret: Ty,
        generics: Vec<GenericParam>,
        resolver: SigResolver,
    ) -> Self {
        let mut sig = Self::new(category, detail, name, params, ret, generics);
        sig.resolver = Some(resolver);
        sig.validate_builtin();
        sig
    }

    fn validate_builtin(&self) {
        let mut declared = HashSet::<GenericId>::new();
        for generic in &self.generics {
            declared.insert(generic.id);
        }

        for param in self.display_params() {
            if let Some(ty) = find_unknown_in_ty(&param.ty) {
                panic!(
                    "Builtin FunctionSig `{}`: expected param `{}` type must not contain Ty::Unknown (found: {:?})",
                    self.name, param.name, ty
                );
            }
            for used in collect_generics_in_ty(&param.ty) {
                if !declared.contains(&used) {
                    panic!(
                        "Builtin FunctionSig `{}`: param `{}` type uses generic {:?} but it is not declared in `generics`",
                        self.name, param.name, used
                    );
                }
            }
        }

        if let Some(ty) = find_unknown_in_ty(&self.ret) {
            panic!(
                "Builtin FunctionSig `{}`: expected return type must not contain Ty::Unknown (found: {:?})",
                self.name, ty
            );
        }
        for used in collect_generics_in_ty(&self.ret) {
            if !declared.contains(&used) {
                panic!(
                    "Builtin FunctionSig `{}`: return type uses generic {:?} but it is not declared in `generics`",
                    self.name, used
                );
            }
        }
    }

    pub fn flat_params(&self) -> Option<&[ParamSig]> {
        if self.params.repeat.is_empty() && self.params.tail.is_empty() {
            return Some(&self.params.head);
        }
        None
    }

    pub fn display_params_len(&self) -> usize {
        self.params.head.len() + self.params.repeat.len() + self.params.tail.len()
    }

    pub fn display_params(&self) -> Vec<&ParamSig> {
        self.params
            .head
            .iter()
            .chain(self.params.repeat.iter())
            .chain(self.params.tail.iter())
            .collect()
    }

    pub fn is_variadic(&self) -> bool {
        !self.params.repeat.is_empty()
    }

    pub fn required_min_args(&self) -> usize {
        if self.params.repeat.is_empty() {
            let mut required = 0usize;
            for (idx, param) in self
                .params
                .head
                .iter()
                .chain(self.params.tail.iter())
                .enumerate()
            {
                if !param.optional {
                    required = idx + 1;
                }
            }
            return required;
        }

        let head_required = self.params.head.iter().filter(|p| !p.optional).count();
        let tail_required = self.params.tail.iter().filter(|p| !p.optional).count();
        head_required + self.params.repeat.len() * self.params.repeat_min_groups + tail_required
    }

    pub fn param_for_arg_index(&self, idx: usize) -> Option<&ParamSig> {
        if self.params.repeat.is_empty() {
            if idx < self.params.head.len() {
                return self.params.head.get(idx);
            }
            return self.params.tail.get(idx - self.params.head.len());
        }

        if idx < self.params.head.len() {
            return self.params.head.get(idx);
        }
        let idx = idx.saturating_sub(self.params.head.len());
        self.params.repeat.get(idx % self.params.repeat.len())
    }
}

fn collect_generics_in_ty(ty: &Ty) -> Vec<GenericId> {
    fn walk(ty: &Ty, out: &mut Vec<GenericId>) {
        match ty {
            Ty::Generic(generic) => out.push(*generic),
            Ty::List(inner) => walk(inner, out),
            Ty::Union(members) => {
                for member in members {
                    walk(member, out);
                }
            }
            Ty::Number | Ty::String | Ty::Boolean | Ty::Date | Ty::Null | Ty::Unknown => {}
            Ty::Fn { params, ret } => {
                for (_, param_ty) in params {
                    walk(param_ty, out);
                }
                walk(ret, out);
            }
            Ty::Ident(inner) => walk(inner, out),
        }
    }

    let mut out = Vec::new();
    walk(ty, &mut out);
    out
}

fn find_unknown_in_ty(ty: &Ty) -> Option<&Ty> {
    match ty {
        Ty::Unknown => Some(ty),
        Ty::List(inner) => find_unknown_in_ty(inner),
        Ty::Union(members) => members.iter().find_map(find_unknown_in_ty),
        Ty::Number | Ty::String | Ty::Boolean | Ty::Date | Ty::Null | Ty::Generic(_) => None,
        Ty::Fn { params, ret } => params
            .iter()
            .find_map(|(_, param_ty)| find_unknown_in_ty(param_ty))
            .or_else(|| find_unknown_in_ty(ret)),
        Ty::Ident(inner) => find_unknown_in_ty(inner),
    }
}

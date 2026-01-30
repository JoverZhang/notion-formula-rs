use super::{FunctionCategory, GenericId, Ty};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GenericParamKind {
    Plain,
    Variant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericParam {
    pub id: GenericId,
    pub kind: GenericParamKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSig {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamShape {
    pub head: Vec<ParamSig>,
    pub repeat: Vec<ParamSig>,
    pub tail: Vec<ParamSig>,
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
        for p in &tail {
            if seen_optional && !p.optional {
                panic!(
                    "ParamShape invariant violated: tail params must be suffix-only optional; found required param after optional: {:?}",
                    p
                );
            }
            if p.optional {
                seen_optional = true;
            }
        }

        Self { head, repeat, tail }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSig {
    pub name: String,
    pub params: ParamShape,
    pub ret: Ty,
    pub category: FunctionCategory,
    pub detail: String,
    pub generics: Vec<GenericParam>,
}

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

    fn validate_builtin(&self) {
        let mut declared = HashSet::<GenericId>::new();
        for g in &self.generics {
            declared.insert(g.id);
        }

        for p in self.display_params() {
            if let Some(ty) = find_unknown_in_ty(&p.ty) {
                panic!(
                    "Builtin FunctionSig `{}`: expected param `{}` type must not contain Ty::Unknown (found: {:?})",
                    self.name, p.name, ty
                );
            }
            for used in collect_generics_in_ty(&p.ty) {
                if !declared.contains(&used) {
                    panic!(
                        "Builtin FunctionSig `{}`: param `{}` type uses generic {:?} but it is not declared in `generics`",
                        self.name, p.name, used
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
            // Fixed-arity signature (no repeat group): required min is the last required param
            // index + 1 across the whole list.
            //
            // This is defensive even if a signature mistakenly places a required param after an
            // optional one.
            let mut required = 0usize;
            for (idx, p) in self
                .params
                .head
                .iter()
                .chain(self.params.tail.iter())
                .enumerate()
            {
                if !p.optional {
                    required = idx + 1;
                }
            }
            return required;
        }

        let head_required = self.params.head.iter().filter(|p| !p.optional).count();
        let tail_required = self.params.tail.iter().filter(|p| !p.optional).count();
        head_required + self.params.repeat.len() + tail_required
    }

    pub fn param_for_arg_index(&self, idx: usize) -> Option<&ParamSig> {
        if self.params.repeat.is_empty() {
            if idx < self.params.head.len() {
                return self.params.head.get(idx);
            }
            return self.params.tail.get(idx - self.params.head.len());
        }

        // Best-effort mapping without knowing total arg count (completion/sighelp).
        if idx < self.params.head.len() {
            return self.params.head.get(idx);
        }
        let idx = idx.saturating_sub(self.params.head.len());
        if self.params.repeat.is_empty() {
            return None;
        }
        self.params.repeat.get(idx % self.params.repeat.len())
    }
}

fn collect_generics_in_ty(ty: &Ty) -> Vec<GenericId> {
    fn walk(ty: &Ty, out: &mut Vec<GenericId>) {
        match ty {
            Ty::Generic(g) => out.push(*g),
            Ty::List(inner) => walk(inner, out),
            Ty::Union(members) => {
                for m in members {
                    walk(m, out);
                }
            }
            Ty::Number | Ty::String | Ty::Boolean | Ty::Date | Ty::Null | Ty::Unknown => {}
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
    }
}

//! Function signature model used by semantic analysis and editor tooling.
//!
//! Signatures use [`ParamShape`] for deterministic arity/shape rules; [`ParamShape::new`] enforces
//! invariants required for stable validation and signature help.

use super::{FunctionCategory, GenericId, Ty};
use std::collections::HashSet;

/// How a generic parameter binds during inference.
///
/// Controls how multiple bindings are merged during inference; see `analysis::infer` for the
/// current rules.
///
/// `Variant` is stricter around `Unknown` participation than `Plain`.
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
///
/// This shape is designed to make arity/shape validation and signature-help presentation stable.
/// By default, repeat shapes assume at least one repeat group (`repeat_min_groups = 1`).
/// Set `repeat_min_groups = 0` for truly optional variadic args (e.g. `splice(...items)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamShape {
    pub head: Vec<ParamSig>,
    pub repeat: Vec<ParamSig>,
    pub tail: Vec<ParamSig>,
    /// Minimum number of repeat-group cycles required. Defaults to `1`.
    pub repeat_min_groups: usize,
}

impl ParamShape {
    /// Construct a new [`ParamShape`] and enforce determinism invariants.
    ///
    /// # Panics
    /// Panics if:
    /// - any `repeat` param is marked `optional`,
    /// - `repeat` is non-empty and any `tail` param is optional (repeat+optional-tail is rejected),
    /// - `tail` contains a required param after an optional param (optional tail must be suffix-only).
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

        Self { head, repeat, tail, repeat_min_groups: 1 }
    }

    /// Set the minimum number of repeat-group cycles. Default is `1`.
    ///
    /// Use `0` for truly optional variadic args (e.g. `splice(list, start, count, ...items)`
    /// where the `items` group can be omitted entirely).
    pub fn with_repeat_min_groups(mut self, min: usize) -> Self {
        self.repeat_min_groups = min;
        self
    }
}

/// Custom type resolution function for builtins whose return type cannot be
/// expressed by the standard generic unification system.
///
/// Receives the base signature and the actual inferred argument types.
/// Returns a fully resolved signature with concrete types.
///
/// This is used by:
/// - The analyser's type inference (`infer.rs`) to compute return types
/// - The evaluator's runtime type validation (in test builds) to verify
///   function implementations return correct types
pub type SigResolver = fn(&FunctionSig, &[Ty]) -> FunctionSig;

/// A function signature used for semantic validation and editor tooling.
///
/// Builtin signatures also carry:
/// - `category` for UI grouping
/// - `detail` for completion/signature help display
#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: String,
    pub params: ParamShape,
    pub ret: Ty,
    pub category: FunctionCategory,
    pub detail: String,
    pub generics: Vec<GenericParam>,
    /// Optional custom type resolver. When set, type inference uses this
    /// instead of the standard generic unification path.
    pub resolver: Option<SigResolver>,
}

/// Equality compares all fields except `resolver` (function pointer address comparison is
/// unreliable across codegen units).
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
    /// Create a signature without additional validation.
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

    /// Create a builtin signature and validate stricter invariants.
    ///
    /// # Panics
    /// Panics if the signature violates builtin constraints (e.g. expected types contain
    /// [`Ty::Unknown`], or a used generic is not declared in `generics`).
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

    /// Create a builtin signature with a custom type resolver.
    ///
    /// # Panics
    /// Same as [`new_builtin`](Self::new_builtin).
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

    /// Returns a flat parameter list for signatures that are exactly `head` params.
    ///
    /// Currently this returns `Some(&head)` only when there is no `repeat` group and no `tail`.
    pub fn flat_params(&self) -> Option<&[ParamSig]> {
        if self.params.repeat.is_empty() && self.params.tail.is_empty() {
            return Some(&self.params.head);
        }
        None
    }

    /// Return the number of displayed parameter slots (`head + repeat + tail`).
    pub fn display_params_len(&self) -> usize {
        self.params.head.len() + self.params.repeat.len() + self.params.tail.len()
    }

    /// Return the displayed parameter slots (`head`, then `repeat`, then `tail`).
    ///
    /// This allocates; callers that only need fixed-arity parameters should prefer [`flat_params`].
    pub fn display_params(&self) -> Vec<&ParamSig> {
        self.params
            .head
            .iter()
            .chain(self.params.repeat.iter())
            .chain(self.params.tail.iter())
            .collect()
    }

    /// Returns true if the signature has a repeating group.
    pub fn is_variadic(&self) -> bool {
        !self.params.repeat.is_empty()
    }

    /// Return the minimum number of arguments required by this signature.
    ///
    /// Currently:
    /// - For fixed-arity signatures (no `repeat`), this is the index of the last required param + 1
    ///   across `head` then `tail`.
    /// - For repeat-group signatures, this assumes one repeat group is required and adds required
    ///   `head` + one `repeat` group + required `tail`.
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
        head_required + self.params.repeat.len() * self.params.repeat_min_groups + tail_required
    }

    /// Best-effort mapping from argument index to a parameter slot.
    ///
    /// For repeat-group signatures this does not consider `tail` (because the total argument count
    /// is unknown); it cycles through the `repeat` group after `head`.
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

use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func_g!(
            FunctionCategory::People,
            "name(person)",
            generics!(g!(0, Plain)),
            "name",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Generic(t0)
            )),
            Ty::String,
        ),
        func_g!(
            FunctionCategory::People,
            "email(person)",
            generics!(g!(0, Plain)),
            "email",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Generic(t0)
            )),
            Ty::String,
        ),
    ]
}

use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![func_g!(
        FunctionCategory::Special,
        "id(page?)",
        generics!(g!(0, Plain)),
        "id",
        params!(opt!(
            "page",
            // if you have Ty::Page, use it here
            Ty::Generic(t0)
        )),
        Ty::String,
    )]
}

use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func_g!(
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
        ),
        func_g!(
            FunctionCategory::Special,
            "equal(a, b)",
            generics!(g!(0, Plain)),
            "equal",
            params!(p!("a", Ty::Generic(t0)), p!("b", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::Special,
            "unequal(a, b)",
            generics!(g!(0, Plain)),
            "unequal",
            params!(p!("a", Ty::Generic(t0)), p!("b", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::Special,
            "let(var, value, expr)",
            generics!(g!(0, Plain)),
            "let",
            // let(var, value, expr)
            params!(
                p!(
                    "var",
                    // identifier slot
                    Ty::Generic(t0)
                ),
                p!("value", Ty::Generic(t0)),
                p!("expr", Ty::Generic(t0))
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::Special,
            "lets(var1, value1, ..., expr)",
            generics!(g!(0, Plain)),
            "lets",
            // lets(a, v1, b, v2, ..., expr)
            repeat_params!(
                head!(),
                repeat!(
                    p!(
                        "var",
                        // identifier slot
                        Ty::Generic(t0)
                    ),
                    p!("value", Ty::Generic(t0))
                ),
                tail!(p!("expr", Ty::Generic(t0))),
            ),
            Ty::Generic(t0),
        ),
    ]
}

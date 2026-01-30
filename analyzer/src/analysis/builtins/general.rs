use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func_g!(
            FunctionCategory::General,
            "if(condition, then, else)",
            generics!(g!(0, Variant)),
            "if",
            params!(
                p!("condition", Ty::Boolean),
                p!("then", Ty::Generic(t0)),
                p!("else", Ty::Generic(t0))
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::General,
            "ifs(condition, value, ..., default)",
            generics!(g!(0, Variant)),
            "ifs",
            repeat_params!(
                head!(),
                repeat!(p!("condition", Ty::Boolean), p!("value", Ty::Generic(t0))),
                tail!(p!("default", Ty::Generic(t0))),
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::General,
            "empty(value)",
            generics!(g!(0, Plain)),
            "empty",
            params!(p!("value", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::General,
            "format(value)",
            generics!(g!(0, Plain)),
            "format",
            params!(p!("value", Ty::Generic(t0))),
            Ty::String,
        ),
        func_g!(
            FunctionCategory::General,
            "toNumber(value)",
            generics!(g!(0, Plain)),
            "toNumber",
            params!(p!("value", Ty::Generic(t0))),
            Ty::Number,
        ),
    ]
}

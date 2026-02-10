use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        // TODO(spec): `formatNumber(value, format, precision)` is not modeled yet.
        func!(
            FunctionCategory::Number,
            "add(a, b)",
            "add",
            params!(p!("a", Ty::Number), p!("b", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "subtract(a, b)",
            "subtract",
            params!(p!("a", Ty::Number), p!("b", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "multiply(a, b)",
            "multiply",
            params!(p!("a", Ty::Number), p!("b", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "mod(a, b)",
            "mod",
            params!(p!("a", Ty::Number), p!("b", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "pow(base, exp)",
            "pow",
            params!(p!("base", Ty::Number), p!("exp", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "divide(a, b)",
            "divide",
            params!(p!("a", Ty::Number), p!("b", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "min(values1, values2, ...)",
            "min",
            repeat_params_with_tail!(
                repeat!(p!(
                    "values",
                    Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))])
                )),
                tail!(),
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "max(values1, values2, ...)",
            "max",
            repeat_params_with_tail!(
                repeat!(p!(
                    "values",
                    Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))])
                )),
                tail!(),
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "sum(values1, values2, ...)",
            "sum",
            repeat_params_with_tail!(
                repeat!(p!(
                    "values",
                    Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))])
                )),
                tail!(),
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "median(values1, values2, ...)",
            "median",
            repeat_params_with_tail!(
                repeat!(p!(
                    "values",
                    Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))])
                )),
                tail!(),
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "mean(values1, values2, ...)",
            "mean",
            repeat_params_with_tail!(
                repeat!(p!(
                    "values",
                    Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))])
                )),
                tail!(),
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "abs(value)",
            "abs",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "round(value, places?)",
            "round",
            params!(p!("value", Ty::Number), opt!("places", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "ceil(value)",
            "ceil",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "floor(value)",
            "floor",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "sqrt(value)",
            "sqrt",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "cbrt(value)",
            "cbrt",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "exp(value)",
            "exp",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "ln(value)",
            "ln",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "log10(value)",
            "log10",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "log2(value)",
            "log2",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "sign(value)",
            "sign",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "pi()",
            "pi",
            params!(),
            Ty::Number,
        ),
        func!(FunctionCategory::Number, "e()", "e", params!(), Ty::Number,),
        func_g!(
            FunctionCategory::Number,
            "toNumber(value)",
            generics!(g!(0, Plain)),
            "toNumber",
            params!(p!("value", Ty::Generic(t0))),
            Ty::Number,
        ),
    ]
}

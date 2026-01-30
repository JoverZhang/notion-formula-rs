use super::super::{FunctionCategory, FunctionSig, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    vec![
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
            "divide(a, b)",
            "divide",
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
            "min(number|number[], ...)",
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
            "max(number|number[], ...)",
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
            "sum(number|number[], ...)",
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
            "median(number|number[], ...)",
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
            "mean(number|number[], ...)",
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
            "abs(number)",
            "abs",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "round(number, places?)",
            "round",
            params!(p!("value", Ty::Number), opt!("places", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "ceil(number)",
            "ceil",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "floor(number)",
            "floor",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "sqrt(number)",
            "sqrt",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "cbrt(number)",
            "cbrt",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "exp(number)",
            "exp",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "ln(number)",
            "ln",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "log10(number)",
            "log10",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "log2(number)",
            "log2",
            params!(p!("value", Ty::Number)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Number,
            "sign(number)",
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
    ]
}

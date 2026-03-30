use super::super::{FunctionCategory, FunctionSig, GenericId, LambdaParam, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    let t1 = GenericId(1);
    vec![
        func_g!(
            FunctionCategory::General,
            "if(condition, then, else)",
            generics!(g!(0, Variant)),
            "if",
            params!(
                p!("condition", Ty::Boolean),
                p!(
                    "then",
                    Ty::Fn {
                        params: vec![],
                        ret: Box::new(Ty::Generic(t0)),
                    }
                ),
                p!(
                    "else",
                    Ty::Fn {
                        params: vec![],
                        ret: Box::new(Ty::Generic(t0)),
                    }
                )
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::General,
            "ifs(condition1, value1, ..., else)",
            generics!(g!(0, Variant)),
            "ifs",
            repeat_params!(
                head!(),
                repeat!(
                    p!("condition1", Ty::Boolean),
                    p!(
                        "value1",
                        Ty::Fn {
                            params: vec![],
                            ret: Box::new(Ty::Generic(t0)),
                        }
                    )
                ),
                tail!(p!(
                    "else",
                    Ty::Fn {
                        params: vec![],
                        ret: Box::new(Ty::Generic(t0)),
                    }
                )),
            ),
            Ty::Generic(t0),
        ),
        // TODO(spec): keep logical operators as operators (`&&`, `||`, `not`) for now.
        // Function-call forms `and(...)`, `or(...)`, `not(...)` are not modeled yet.
        func_g!(
            FunctionCategory::General,
            "empty(value?)",
            generics!(g!(0, Plain)),
            "empty",
            params!(opt!("value", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::General,
            "length(value)",
            generics!(g!(0, Plain)),
            "length",
            params!(p!(
                "value",
                Ty::Union(vec![Ty::String, Ty::List(Box::new(Ty::Generic(t0)))])
            )),
            Ty::Number,
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
            "equal(a, b)",
            generics!(g!(0, Plain)),
            "equal",
            params!(p!("a", Ty::Generic(t0)), p!("b", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::General,
            "unequal(a, b)",
            generics!(g!(0, Plain)),
            "unequal",
            params!(p!("a", Ty::Generic(t0)), p!("b", Ty::Generic(t0))),
            Ty::Boolean,
        ),
        // TODO(binder): `lets(var1, value1, ..., expr)` binder semantics need multi-generic variadic support.
        func_g!(
            FunctionCategory::General,
            "let(ident, value, body)",
            generics!(g!(0, Plain), g!(1, Plain)),
            "let",
            params!(
                p!("ident", Ty::Ident(Box::new(Ty::Generic(t0)))),
                p!("value", Ty::Generic(t0)),
                p!(
                    "body",
                    Ty::Fn {
                        params: vec![(LambdaParam::ParamRef("ident".into()), Ty::Generic(t0))],
                        ret: Box::new(Ty::Generic(t1)),
                    }
                )
            ),
            Ty::Generic(t1),
        ),
    ]
}

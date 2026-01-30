use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func_g!(
            FunctionCategory::List,
            "at(list, index)",
            generics!(g!(0, Plain)),
            "at",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("index", Ty::Number)
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "first(list)",
            generics!(g!(0, Plain)),
            "first",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "last(list)",
            generics!(g!(0, Plain)),
            "last",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "slice(list, start, end?)",
            generics!(g!(0, Plain)),
            "slice",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("start", Ty::Number),
                opt!("end", Ty::Number)
            ),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "concat(list, ...)",
            generics!(g!(0, Plain)),
            "concat",
            repeat_params_with_tail!(
                repeat!(p!("lists", Ty::List(Box::new(Ty::Generic(t0))))),
                tail!(),
            ),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "sort(list)",
            generics!(g!(0, Plain)),
            "sort",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "reverse(list)",
            generics!(g!(0, Plain)),
            "reverse",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "join(list, separator)",
            generics!(g!(0, Plain)),
            "join",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("separator", Ty::String)
            ),
            Ty::String,
        ),
        func!(
            FunctionCategory::List,
            "split(text, separator)",
            "split",
            params!(p!("text", Ty::String), p!("separator", Ty::String)),
            Ty::List(Box::new(Ty::String)),
        ),
        func_g!(
            FunctionCategory::List,
            "unique(list)",
            generics!(g!(0, Plain)),
            "unique",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "includes(list, value)",
            generics!(g!(0, Plain)),
            "includes",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("value", Ty::Generic(t0))
            ),
            Ty::Boolean,
        ),
        // TODO(lambda-typing): Intentionally removed until we have a real lambda/function type system.
        // NOTE: Notion’s predicate/mapper DSL may include (current, index) etc.; keep minimal forms here.
        // TODO(lambda-typing): find<T>(list: T[], predicate: (current) -> boolean) -> T
        // TODO(lambda-typing): findIndex<T>(list: T[], predicate: (current) -> boolean) -> number
        // TODO(lambda-typing): filter<T>(list: T[], predicate: (current) -> boolean) -> T[]
        // TODO(lambda-typing): some<T>(list: T[], predicate: (current) -> boolean) -> boolean
        // TODO(lambda-typing): every<T>(list: T[], predicate: (current) -> boolean) -> boolean
        // TODO(lambda-typing): map<T, U>(list: T[], mapper: (current) -> U) -> U[]
        func_g!(
            FunctionCategory::List,
            "flat(list)",
            generics!(g!(0, Plain)),
            "flat",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
    ]
}

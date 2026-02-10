use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func!(
            FunctionCategory::Text,
            "substring(text, start, end?)",
            "substring",
            params!(
                p!("text", Ty::String),
                p!("start", Ty::Number),
                opt!("end", Ty::Number)
            ),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "contains(text, search)",
            "contains",
            params!(p!("text", Ty::String), p!("search", Ty::String)),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::Text,
            "test(text, regex)",
            "test",
            params!(p!("text", Ty::String), p!("regex", Ty::String)),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::Text,
            "match(text, regex)",
            "match",
            params!(p!("text", Ty::String), p!("regex", Ty::String)),
            Ty::List(Box::new(Ty::String)),
        ),
        func!(
            FunctionCategory::Text,
            "replace(text, regex, replacement)",
            "replace",
            params!(
                p!("text", Ty::String),
                p!("regex", Ty::String),
                p!("replacement", Ty::String)
            ),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "replaceAll(text, regex, replacement)",
            "replaceAll",
            params!(
                p!("text", Ty::String),
                p!("regex", Ty::String),
                p!("replacement", Ty::String)
            ),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "lower(text)",
            "lower",
            params!(p!("text", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "upper(text)",
            "upper",
            params!(p!("text", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "trim(text)",
            "trim",
            params!(p!("text", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "repeat(text, times)",
            "repeat",
            params!(p!("text", Ty::String), p!("times", Ty::Number)),
            Ty::String,
        ),
        // TODO(spec): `padStart(text, length, pad)` is not modeled yet.
        // TODO(spec): `padEnd(text, length, pad)` is not modeled yet.
        // TODO(type-model): `link(label, url) -> Link` is blocked on rich text types.
        // TODO(type-model): `style(text, styles1, styles2, ...) -> StyledText` is blocked on rich text types.
        // TODO(type-model): `unstyle(text, styles?) -> string` with `StyledText` input is blocked on rich text types.
        func_g!(
            FunctionCategory::Text,
            "concat(lists1, lists2, ...)",
            generics!(g!(0, Plain)),
            "concat",
            repeat_params!(
                head!(p!("lists1", Ty::List(Box::new(Ty::Generic(t0))))),
                repeat!(p!("listsN", Ty::List(Box::new(Ty::Generic(t0))))),
                tail!(),
            ),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::Text,
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
            FunctionCategory::Text,
            "split(text, separator)",
            "split",
            params!(p!("text", Ty::String), p!("separator", Ty::String)),
            Ty::List(Box::new(Ty::String)),
        ),
    ]
}

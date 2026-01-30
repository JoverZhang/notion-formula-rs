use super::super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        func_g!(
            FunctionCategory::Text,
            "length(text|any[])",
            generics!(g!(0, Plain)),
            "length",
            params!(p!(
                "value",
                Ty::Union(vec![Ty::String, Ty::List(Box::new(Ty::Generic(t0)))])
            )),
            Ty::Number,
        ),
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
            "repeat(text, times)",
            "repeat",
            params!(p!("text", Ty::String), p!("times", Ty::Number)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "link(label, url)",
            "link",
            params!(p!("label", Ty::String), p!("url", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "style(text, styleOrColor, ...)",
            "style",
            repeat_params!(
                head!(p!("text", Ty::String)),
                repeat!(p!("styles", Ty::String)),
                tail!(),
            ),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "unstyle(text, style?)",
            "unstyle",
            params!(p!("text", Ty::String), opt!("styles", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Text,
            "trim(text)",
            "trim",
            params!(p!("text", Ty::String)),
            Ty::String,
        ),
    ]
}

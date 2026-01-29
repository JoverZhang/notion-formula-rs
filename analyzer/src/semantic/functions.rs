use super::{FunctionCategory, FunctionSig, GenericId, Ty};

pub fn builtins_functions() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    vec![
        // =========================
        // General / Logic
        // =========================
        func_g!(
            FunctionCategory::General,
            "if(condition, then, else)",
            generics!(g!(0, Plain)),
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
        func!(
            FunctionCategory::General,
            "empty(value)",
            "empty",
            params!(p!("value", Ty::Unknown)),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::General,
            "format(value)",
            "format",
            params!(p!("value", Ty::Unknown)),
            Ty::String,
        ),
        func!(
            FunctionCategory::General,
            "toNumber(value)",
            "toNumber",
            params!(p!("value", Ty::Unknown)),
            Ty::Number,
        ),
        // =========================
        // Text
        // =========================
        func!(
            FunctionCategory::Text,
            "length(text|any[])",
            "length",
            params!(p!(
                "value",
                Ty::Union(vec![Ty::String, Ty::List(Box::new(Ty::Unknown))])
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
        // =========================
        // Number
        // =========================
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
        // =========================
        // Date
        // =========================
        func!(FunctionCategory::Date, "now()", "now", params!(), Ty::Date,),
        func!(
            FunctionCategory::Date,
            "today()",
            "today",
            params!(),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "minute(date)",
            "minute",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "hour(date)",
            "hour",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "day(date)",
            "day",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "date(date)",
            "date",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "week(date)",
            "week",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "month(date)",
            "month",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "year(date)",
            "year",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "dateAdd(date, amount, unit)",
            "dateAdd",
            params!(
                p!("date", Ty::Date),
                p!("amount", Ty::Number),
                p!("unit", Ty::String)
            ),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "dateSubtract(date, amount, unit)",
            "dateSubtract",
            params!(
                p!("date", Ty::Date),
                p!("amount", Ty::Number),
                p!("unit", Ty::String)
            ),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "dateBetween(a, b, unit)",
            "dateBetween",
            params!(p!("a", Ty::Date), p!("b", Ty::Date), p!("unit", Ty::String)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "dateRange(start, end)",
            "dateRange",
            params!(p!("start", Ty::Date), p!("end", Ty::Date)),
            // Note: if you have Ty::DateRange, use it instead of Ty::Date here.
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "dateStart(range)",
            "dateStart",
            params!(p!(
                "range",
                // Note: if you have Ty::DateRange, use it here instead of Ty::Date.
                Ty::Date
            )),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "dateEnd(range)",
            "dateEnd",
            params!(p!(
                "range",
                // Note: if you have Ty::DateRange, use it here instead of Ty::Date.
                Ty::Date
            )),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "timestamp(date)",
            "timestamp",
            params!(p!("date", Ty::Date)),
            Ty::Number,
        ),
        func!(
            FunctionCategory::Date,
            "fromTimestamp(timestampMs)",
            "fromTimestamp",
            params!(p!("timestamp", Ty::Number)),
            Ty::Date,
        ),
        func!(
            FunctionCategory::Date,
            "formatDate(date, format)",
            "formatDate",
            params!(p!("date", Ty::Date), p!("format", Ty::String)),
            Ty::String,
        ),
        func!(
            FunctionCategory::Date,
            "parseDate(text)",
            "parseDate",
            params!(p!("text", Ty::String)),
            Ty::Date,
        ),
        // =========================
        // People
        // =========================
        func!(
            FunctionCategory::People,
            "name(person)",
            "name",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Unknown
            )),
            Ty::String,
        ),
        func!(
            FunctionCategory::People,
            "email(person)",
            "email",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Unknown
            )),
            Ty::String,
        ),
        // =========================
        // List
        // =========================
        func!(
            FunctionCategory::List,
            "at(list, index)",
            "at",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("index", Ty::Number)
            ),
            Ty::Unknown,
        ),
        func!(
            FunctionCategory::List,
            "first(list)",
            "first",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::Unknown,
        ),
        func!(
            FunctionCategory::List,
            "last(list)",
            "last",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::Unknown,
        ),
        func!(
            FunctionCategory::List,
            "slice(list, start, end?)",
            "slice",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("start", Ty::Number),
                opt!("end", Ty::Number)
            ),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "concat(list, ...)",
            "concat",
            repeat_params_with_tail!(
                repeat!(p!("lists", Ty::List(Box::new(Ty::Unknown)))),
                tail!(),
            ),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "sort(list)",
            "sort",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "reverse(list)",
            "reverse",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "join(list, separator)",
            "join",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
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
        func!(
            FunctionCategory::List,
            "unique(list)",
            "unique",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "includes(list, value)",
            "includes",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("value", Ty::Unknown)
            ),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::List,
            "find(list, predicate)",
            "find",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!(
                    "predicate",
                    // lambda expr (current/index) in Notion DSL
                    Ty::Unknown
                )
            ),
            Ty::Unknown,
        ),
        func!(
            FunctionCategory::List,
            "findIndex(list, predicate)",
            "findIndex",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("predicate", Ty::Unknown)
            ),
            Ty::Number,
        ),
        func!(
            FunctionCategory::List,
            "filter(list, predicate)",
            "filter",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("predicate", Ty::Unknown)
            ),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "some(list, predicate)",
            "some",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("predicate", Ty::Unknown)
            ),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::List,
            "every(list, predicate)",
            "every",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("predicate", Ty::Unknown)
            ),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::List,
            "map(list, mapper)",
            "map",
            params!(
                p!("list", Ty::List(Box::new(Ty::Unknown))),
                p!("mapper", Ty::Unknown)
            ),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        func!(
            FunctionCategory::List,
            "flat(list)",
            "flat",
            params!(p!("list", Ty::List(Box::new(Ty::Unknown)))),
            Ty::List(Box::new(Ty::Unknown)),
        ),
        // =========================
        // Special / Utility
        // =========================
        func!(
            FunctionCategory::Special,
            "id(page?)",
            "id",
            params!(opt!(
                "page",
                // if you have Ty::Page, use it here
                Ty::Unknown
            )),
            Ty::String,
        ),
        func!(
            FunctionCategory::Special,
            "equal(a, b)",
            "equal",
            params!(p!("a", Ty::Unknown), p!("b", Ty::Unknown)),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::Special,
            "unequal(a, b)",
            "unequal",
            params!(p!("a", Ty::Unknown), p!("b", Ty::Unknown)),
            Ty::Boolean,
        ),
        func!(
            FunctionCategory::Special,
            "let(var, value, expr)",
            "let",
            // let(var, value, expr)
            params!(
                p!(
                    "var",
                    // identifier slot
                    Ty::Unknown
                ),
                p!("value", Ty::Unknown),
                p!("expr", Ty::Unknown)
            ),
            Ty::Unknown,
        ),
        func!(
            FunctionCategory::Special,
            "lets(var1, value1, ..., expr)",
            "lets",
            // lets(a, v1, b, v2, ..., expr)
            repeat_params!(
                head!(),
                repeat!(
                    p!(
                        "var",
                        // identifier slot
                        Ty::Unknown
                    ),
                    p!("value", Ty::Unknown)
                ),
                tail!(p!("expr", Ty::Unknown)),
            ),
            Ty::Unknown,
        ),
    ]
}

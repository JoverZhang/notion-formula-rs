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
        // =========================
        // Text
        // =========================
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
            "sum(number, ...)",
            "sum",
            repeat_params_with_tail!(
                // TODO: restore `number[]` once list literals or an equivalent array expression exists.
                repeat!(p!("values", Ty::Number)),
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
        func_g!(
            FunctionCategory::People,
            "name(person)",
            generics!(g!(0, Plain)),
            "name",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Generic(t0)
            )),
            Ty::String,
        ),
        func_g!(
            FunctionCategory::People,
            "email(person)",
            generics!(g!(0, Plain)),
            "email",
            params!(p!(
                "person",
                // TODO: Notion's person type is more complex than this.
                Ty::Generic(t0)
            )),
            Ty::String,
        ),
        // =========================
        // List
        // =========================
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
        // =========================
        // Special / Utility
        // =========================
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

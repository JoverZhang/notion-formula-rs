use super::super::{FunctionCategory, FunctionSig, Ty};

pub(super) fn builtins() -> Vec<FunctionSig> {
    vec![
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
    ]
}

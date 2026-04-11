use super::sig;
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(parser, FunctionCategory::Date, "now() -> date"),
        sig(parser, FunctionCategory::Date, "today() -> date"),
        sig(
            parser,
            FunctionCategory::Date,
            "minute(date: date) -> number",
        ),
        sig(parser, FunctionCategory::Date, "hour(date: date) -> number"),
        sig(parser, FunctionCategory::Date, "day(date: date) -> number"),
        sig(parser, FunctionCategory::Date, "date(date: date) -> number"),
        sig(parser, FunctionCategory::Date, "week(date: date) -> number"),
        sig(
            parser,
            FunctionCategory::Date,
            "month(date: date) -> number",
        ),
        sig(parser, FunctionCategory::Date, "year(date: date) -> number"),
        sig(
            parser,
            FunctionCategory::Date,
            "dateAdd(date: date, amount: number, unit: string) -> date",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "dateSubtract(date: date, amount: number, unit: string) -> date",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "dateBetween(a: date, b: date, unit: string) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "timestamp(date: date) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "fromTimestamp(timestamp: number) -> date",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "formatDate(date: date, format: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Date,
            "parseDate(text: string) -> date",
        ),
    ]
}

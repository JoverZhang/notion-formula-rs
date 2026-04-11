use super::{sig, sig_with_detail};
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(
            parser,
            FunctionCategory::Number,
            "formatNumber(value: number, format: string, precision: number) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "add(a: number, b: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "subtract(a: number, b: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "multiply(a: number, b: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "mod(a: number, b: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "pow(base: number, exp: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "divide(a: number, b: number) -> number",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Number,
            "min(values: number | number[], ...) -> number",
            "min(values1, values2, ...)",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Number,
            "max(values: number | number[], ...) -> number",
            "max(values1, values2, ...)",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Number,
            "sum(values: number | number[], ...) -> number",
            "sum(values1, values2, ...)",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Number,
            "median(values: number | number[], ...) -> number",
            "median(values1, values2, ...)",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Number,
            "mean(values: number | number[], ...) -> number",
            "mean(values1, values2, ...)",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "abs(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "round(value: number, places?: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "ceil(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "floor(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "sqrt(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "cbrt(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "exp(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "ln(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "log10(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "log2(value: number) -> number",
        ),
        sig(
            parser,
            FunctionCategory::Number,
            "sign(value: number) -> number",
        ),
        sig(parser, FunctionCategory::Number, "pi() -> number"),
        sig(parser, FunctionCategory::Number, "e() -> number"),
        sig(
            parser,
            FunctionCategory::Number,
            "toNumber<T: Plain>(value: T) -> number",
        ),
    ]
}

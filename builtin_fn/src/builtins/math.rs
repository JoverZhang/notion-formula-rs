use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Number;

        formatNumber(value: number, format: string, precision: number) -> string;
        add(a: number, b: number) -> number;
        subtract(a: number, b: number) -> number;
        multiply(a: number, b: number) -> number;
        mod(a: number, b: number) -> number;
        pow(base: number, exp: number) -> number;
        divide(a: number, b: number) -> number;

        min(repeat(min = 1) { values: number | number[] }) -> number;
        max(repeat(min = 1) { values: number | number[] }) -> number;
        sum(repeat(min = 1) { values: number | number[] }) -> number;
        median(repeat(min = 1) { values: number | number[] }) -> number;
        mean(repeat(min = 1) { values: number | number[] }) -> number;

        abs(value: number) -> number;
        round(value: number, places?: number) -> number;
        ceil(value: number) -> number;
        floor(value: number) -> number;
        sqrt(value: number) -> number;
        cbrt(value: number) -> number;
        exp(value: number) -> number;
        ln(value: number) -> number;
        log10(value: number) -> number;
        log2(value: number) -> number;
        sign(value: number) -> number;
        pi() -> number;
        e() -> number;
        toNumber(value: any) -> number;
    }
}

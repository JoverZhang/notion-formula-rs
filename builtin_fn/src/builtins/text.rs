use super::{sig, sig_with_detail};
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(
            parser,
            FunctionCategory::Text,
            "substring(text: string, start: number, end?: number) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "contains(text: string, search: string) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "test(text: string, regex: string) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "match(text: string, regex: string) -> string[]",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "replace(text: string, regex: string, replacement: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "replaceAll(text: string, regex: string, replacement: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "lower(text: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "upper(text: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "trim(text: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "repeat(text: string, times: number) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "padStart(text: string | number, length: number, pad: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "padEnd(text: string | number, length: number, pad: string) -> string",
        ),
        sig_with_detail(
            parser,
            FunctionCategory::Text,
            "concat<T: Plain>(lists1: T[], listsN: T[], ...) -> T[]",
            "concat(lists1, lists2, ...)",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "join<T: Plain>(list: T[], separator: string) -> string",
        ),
        sig(
            parser,
            FunctionCategory::Text,
            "split(text: string, separator: string) -> string[]",
        ),
    ]
}

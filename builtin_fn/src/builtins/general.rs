use super::sig;
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(
            parser,
            FunctionCategory::General,
            "if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "ifs<T: Variant>(condition1: boolean, value1: () -> T, ..., else: () -> T) -> T",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "empty<T: Plain>(value?: T) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "length<T: Plain>(value: string | T[]) -> number",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "format<T: Plain>(value: T) -> string",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "equal<T: Plain>(a: T, b: T) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "unequal<T: Plain>(a: T, b: T) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::General,
            "let<T: Plain, U: Plain>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U",
        ),
    ]
}

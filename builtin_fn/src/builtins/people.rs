use super::sig;
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(
            parser,
            FunctionCategory::People,
            "name<T: Plain>(person: T) -> string",
        ),
        sig(
            parser,
            FunctionCategory::People,
            "email<T: Plain>(person: T) -> string",
        ),
    ]
}

use super::sig;
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig};

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![sig(
        parser,
        FunctionCategory::Special,
        "id<T: Plain>(page?: T) -> string",
    )]
}

mod builtins;
mod param_shape;
mod parser;
mod signature;
mod type_hints;
mod types;

pub use builtins::builtins_functions;
pub use param_shape::resolve_repeat_tail_used;
pub use parser::{
    BuiltinSigParseError, BuiltinSigParseErrorKind, BuiltinSigParser, GenericKindRegistry,
};
pub use signature::{
    FunctionSig, GenericParam, GenericParamKind, ParamShape, ParamSig, SigResolver,
};
pub use type_hints::normalize_union;
pub use types::{FunctionCategory, GenericId, LambdaParam, Ty};

pub fn default_parser() -> BuiltinSigParser {
    BuiltinSigParser::new(GenericKindRegistry::with_builtin_kinds())
}

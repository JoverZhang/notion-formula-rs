extern crate self as builtin_fn;

mod builtins;
mod catalog;
mod param_shape;
mod parser;
mod resolution;
mod signature;
mod type_hints;
mod types;

pub use builtin_fn_macros::builtin_functions;
pub use builtins::{builtin_categories, builtins_functions};
pub use catalog::{BuiltinCatalogEntry, BuiltinCategory};
pub use param_shape::resolve_repeat_tail_used;
pub use parser::{
    BuiltinSigParseError, BuiltinSigParseErrorKind, BuiltinSigParser, GenericKindRegistry,
};
pub use resolution::{
    ArgumentObservation, ArgumentTypeStatus, CallShapeError, CallSignatureInput, ParamRef,
    ResolvedArgument, ResolvedFunctionSig, ResolvedParamSlot, ResolverInput, ShapeValidity,
    check_argument_type, param_for_ref, resolve_call_signature, type_accepts,
};
pub use signature::{
    FunctionSig, GenericParam, GenericParamKind, ParamShape, ParamSig, SigResolver,
};
pub use type_hints::normalize_union;
pub use types::{FunctionCategory, GenericId, LambdaParam, Ty};

pub fn default_parser() -> BuiltinSigParser {
    BuiltinSigParser::new(GenericKindRegistry::with_builtin_kinds())
}

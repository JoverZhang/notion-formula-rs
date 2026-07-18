use crate::{BuiltinCategory, FunctionSig};

mod date;
mod general;
mod list;
mod math;
mod people;
mod special;
mod text;

/// Return the complete declaration catalog in stable category order.
pub fn builtin_categories() -> Vec<BuiltinCategory> {
    vec![
        general::definitions(),
        text::definitions(),
        math::definitions(),
        date::definitions(),
        people::definitions(),
        list::definitions(),
        special::definitions(),
    ]
}

/// Return only declarations that have semantic and runtime implementation obligations.
pub fn builtins_functions() -> Vec<FunctionSig> {
    builtin_categories()
        .into_iter()
        .flat_map(BuiltinCategory::into_functions)
        .collect()
}

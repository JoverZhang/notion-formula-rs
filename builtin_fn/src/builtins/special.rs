use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Special;

        id() -> string;
    }
}

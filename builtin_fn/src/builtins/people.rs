use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: People;

        #[unsupported]
        /// Runtime inputs do not currently provide a person's display name.
        name(person: any) -> string;

        #[unsupported]
        /// Runtime inputs do not currently provide a person's email address.
        email(person: any) -> string;
    }
}

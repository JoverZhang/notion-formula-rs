use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: General;

        if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T;

        ifs<T: Variant>(
            repeat(min = 1) {
                condition: boolean,
                value: () -> T,
            },
            else: () -> T,
        ) -> T;

        #[unsupported]
        /// Currently expressed by the `&&` operator rather than a builtin call.
        and(
            repeat(min = 2) {
                condition: boolean,
            },
        ) -> boolean;

        #[unsupported]
        /// Currently expressed by the `||` operator rather than a builtin call.
        or(
            repeat(min = 2) {
                condition: boolean,
            },
        ) -> boolean;

        #[unsupported]
        /// Currently expressed by the `not` prefix operator rather than a builtin call.
        not(condition: boolean) -> boolean;

        empty(value?: any) -> boolean;
        length(value: string | any[]) -> number;
        format(value: any) -> string;
        equal(a: any, b: any) -> boolean;
        unequal(a: any, b: any) -> boolean;

        let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;

        #[unsupported]
        /// Precise sequential binder typing requires a heterogeneous binder-pack model.
        lets(
            repeat(min = 1) {
                var: Ident<any>,
                value: any,
            },
            expr: () -> any,
        ) -> any;
    }
}

use crate::{BuiltinCategory, builtin_functions};

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Text;

        substring(text: string, start: number, end?: number) -> string;
        contains(text: string, search: string) -> boolean;
        test(text: string, regex: string) -> boolean;
        match(text: string, regex: string) -> string[];
        replace(text: string, regex: string, replacement: string) -> string;
        replaceAll(text: string, regex: string, replacement: string) -> string;
        lower(text: string) -> string;
        upper(text: string) -> string;
        trim(text: string) -> string;
        repeat(text: string, times: number) -> string;
        padStart(text: string | number, length: number, pad: string) -> string;
        padEnd(text: string | number, length: number, pad: string) -> string;

        #[unsupported]
        /// The semantic type model does not yet represent `Link`.
        link(label: string, url: string) -> Link;

        #[unsupported]
        /// The semantic type model does not yet represent `StyledText`.
        style(
            text: string,
            repeat(min = 1) {
                styles: string,
            },
        ) -> StyledText;

        #[unsupported]
        /// The semantic type model does not yet represent `StyledText`.
        unstyle(text: string | StyledText, styles?: string) -> string;

        concat<T>(
            repeat(min = 2) {
                lists: T[],
            },
        ) -> T[];

        join<T>(list: T[], separator: string) -> string;
        split(text: string, separator: string) -> string[];
    }
}

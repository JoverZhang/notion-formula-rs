use builtin_fn::{BuiltinCategory, ResolverInput, Ty, builtin_functions};

fn passthrough(input: &ResolverInput<'_>) -> Ty {
    input.default_return_ty.clone()
}

fn main() {
    let catalog: BuiltinCategory = builtin_functions! {
        category: List;

        #[resolver(passthrough)]
        flat<T>(list: T[]) -> T[];

        concat<T>(
            repeat(min = 2) {
                lists: T[],
            },
        ) -> T[];

        splice<T>(
            list: T[],
            startIndex: number,
            deleteCount: number,
            repeat(min = 0) {
                items: T,
            },
        ) -> T[];

        ifs<T: Variant>(
            repeat(min = 1) {
                condition: boolean,
                value: () -> T,
            },
            else: () -> T,
        ) -> T;

        caseOf<T, U: Variant>(
            subject: T,
            repeat(min = 1) {
                candidate: T,
                result: () -> U,
            },
            otherwise: () -> U,
        ) -> U;

        #[unsupported]
        /// The runtime does not have this type.
        style(text: string, style?: string) -> StyledText;
    };

    assert_eq!(catalog.entries.len(), 6);
}

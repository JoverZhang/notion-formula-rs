use builtin_fn::{BuiltinCategory, ParamShape, ResolverInput, Ty, builtin_functions};

fn passthrough_resolver(input: &ResolverInput<'_>) -> Ty {
    input.default_return_ty.clone()
}

fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: List;

        #[resolver(passthrough_resolver)]
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
        /// `StyledText` is not represented yet.
        style(
            text: string,
            repeat(min = 1) {
                styles: string,
            },
        ) -> StyledText;
    }
}

#[test]
fn category_macro_covers_all_parameter_layouts() {
    let category = definitions();
    assert_eq!(category.entries.len(), 6);

    let shape = |name: &str| -> ParamShape {
        category
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .and_then(|entry| entry.implementation.as_ref())
            .unwrap_or_else(|| panic!("missing supported declaration `{name}`"))
            .params
            .clone()
    };

    let flat = shape("flat");
    assert_eq!(
        (flat.head.len(), flat.repeat.len(), flat.tail.len()),
        (1, 0, 0)
    );

    let concat = shape("concat");
    assert_eq!(
        (concat.head.len(), concat.repeat.len(), concat.tail.len()),
        (0, 1, 0)
    );
    assert_eq!(concat.repeat_min_groups, 2);
    assert_eq!(concat.repeat[0].name, "lists");

    let splice = shape("splice");
    assert_eq!(
        (splice.head.len(), splice.repeat.len(), splice.tail.len()),
        (3, 1, 0)
    );
    assert_eq!(splice.repeat_min_groups, 0);

    let ifs = shape("ifs");
    assert_eq!(
        (ifs.head.len(), ifs.repeat.len(), ifs.tail.len()),
        (0, 2, 1)
    );

    let case_of = shape("caseOf");
    assert_eq!(
        (case_of.head.len(), case_of.repeat.len(), case_of.tail.len()),
        (1, 2, 1)
    );
}

#[test]
fn category_macro_derives_catalog_presentation_and_support_status() {
    let category = definitions();
    let by_name = |name: &str| {
        category
            .entries
            .iter()
            .find(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("missing catalog entry `{name}`"))
    };

    assert_eq!(by_name("concat").detail, "concat(lists1, lists2, ...)");
    assert_eq!(
        by_name("ifs").detail,
        "ifs(condition1, value1, condition2, value2, ..., else)"
    );
    assert_eq!(
        by_name("splice").detail,
        "splice(list, startIndex, deleteCount, items1, items2, ...)"
    );
    assert_eq!(
        by_name("caseOf").signature,
        "caseOf<T, U: Variant>(subject: T, candidate1: T, result1: () -> U, candidate2: T, result2: () -> U, ..., otherwise: () -> U) -> U"
    );

    let style = by_name("style");
    assert!(!style.is_supported());
    assert_eq!(style.docs, ["`StyledText` is not represented yet."]);
    assert_eq!(
        style.signature,
        "style(text: string, styles1: string, styles2: string, ...) -> StyledText"
    );
}

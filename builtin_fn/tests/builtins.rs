use std::collections::HashSet;

use builtin_fn::{FunctionCategory, builtin_categories, builtins_functions};

#[test]
fn whole_catalog_matches_the_ordered_contract() {
    let categories = builtin_categories();
    let expected = [
        (
            FunctionCategory::General,
            &[
                "if", "ifs", "and", "or", "not", "empty", "length", "format", "equal", "unequal",
                "let", "lets",
            ][..],
        ),
        (
            FunctionCategory::Text,
            &[
                "substring",
                "contains",
                "test",
                "match",
                "replace",
                "replaceAll",
                "lower",
                "upper",
                "trim",
                "repeat",
                "padStart",
                "padEnd",
                "link",
                "style",
                "unstyle",
                "concat",
                "join",
                "split",
            ][..],
        ),
        (
            FunctionCategory::Number,
            &[
                "formatNumber",
                "add",
                "subtract",
                "multiply",
                "mod",
                "pow",
                "divide",
                "min",
                "max",
                "sum",
                "median",
                "mean",
                "abs",
                "round",
                "ceil",
                "floor",
                "sqrt",
                "cbrt",
                "exp",
                "ln",
                "log10",
                "log2",
                "sign",
                "pi",
                "e",
                "toNumber",
            ][..],
        ),
        (
            FunctionCategory::Date,
            &[
                "now",
                "today",
                "minute",
                "hour",
                "day",
                "date",
                "week",
                "month",
                "year",
                "dateAdd",
                "dateSubtract",
                "dateBetween",
                "dateRange",
                "dateStart",
                "dateEnd",
                "timestamp",
                "fromTimestamp",
                "formatDate",
                "parseDate",
            ][..],
        ),
        (FunctionCategory::People, &["name", "email"]),
        (
            FunctionCategory::List,
            &[
                "at",
                "first",
                "last",
                "slice",
                "splice",
                "sort",
                "reverse",
                "unique",
                "includes",
                "map",
                "filter",
                "find",
                "findIndex",
                "some",
                "every",
                "count",
                "flat",
            ][..],
        ),
        (FunctionCategory::Special, &["id"]),
    ];

    assert_eq!(categories.len(), expected.len());
    for (category, (expected_category, expected_names)) in categories.iter().zip(expected) {
        assert_eq!(category.category, expected_category);
        assert_eq!(
            category
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            expected_names
        );
    }

    assert_eq!(
        categories
            .iter()
            .flat_map(|category| &category.entries)
            .filter(|entry| !entry.is_supported())
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        [
            "and",
            "or",
            "not",
            "lets",
            "link",
            "style",
            "unstyle",
            "dateRange",
            "dateStart",
            "dateEnd",
            "name",
            "email",
        ]
    );

    let supported_names = categories
        .iter()
        .flat_map(|category| &category.entries)
        .filter(|entry| entry.is_supported())
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        builtins_functions()
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        supported_names,
        "executable catalog must preserve declaration order"
    );
}

#[test]
fn whole_catalog_is_self_consistent() {
    let categories = builtin_categories();

    let mut category_names = HashSet::new();
    let mut builtin_names = HashSet::new();

    for category in &categories {
        assert!(
            category_names.insert(format!("{:?}", category.category)),
            "duplicate category {:?}",
            category.category
        );
        assert!(!category.entries.is_empty(), "empty category");

        for entry in &category.entries {
            assert!(
                builtin_names.insert(entry.name.as_str()),
                "duplicate builtin `{}`",
                entry.name
            );
            assert_eq!(entry.category, category.category);
            assert!(entry.signature.starts_with(&entry.name));
            assert!(entry.detail.starts_with(&format!("{}(", entry.name)));
            if !entry.is_supported() {
                assert!(
                    !entry.docs.is_empty(),
                    "unsupported `{}` has no docs",
                    entry.name
                );
            }
        }
    }
}

#[test]
fn resolver_is_attached_only_for_flat() {
    for function in builtins_functions() {
        assert_eq!(function.resolver.is_some(), function.name == "flat");
    }
}

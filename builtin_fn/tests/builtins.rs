use std::collections::HashSet;

use builtin_fn::{FunctionCategory, builtin_categories, builtins_functions};

#[test]
fn whole_catalog_preserves_category_and_supported_order() {
    let categories = builtin_categories();
    assert_eq!(
        categories
            .iter()
            .map(|category| category.category)
            .collect::<Vec<_>>(),
        [
            FunctionCategory::General,
            FunctionCategory::Text,
            FunctionCategory::Number,
            FunctionCategory::Date,
            FunctionCategory::People,
            FunctionCategory::List,
            FunctionCategory::Special,
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

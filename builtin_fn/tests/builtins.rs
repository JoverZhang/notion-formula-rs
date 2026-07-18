use std::collections::HashSet;

use builtin_fn::{BuiltinCategory, builtin_categories, builtins_functions};

fn catalog_snapshot(categories: &[BuiltinCategory]) -> Vec<(String, Vec<(String, bool)>)> {
    categories
        .iter()
        .map(|category| {
            (
                format!("{:?}", category.category),
                category
                    .entries
                    .iter()
                    .map(|entry| (entry.name.clone(), entry.is_supported()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn whole_catalog_order_uniqueness_and_support_status_are_self_consistent() {
    let categories = builtin_categories();
    assert_eq!(
        catalog_snapshot(&categories),
        catalog_snapshot(&builtin_categories()),
        "catalog order must be deterministic"
    );

    let mut category_names = HashSet::new();
    let mut builtin_names = HashSet::new();
    let mut supported_names = Vec::new();

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
            if entry.is_supported() {
                supported_names.push(entry.name.as_str());
            } else {
                assert!(
                    !entry.docs.is_empty(),
                    "unsupported `{}` has no docs",
                    entry.name
                );
            }
        }
    }

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
fn resolver_is_attached_only_for_flat() {
    for function in builtins_functions() {
        assert_eq!(function.resolver.is_some(), function.name == "flat");
    }
}

use builtin_fn::{FunctionCategory, builtins_functions};
use std::collections::HashSet;

#[test]
fn builtin_registry_has_no_duplicate_names() {
    let mut seen = HashSet::new();
    for sig in builtins_functions() {
        assert!(
            seen.insert(sig.name.clone()),
            "duplicate builtin `{}`",
            sig.name
        );
    }
}

#[test]
fn builtin_registry_category_order_is_stable() {
    let categories = builtins_functions()
        .into_iter()
        .map(|sig| sig.category)
        .fold(Vec::<FunctionCategory>::new(), |mut acc, category| {
            if acc.last().copied() != Some(category) {
                acc.push(category);
            }
            acc
        });

    assert_eq!(
        categories,
        vec![
            FunctionCategory::General,
            FunctionCategory::Text,
            FunctionCategory::Number,
            FunctionCategory::Date,
            FunctionCategory::People,
            FunctionCategory::List,
            FunctionCategory::Special,
        ]
    );
}

#[test]
fn resolver_is_attached_only_for_flat() {
    for sig in builtins_functions() {
        if sig.name == "flat" {
            assert!(sig.resolver.is_some(), "flat should have a resolver");
        } else {
            assert!(
                sig.resolver.is_none(),
                "{} should not have a resolver",
                sig.name
            );
        }
    }
}

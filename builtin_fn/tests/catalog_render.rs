use builtin_fn::{builtin_categories, render_builtin_catalog, render_builtin_readme};

#[test]
fn catalog_renderer_is_deterministic_and_preserves_surrounding_text() {
    let categories = builtin_categories();
    let first = render_builtin_catalog(&categories);
    let second = render_builtin_catalog(&categories);
    assert_eq!(first, second);
    assert!(first.contains("concat<T>(lists1: T[], lists2: T[], ...) -> T[]"));
    assert!(first.contains(
        "// Unsupported: Precise sequential binder typing requires a heterogeneous binder-pack model.\nlets("
    ));

    let source = "before\n<!-- BEGIN GENERATED BUILTIN CATALOG -->\nstale\n<!-- END GENERATED BUILTIN CATALOG -->\nafter\n";
    let rendered = render_builtin_readme(source, &categories).expect("marked region");
    assert!(rendered.starts_with("before\n<!-- BEGIN GENERATED BUILTIN CATALOG -->\n\n"));
    assert!(rendered.ends_with("<!-- END GENERATED BUILTIN CATALOG -->\nafter\n"));
    assert!(!rendered.contains("stale"));
}

#[test]
fn committed_readme_catalog_is_current() {
    let current = include_str!("../../docs/builtin_functions/README.md");
    let rendered = render_builtin_readme(current, &builtin_categories()).expect("marked region");
    assert_eq!(rendered, current);
}

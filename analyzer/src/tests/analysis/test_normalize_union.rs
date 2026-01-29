use crate::semantic::{Ty, normalize_union};

#[test]
fn normalize_union_is_deterministic() {
    let a = normalize_union([Ty::String, Ty::Number]);
    let b = normalize_union([Ty::Number, Ty::String]);
    assert_eq!(a, b);
    assert_eq!(a, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn normalize_union_flattens_and_dedupes() {
    let ty = normalize_union([
        Ty::Union(vec![Ty::String, Ty::Number]),
        Ty::String,
        Ty::Union(vec![Ty::Number]),
    ]);
    assert_eq!(ty, Ty::Union(vec![Ty::Number, Ty::String]));
}

#[test]
fn normalize_union_collapses_single_member() {
    assert_eq!(normalize_union([Ty::String]), Ty::String);
}

#[test]
fn normalize_union_empty_is_unknown() {
    assert_eq!(normalize_union(std::iter::empty()), Ty::Unknown);
}

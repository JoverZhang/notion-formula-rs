use super::super::{normalize_union, FunctionCategory, FunctionSig, GenericId, LambdaParam, Ty};

/// Custom resolver for `flat(list)`.
///
/// Deep-flattens all nesting levels (like JS `Array.flat(Infinity)`).
/// Recursively collects all non-List leaf types and returns `List(union_of_leaves)`.
///
/// Examples:
/// - `flat(number[][])` -> `number[]`
/// - `flat(number[][][])` -> `number[]`
/// - `flat(number[])` -> `number[]` (already flat)
/// - `flat((number | string[])[])` -> `(number | string)[]`
/// - `flat(unknown[])` -> `unknown[]` (fallback)
fn resolve_flat(sig: &FunctionSig, arg_tys: &[Ty]) -> FunctionSig {
    let ret = match arg_tys.first() {
        Some(Ty::List(inner)) => {
            let mut leaves = Vec::new();
            collect_leaf_types(inner, &mut leaves);
            Ty::List(Box::new(normalize_union(leaves)))
        }
        _ => Ty::List(Box::new(Ty::Unknown)), // non-list arg, fallback
    };

    FunctionSig { ret, ..sig.clone() }
}

/// Recursively collect all non-List leaf types from a type tree.
///
/// - `List(T)` → recurse into `T`
/// - `Union([A, B])` → recurse into each member
/// - anything else → leaf, push to `out`
fn collect_leaf_types(ty: &Ty, out: &mut Vec<Ty>) {
    match ty {
        Ty::List(inner) => collect_leaf_types(inner, out),
        Ty::Union(members) => {
            for m in members {
                collect_leaf_types(m, out);
            }
        }
        other => out.push(other.clone()),
    }
}

pub(super) fn builtins() -> Vec<FunctionSig> {
    let t0 = GenericId(0);
    let t1 = GenericId(1);
    vec![
        func_g!(
            FunctionCategory::List,
            "at(list, index)",
            generics!(g!(0, Plain)),
            "at",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("index", Ty::Number)
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "first(list)",
            generics!(g!(0, Plain)),
            "first",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "last(list)",
            generics!(g!(0, Plain)),
            "last",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "slice(list, start, end?)",
            generics!(g!(0, Plain)),
            "slice",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("start", Ty::Number),
                opt!("end", Ty::Number)
            ),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "splice(list, startIndex, deleteCount, ...items)",
            generics!(g!(0, Plain)),
            "splice",
            repeat_params!(
                head!(
                    p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                    p!("startIndex", Ty::Number),
                    p!("deleteCount", Ty::Number),
                ),
                repeat!(p!("items", Ty::Generic(t0))),
                tail!(),
            )
            .with_repeat_min_groups(0),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "sort(list)",
            generics!(g!(0, Plain)),
            "sort",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "reverse(list)",
            generics!(g!(0, Plain)),
            "reverse",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "unique(list)",
            generics!(g!(0, Plain)),
            "unique",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "includes(list, value)",
            generics!(g!(0, Plain)),
            "includes",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!("value", Ty::Generic(t0))
            ),
            Ty::Boolean,
        ),
        // Lambda-taking list builtins. Each uses LambdaParam::Current to bind the
        // iteration variable `current` with type T0 (the list element type).
        func_g!(
            FunctionCategory::List,
            "map(list, mapper)",
            generics!(g!(0, Plain), g!(1, Plain)),
            "map",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "mapper",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Generic(t1)),
                    }
                )
            ),
            Ty::List(Box::new(Ty::Generic(t1))),
        ),
        func_g!(
            FunctionCategory::List,
            "filter(list, predicate)",
            generics!(g!(0, Plain)),
            "filter",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::List(Box::new(Ty::Generic(t0))),
        ),
        func_g!(
            FunctionCategory::List,
            "find(list, predicate)",
            generics!(g!(0, Plain)),
            "find",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::Generic(t0),
        ),
        func_g!(
            FunctionCategory::List,
            "findIndex(list, predicate)",
            generics!(g!(0, Plain)),
            "findIndex",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::Number,
        ),
        func_g!(
            FunctionCategory::List,
            "some(list, predicate)",
            generics!(g!(0, Plain)),
            "some",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::List,
            "every(list, predicate)",
            generics!(g!(0, Plain)),
            "every",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::Boolean,
        ),
        func_g!(
            FunctionCategory::List,
            "count(list, predicate)",
            generics!(g!(0, Plain)),
            "count",
            params!(
                p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
                p!(
                    "predicate",
                    Ty::Fn {
                        params: vec![(LambdaParam::Current, Ty::Generic(t0))],
                        ret: Box::new(Ty::Boolean),
                    }
                )
            ),
            Ty::Number,
        ),
        func_gr!(
            FunctionCategory::List,
            "flat(list)",
            generics!(g!(0, Plain)),
            "flat",
            params!(p!("list", Ty::List(Box::new(Ty::Generic(t0))))),
            Ty::List(Box::new(Ty::Generic(t0))),
            resolve_flat,
        ),
    ]
}

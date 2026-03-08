use super::super::{normalize_union, FunctionCategory, FunctionSig, GenericId, Ty};

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
        // TODO(lambda-typing): Intentionally removed until we have a real lambda/function type system.
        // NOTE: Notion’s predicate/mapper DSL may include (current, index) etc.; keep minimal forms here.
        // TODO(lambda-typing): find<T>(list: T[], predicate: (current) -> boolean) -> T
        // TODO(lambda-typing): findIndex<T>(list: T[], predicate: (current) -> boolean) -> number
        // TODO(lambda-typing): filter<T>(list: T[], predicate: (current) -> boolean) -> T[]
        // TODO(lambda-typing): some<T>(list: T[], predicate: (current) -> boolean) -> boolean
        // TODO(lambda-typing): every<T>(list: T[], predicate: (current) -> boolean) -> boolean
        // TODO(lambda-typing): map<T, U>(list: T[], mapper: (current) -> U) -> U[]
        // TODO(lambda-typing): count<T>(list: T[], predicate: (current) -> boolean) -> number
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

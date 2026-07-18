use crate::{
    ArgumentObservation, BuiltinCategory, ResolverInput, Ty, builtin_functions, normalize_union,
};

fn resolve_flat(input: &ResolverInput<'_>) -> Ty {
    match input.arguments.first() {
        Some(ArgumentObservation::Typed(Ty::List(element))) => {
            let mut leaves = Vec::new();
            collect_leaf_types(element, &mut leaves);
            Ty::List(Box::new(normalize_union(leaves)))
        }
        _ => input.default_return_ty.clone(),
    }
}

fn collect_leaf_types(ty: &Ty, out: &mut Vec<Ty>) {
    match ty {
        Ty::List(inner) => collect_leaf_types(inner, out),
        Ty::Union(members) => {
            for member in members {
                collect_leaf_types(member, out);
            }
        }
        other => out.push(other.clone()),
    }
}

pub(super) fn definitions() -> BuiltinCategory {
    builtin_functions! {
        category: List;

        at<T>(list: T[], index: number) -> T;
        first<T>(list: T[]) -> T;
        last<T>(list: T[]) -> T;
        slice<T>(list: T[], start: number, end?: number) -> T[];

        splice<T>(
            list: T[],
            startIndex: number,
            deleteCount: number,
            repeat(min = 0) {
                items: T,
            },
        ) -> T[];

        sort<T>(list: T[]) -> T[];
        reverse<T>(list: T[]) -> T[];
        unique<T>(list: T[]) -> T[];
        includes<T>(list: T[], value: T) -> boolean;
        map<T, U>(list: T[], mapper: (current: T) -> U) -> U[];
        filter<T>(list: T[], predicate: (current: T) -> boolean) -> T[];
        find<T>(list: T[], predicate: (current: T) -> boolean) -> T;
        findIndex<T>(list: T[], predicate: (current: T) -> boolean) -> number;
        some<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
        every<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
        count<T>(list: T[], predicate: (current: T) -> boolean) -> number;

        #[resolver(resolve_flat)]
        flat<T>(list: T[]) -> T[];
    }
}

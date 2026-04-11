use super::{sig, sig_with_resolver};
use crate::{BuiltinSigParser, FunctionCategory, FunctionSig, Ty, normalize_union};

fn resolve_flat(sig: &FunctionSig, arg_tys: &[Ty]) -> FunctionSig {
    let ret = match arg_tys.first() {
        Some(Ty::List(inner)) => {
            let mut leaves = Vec::new();
            collect_leaf_types(inner, &mut leaves);
            Ty::List(Box::new(normalize_union(leaves)))
        }
        _ => Ty::List(Box::new(Ty::Unknown)),
    };

    FunctionSig { ret, ..sig.clone() }
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

pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(
            parser,
            FunctionCategory::List,
            "at<T: Plain>(list: T[], index: number) -> T",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "first<T: Plain>(list: T[]) -> T",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "last<T: Plain>(list: T[]) -> T",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "slice<T: Plain>(list: T[], start: number, end?: number) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "splice<T: Plain>(list: T[], startIndex: number, deleteCount: number, ...items: T[]) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "sort<T: Plain>(list: T[]) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "reverse<T: Plain>(list: T[]) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "unique<T: Plain>(list: T[]) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "includes<T: Plain>(list: T[], value: T) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "map<T: Plain, U: Plain>(list: T[], mapper: (current: T) -> U) -> U[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "filter<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> T[]",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "find<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> T",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "findIndex<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> number",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "some<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "every<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> boolean",
        ),
        sig(
            parser,
            FunctionCategory::List,
            "count<T: Plain>(list: T[], predicate: (current: T) -> boolean) -> number",
        ),
        sig_with_resolver(
            parser,
            FunctionCategory::List,
            "flat<T: Plain>(list: T[]) -> T[]",
            resolve_flat,
        ),
    ]
}

//! Helpers for deterministic union normalization.

use super::Ty;

/// Normalize a union-like set of types into a deterministic [`Ty`].
///
/// Currently this:
/// - flattens nested `Union` members,
/// - deduplicates members,
/// - sorts members deterministically (so outputs are stable across runs),
/// - returns the single member directly when only one remains,
/// - returns [`Ty::Unknown`] when given an empty iterator.
pub fn normalize_union(members: impl IntoIterator<Item = Ty>) -> Ty {
    normalize_union_impl(members).unwrap_or(Ty::Unknown)
}

fn normalize_union_impl(members: impl IntoIterator<Item = Ty>) -> Option<Ty> {
    let mut flat = Vec::<Ty>::new();
    for member in members {
        push_flattened_union_member(&mut flat, member);
    }

    // Deduplicate while preserving (later) deterministic ordering.
    let mut unique = Vec::<Ty>::new();
    for ty in flat {
        if !unique.contains(&ty) {
            unique.push(ty);
        }
    }

    unique.sort_by_key(ty_sort_key);

    match unique.len() {
        0 => None,
        1 => Some(unique.remove(0)),
        _ => Some(Ty::Union(unique)),
    }
}

fn push_flattened_union_member(out: &mut Vec<Ty>, ty: Ty) {
    match ty {
        Ty::Union(members) => {
            for member in members {
                push_flattened_union_member(out, member);
            }
        }
        other => out.push(other),
    }
}

fn ty_sort_key(ty: &Ty) -> (u8, String) {
    match ty {
        Ty::Null => (0, "null".into()),
        Ty::Boolean => (1, "boolean".into()),
        Ty::Number => (2, "number".into()),
        Ty::String => (3, "string".into()),
        Ty::Date => (4, "date".into()),
        Ty::List(inner) => (5, format!("list<{}>", ty_sort_key(inner).1)),
        Ty::Generic(id) => (6, format!("T{}", id.0)),
        // By the time we sort, unions should already be flattened.
        Ty::Union(_) => (7, "union".into()),
        Ty::Unknown => (8, "unknown".into()),
    }
}

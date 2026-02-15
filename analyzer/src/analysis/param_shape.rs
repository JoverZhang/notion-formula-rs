//! Resolve repeat/tail mapping for [`ParamShape`].
//!
//! This splits a call's `total` args into `head + repeat_groups * repeat + tail_used`.
//! Used by signature help and arity checks. If more than one split fits, it picks the largest
//! `tail_used`.

use super::{ParamShape, ParamSig};

/// Resolve `tail_used` for `total` args.
///
/// Returns `None` if `total` cannot fit the repeat shape, or if there is no repeat section.
/// If more than one split fits, it prefers the largest `tail_used`.
pub(crate) fn resolve_repeat_tail_used(params: &ParamShape, total: usize) -> Option<usize> {
    resolve_repeat_tail_used_with_min_groups(params, total, 1)
}

fn resolve_repeat_tail_used_with_min_groups(
    params: &ParamShape,
    total: usize,
    repeat_min_groups: usize,
) -> Option<usize> {
    if params.repeat.is_empty() {
        return Some(params.tail.len());
    }

    let head_len = params.head.len();
    if total < head_len {
        return None;
    }

    let repeat_len = params.repeat.len();
    let tail_min = required_tail_prefix_len(&params.tail);
    let min_middle = repeat_len.saturating_mul(repeat_min_groups);

    for tail_used in (tail_min..=params.tail.len()).rev() {
        if total < head_len + tail_used {
            continue;
        }
        let middle = total - head_len - tail_used;
        if middle >= min_middle && middle.is_multiple_of(repeat_len) {
            return Some(tail_used);
        }
    }

    None
}

fn required_tail_prefix_len(tail: &[ParamSig]) -> usize {
    let mut required = 0usize;
    for (idx, p) in tail.iter().enumerate() {
        if !p.optional {
            required = idx + 1;
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Ty;

    fn p(name: &str, optional: bool) -> ParamSig {
        ParamSig {
            name: name.to_string(),
            ty: Ty::Unknown,
            optional,
        }
    }

    #[test]
    fn resolve_repeat_tail_used_prefers_largest_tail_used_when_ambiguous() {
        // NOTE: This shape violates `ParamShape::new` invariants (repeat + optional tail).
        // We still test the resolver's deterministic choice rule to prevent future drift.
        let params = ParamShape {
            head: vec![],
            repeat: vec![p("x", false), p("y", false)],
            tail: vec![p("t1", true), p("t2", true)],
        };

        // total=4 can be:
        // - tail_used=2, middle=2 (1 repeat group)
        // - tail_used=0, middle=4 (2 repeat groups)
        // Prefer the larger tail_used.
        assert_eq!(resolve_repeat_tail_used(&params, 4), Some(2));
    }
}

//! Resolve repeat/tail mapping for [`ParamShape`].
//!
//! This splits a call's `total` args into `head + repeat_groups * repeat + tail_used`.
//! Used by signature help and arity checks. If more than one split fits, it picks the largest
//! `tail_used`.

use super::{ParamShape, ParamSig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletedRepeatShape {
    /// The total arg count (may be increased to reach the next valid shape).
    pub(crate) total: usize,
    /// How many args at the end are assigned to `tail`.
    pub(crate) tail_used: usize,
    /// Tail starts at `tail_start`, so tail args are `[tail_start, total)`.
    pub(crate) tail_start: usize,
    /// How many repeat groups were used.
    pub(crate) repeat_groups: usize,
}

/// Resolve `tail_used` for `total` args.
///
/// Returns `None` if `total` cannot fit the repeat shape, or if there is no repeat section.
/// If more than one split fits, it prefers the largest `tail_used`.
pub(crate) fn resolve_repeat_tail_used(params: &ParamShape, total: usize) -> Option<usize> {
    resolve_repeat_tail_used_with_min_groups(params, total, 1)
}

/// Return a parseable repeat shape, bumping `total` up when needed.
///
/// If `total` does not fit, this picks the smallest valid `total >= total` (then the largest
/// `tail_used` on ties).
pub(crate) fn complete_repeat_shape(
    params: &ParamShape,
    total: usize,
) -> Option<CompletedRepeatShape> {
    complete_repeat_shape_with_min_groups(params, total, 1)
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

fn complete_repeat_shape_with_min_groups(
    params: &ParamShape,
    total: usize,
    repeat_min_groups: usize,
) -> Option<CompletedRepeatShape> {
    if params.repeat.is_empty() {
        return None;
    }

    let head_len = params.head.len();
    let repeat_len = params.repeat.len();
    if repeat_len == 0 {
        return None;
    }

    // If already parseable, keep `total` as-is.
    if let Some(tail_used) =
        resolve_repeat_tail_used_with_min_groups(params, total, repeat_min_groups)
    {
        let tail_start = total.saturating_sub(tail_used);
        let middle = total.saturating_sub(head_len + tail_used);
        let repeat_groups = middle / repeat_len;
        return Some(CompletedRepeatShape {
            total,
            tail_used,
            tail_start,
            repeat_groups,
        });
    }

    let tail_min = required_tail_prefix_len(&params.tail);
    let min_middle = repeat_len.saturating_mul(repeat_min_groups);

    let mut best: Option<(usize /* total */, usize /* tail_used */)> = None;
    for tail_used in tail_min..=params.tail.len() {
        // Minimum total to even have head + tail_used.
        let min_total_for_tail = head_len.saturating_add(tail_used);
        // Minimum total to satisfy repeat_min_groups.
        let min_total_for_middle = head_len
            .saturating_add(tail_used)
            .saturating_add(min_middle);

        let base_total = total.max(min_total_for_tail).max(min_total_for_middle);
        let middle_base = base_total - head_len - tail_used;
        let middle = ceil_to_multiple(middle_base, repeat_len);
        let completed_total = head_len + tail_used + middle;

        match best {
            None => best = Some((completed_total, tail_used)),
            Some((best_total, best_tail_used)) => {
                if completed_total < best_total
                    || (completed_total == best_total && tail_used > best_tail_used)
                {
                    best = Some((completed_total, tail_used));
                }
            }
        }
    }

    let (completed_total, tail_used) = best?;
    let tail_start = completed_total - tail_used;
    let middle = completed_total - head_len - tail_used;
    let repeat_groups = middle / repeat_len;
    Some(CompletedRepeatShape {
        total: completed_total,
        tail_used,
        tail_start,
        repeat_groups,
    })
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

fn ceil_to_multiple(n: usize, m: usize) -> usize {
    if m == 0 {
        return n;
    }
    if n == 0 {
        return 0;
    }
    let rem = n % m;
    if rem == 0 { n } else { n + (m - rem) }
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

    #[test]
    fn complete_repeat_shape_bumps_total_to_next_valid_multiple() {
        let params = ParamShape::new(vec![], vec![p("x", false), p("y", false)], vec![]);

        // total=3 cannot be split into 2-wide repeat groups with at least 1 group.
        // The next valid total is 4.
        let shape = complete_repeat_shape(&params, 3).expect("expected completion shape");
        assert_eq!(shape.total, 4);
        assert_eq!(shape.tail_used, 0);
        assert_eq!(shape.tail_start, 4);
        assert_eq!(shape.repeat_groups, 2);
    }
}

//! Resolve repeat/tail mapping for [`ParamShape`].

use crate::{ParamShape, ParamSig};

/// Resolve `tail_used` for `total` args.
///
/// Returns `None` if `total` cannot fit the repeat shape.
/// If more than one split fits, it prefers the largest `tail_used`.
pub fn resolve_repeat_tail_used(params: &ParamShape, total: usize) -> Option<usize> {
    resolve_repeat_tail_used_with_min_groups(params, total, params.repeat_min_groups)
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
    for (idx, param) in tail.iter().enumerate() {
        if !param.optional {
            required = idx + 1;
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ty;

    fn p(name: &str, optional: bool) -> ParamSig {
        ParamSig {
            name: name.to_string(),
            ty: Ty::Unknown,
            optional,
        }
    }

    #[test]
    fn resolve_repeat_tail_used_prefers_largest_tail_used_when_ambiguous() {
        let params = ParamShape {
            head: vec![],
            repeat: vec![p("x", false), p("y", false)],
            tail: vec![p("t1", true), p("t2", true)],
            repeat_min_groups: 1,
        };

        assert_eq!(resolve_repeat_tail_used(&params, 4), Some(2));
    }
}

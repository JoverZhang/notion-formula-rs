use super::{ParamShape, ParamSig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletedRepeatShape {
    /// The (possibly increased) total argument count used to make the shape parseable.
    pub(crate) total: usize,
    pub(crate) tail_used: usize,
    pub(crate) tail_start: usize,
    pub(crate) repeat_groups: usize,
}

pub(crate) fn resolve_repeat_tail_used(params: &ParamShape, total: usize) -> Option<usize> {
    resolve_repeat_tail_used_with_min_groups(params, total, 1)
}

pub(crate) fn complete_repeat_shape(params: &ParamShape, total: usize) -> Option<CompletedRepeatShape> {
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
    if let Some(tail_used) = resolve_repeat_tail_used_with_min_groups(params, total, repeat_min_groups) {
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
    if rem == 0 {
        n
    } else {
        n + (m - rem)
    }
}


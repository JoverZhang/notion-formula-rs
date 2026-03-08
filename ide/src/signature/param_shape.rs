//! Repeat-parameter shape resolution and active-parameter index mapping.
//!
//! Functions with variadic (`repeat`) parameters need special logic to
//! determine how many repeat groups are present and which tail parameters
//! are in use, given the total number of arguments at the call site.

use analyzer::semantic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CompletedRepeatShape {
    pub(super) tail_start: usize,
    pub(super) repeat_groups: usize,
}

fn required_tail_prefix_len(tail: &[semantic::ParamSig]) -> usize {
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

fn resolve_repeat_tail_used_with_min_groups(
    params: &semantic::ParamShape,
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

pub(super) fn complete_repeat_shape(
    params: &semantic::ParamShape,
    total: usize,
) -> Option<CompletedRepeatShape> {
    const REPEAT_MIN_GROUPS: usize = 1;

    if params.repeat.is_empty() {
        return None;
    }

    let head_len = params.head.len();
    let repeat_len = params.repeat.len();
    if repeat_len == 0 {
        return None;
    }

    if let Some(tail_used) =
        resolve_repeat_tail_used_with_min_groups(params, total, REPEAT_MIN_GROUPS)
    {
        let tail_start = total.saturating_sub(tail_used);
        let middle = total.saturating_sub(head_len + tail_used);
        let repeat_groups = middle / repeat_len;
        return Some(CompletedRepeatShape {
            tail_start,
            repeat_groups,
        });
    }

    let tail_min = required_tail_prefix_len(&params.tail);
    let min_middle = repeat_len.saturating_mul(REPEAT_MIN_GROUPS);

    let mut best: Option<(usize /* total */, usize /* tail_used */)> = None;
    for tail_used in tail_min..=params.tail.len() {
        let min_total_for_tail = head_len.saturating_add(tail_used);
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
        tail_start,
        repeat_groups,
    })
}

pub(super) fn active_parameter_for_call(
    sig: &semantic::FunctionSig,
    arg_index_full: usize,
    total_args_for_shape: usize,
    is_method_style: bool,
) -> usize {
    let idx = if sig.params.repeat.is_empty() {
        let total_params = sig.params.head.len() + sig.params.tail.len();
        if total_params == 0 {
            0
        } else {
            arg_index_full.min(total_params - 1)
        }
    } else {
        let head_len = sig.params.head.len();
        let repeat_len = sig.params.repeat.len();
        let tail_len = sig.params.tail.len();

        if repeat_len == 0 {
            return 0;
        }

        let Some(shape) = complete_repeat_shape(&sig.params, total_args_for_shape) else {
            return 0;
        };

        if arg_index_full < head_len {
            arg_index_full
        } else if arg_index_full >= shape.tail_start {
            let tail_idx = arg_index_full.saturating_sub(shape.tail_start);
            let max_tail = tail_len.saturating_sub(1);
            let tail_idx = tail_idx.min(max_tail);
            head_len + repeat_len * shape.repeat_groups + tail_idx
        } else {
            let idx_in_repeat = arg_index_full.saturating_sub(head_len);
            let repeat_pos = idx_in_repeat % repeat_len;

            let cycle = idx_in_repeat / repeat_len;
            head_len + cycle * repeat_len + repeat_pos
        }
    };

    if is_method_style {
        idx.saturating_sub(1)
    } else {
        idx
    }
}

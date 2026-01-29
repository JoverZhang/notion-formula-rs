use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FuzzyScore {
    pub(super) is_prefix: bool,
    pub(super) gap_sum: usize,
    pub(super) max_run: usize,
    pub(super) first_pos: usize,
    pub(super) label_len: usize,
}

pub(super) fn fuzzy_score(query: &str, label: &str) -> Option<FuzzyScore> {
    let query_chars: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
    if query_chars.is_empty() {
        return None;
    }

    let label_chars: Vec<char> = label.chars().map(|c| c.to_ascii_lowercase()).collect();
    if label_chars.is_empty() {
        return None;
    }

    let mut positions = Vec::with_capacity(query_chars.len());
    let mut j = 0usize;
    for &qc in &query_chars {
        while j < label_chars.len() && label_chars[j] != qc {
            j += 1;
        }
        if j == label_chars.len() {
            return None;
        }
        positions.push(j);
        j += 1;
    }

    let first_pos = *positions.first().unwrap_or(&0);
    let label_len = label_chars.len();

    let mut gap_sum = 0usize;
    let mut max_run = 1usize;
    let mut current_run = 1usize;
    for window in positions.windows(2) {
        let prev = window[0];
        let next = window[1];
        if next == prev + 1 {
            current_run += 1;
            max_run = usize::max(max_run, current_run);
        } else {
            current_run = 1;
            gap_sum = gap_sum.saturating_add(next.saturating_sub(prev).saturating_sub(1));
        }
    }

    let label_lower: String = label_chars.iter().collect();
    let query_lower: String = query_chars.iter().collect();
    let is_prefix = label_lower.starts_with(&query_lower);

    Some(FuzzyScore {
        is_prefix,
        gap_sum,
        max_run,
        first_pos,
        label_len,
    })
}

pub(super) fn fuzzy_score_cmp(a: FuzzyScore, b: FuzzyScore) -> Ordering {
    b.is_prefix
        .cmp(&a.is_prefix)
        .then_with(|| a.gap_sum.cmp(&b.gap_sum))
        .then_with(|| b.max_run.cmp(&a.max_run))
        .then_with(|| a.first_pos.cmp(&b.first_pos))
        .then_with(|| a.label_len.cmp(&b.label_len))
}

pub(super) fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

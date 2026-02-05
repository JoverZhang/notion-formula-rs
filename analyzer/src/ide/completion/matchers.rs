//! Query normalization and fuzzy matching for completion labels.
//!
//! Core idea: subsequence match (not substring).
//! - `query` matches `label` if every query character appears in `label` in order.
//! - Matching is ASCII-case-insensitive.
//!
//! The score is not a single number; it is a set of heuristics used for ranking.
//! See `fuzzy_score_cmp` for the exact priority order.

use std::cmp::Ordering;

/// Heuristic metrics for ranking a subsequence match.
///
/// Interpretation (better = ranks earlier):
/// - `is_prefix`: `label` starts with `query` (strong signal).
/// - `gap_sum`: total number of skipped characters between matched characters
///   (smaller is better).
/// - `max_run`: length of the longest consecutive run of matched characters
///   (larger is better; prefers contiguous-looking matches).
/// - `first_pos`: index of the first matched character in `label` (smaller is better).
/// - `label_len`: total length of `label` in chars (smaller is a mild tie-breaker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FuzzyScore {
    pub(super) is_prefix: bool,
    pub(super) gap_sum: usize,
    pub(super) max_run: usize,
    pub(super) first_pos: usize,
    pub(super) label_len: usize,
}

/// Computes a fuzzy match score for `query` against `label`.
///
/// Matching:
/// - ASCII-case-insensitive.
/// - Subsequence match: `query` chars must appear in `label` in order (not necessarily contiguous).
///
/// If no match exists, returns `None`.
///
/// Scoring details:
/// - We record the matched character positions in `label`.
/// - From these positions we derive `gap_sum`, `max_run`, and `first_pos`.
/// - `is_prefix` is computed as a separate strong signal.
///
/// NOTE: This function currently does NOT ignore '_'.
/// If callers want '_' to be insignificant, they should normalize inputs first
/// (e.g., via `normalize_for_match`) and/or define how positions should be interpreted.
pub(super) fn fuzzy_score(query: &str, label: &str) -> Option<FuzzyScore> {
    // Lowercase ASCII for stable matching.
    let query_chars: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
    if query_chars.is_empty() {
        return None;
    }

    let label_chars: Vec<char> = label.chars().map(|c| c.to_ascii_lowercase()).collect();
    if label_chars.is_empty() {
        return None;
    }

    // Greedy subsequence match:
    // For each query char, find the earliest next occurrence in label after the previous match.
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

    // Derive "compactness" signals from matched positions.
    // - `gap_sum`: total skipped chars between matches (prefers compact matches).
    // - `max_run`: longest consecutive run (prefers contiguous-looking matches).
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

    // Strong signal: exact prefix match (case-insensitive).
    // (This is computed separately from subsequence match.)
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

/// Orders fuzzy scores from best to worst.
///
/// Priority (earlier is better):
/// 1) prefix match
/// 2) smaller gaps between matched chars
/// 3) larger contiguous run
/// 4) earlier first match position
/// 5) shorter label (tie-break)
pub(super) fn fuzzy_score_cmp(a: FuzzyScore, b: FuzzyScore) -> Ordering {
    b.is_prefix
        .cmp(&a.is_prefix)
        .then_with(|| a.gap_sum.cmp(&b.gap_sum))
        .then_with(|| b.max_run.cmp(&a.max_run))
        .then_with(|| a.first_pos.cmp(&b.first_pos))
        .then_with(|| a.label_len.cmp(&b.label_len))
}

/// Normalizes a label/query for matching (lowercases ASCII and removes `_`).
pub(super) fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

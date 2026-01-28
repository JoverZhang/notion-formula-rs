use std::cmp::Ordering;

use super::{CompletionItem, CompletionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FuzzyScore {
    is_prefix: bool,
    gap_sum: usize,
    max_run: usize,
    first_pos: usize,
    label_len: usize,
}

fn fuzzy_score(query: &str, label: &str) -> Option<FuzzyScore> {
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

fn fuzzy_score_cmp(a: FuzzyScore, b: FuzzyScore) -> Ordering {
    b.is_prefix
        .cmp(&a.is_prefix)
        .then_with(|| a.gap_sum.cmp(&b.gap_sum))
        .then_with(|| b.max_run.cmp(&a.max_run))
        .then_with(|| a.first_pos.cmp(&b.first_pos))
        .then_with(|| a.label_len.cmp(&b.label_len))
}

fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn kind_priority(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::Function => 0,
        CompletionKind::Builtin => 1,
        CompletionKind::Property => 2,
        CompletionKind::Operator => 3,
    }
}

pub(super) fn fuzzy_rank_items(query: &str, items: &mut Vec<CompletionItem>) {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MatchClass {
        Exact,
        Contains { pos: usize },
        Fuzzy(FuzzyScore),
        None,
    }

    fn match_class_for_item(query_norm: &str, item: &CompletionItem) -> MatchClass {
        if !matches!(
            item.kind,
            CompletionKind::Function | CompletionKind::Property
        ) {
            return MatchClass::None;
        }

        let label_norm = normalize_for_match(&item.label);
        if label_norm == query_norm {
            return MatchClass::Exact;
        }
        if let Some(pos) = label_norm.find(query_norm) {
            return MatchClass::Contains { pos };
        }
        if let Some(score) = fuzzy_score(query_norm, &label_norm) {
            return MatchClass::Fuzzy(score);
        }
        MatchClass::None
    }

    fn match_class_rank(class: MatchClass) -> u8 {
        match class {
            MatchClass::Exact => 0,
            MatchClass::Contains { .. } => 1,
            MatchClass::Fuzzy(_) => 2,
            MatchClass::None => 3,
        }
    }

    #[derive(Debug)]
    struct Ranked {
        original_idx: usize,
        label_norm_len: usize,
        class: MatchClass,
        item: CompletionItem,
    }

    let query_norm = normalize_for_match(query);

    let mut ranked: Vec<Ranked> = items
        .drain(..)
        .enumerate()
        .map(|(idx, item)| {
            let label_norm_len = normalize_for_match(&item.label).chars().count();
            let class = match_class_for_item(&query_norm, &item);
            Ranked {
                original_idx: idx,
                label_norm_len,
                class,
                item,
            }
        })
        .collect();

    ranked.sort_by(|a, b| {
        let ar = match_class_rank(a.class);
        let br = match_class_rank(b.class);
        ar.cmp(&br).then_with(|| match (a.class, b.class) {
            (MatchClass::Exact, MatchClass::Exact) => a
                .label_norm_len
                .cmp(&b.label_norm_len)
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::Contains { pos: ap }, MatchClass::Contains { pos: bp }) => a
                .label_norm_len
                .cmp(&b.label_norm_len)
                .then_with(|| ap.cmp(&bp))
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::Fuzzy(sa), MatchClass::Fuzzy(sb)) => fuzzy_score_cmp(sa, sb)
                .then_with(|| kind_priority(a.item.kind).cmp(&kind_priority(b.item.kind)))
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::None, MatchClass::None) => a.original_idx.cmp(&b.original_idx),
            _ => a.original_idx.cmp(&b.original_idx),
        })
    });

    *items = ranked.into_iter().map(|r| r.item).collect();
}

pub(super) fn preferred_indices_for_items(
    items: &[CompletionItem],
    query: &str,
    preferred_limit: usize,
) -> Vec<usize> {
    if preferred_limit == 0 {
        return Vec::new();
    }
    let query_norm = normalize_for_match(query);

    fn matches_query(query_norm: &str, item: &CompletionItem) -> bool {
        if !matches!(
            item.kind,
            CompletionKind::Function | CompletionKind::Property
        ) {
            return false;
        }
        let label_norm = normalize_for_match(&item.label);
        label_norm == query_norm
            || label_norm.contains(query_norm)
            || fuzzy_score(query_norm, &label_norm).is_some()
    }

    let mut out = Vec::with_capacity(preferred_limit);
    for (idx, item) in items.iter().enumerate() {
        if out.len() >= preferred_limit {
            break;
        }
        if item.is_disabled {
            continue;
        }
        if matches_query(&query_norm, item) {
            out.push(idx);
        }
    }
    out
}

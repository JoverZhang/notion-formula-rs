//! Ranks and post-processes completion items.
//! Ranking uses an ASCII-ish normalized query (lowercased; `_` removed).
//! Spans/cursors are UTF-8 byte offsets; ranges are half-open `[start, end)`.

use super::matchers::{FuzzyScore, fuzzy_score, fuzzy_score_cmp, normalize_for_match};
use super::position::PositionKind;
use super::{
    CompletionConfig, CompletionData, CompletionItem, CompletionKind, CompletionOutput, TextEdit,
};
use crate::lexer::Span;
use crate::semantic;

fn kind_priority(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::Function => 0,
        CompletionKind::Builtin => 1,
        CompletionKind::Property => 2,
        CompletionKind::Operator => 3,
    }
}

pub(super) fn finalize_output(
    text: &str,
    mut output: CompletionOutput,
    config: CompletionConfig,
    position_kind: PositionKind,
) -> CompletionOutput {
    attach_primary_edits(output.replace, &mut output.items);

    let Some(query) = completion_query_for_replace(text, output.replace) else {
        output.preferred_indices = Vec::new();
        return output;
    };

    if matches!(position_kind, PositionKind::AfterDot) {
        fuzzy_filter_and_rank_postfix_items(&query, &mut output.items);
    } else {
        fuzzy_rank_items(&query, &mut output.items);
    }
    output.preferred_indices =
        preferred_indices_for_items(&output.items, &query, config.preferred_limit);
    output
}

fn completion_query_for_replace(text: &str, replace: Span) -> Option<String> {
    if replace.start == replace.end {
        return None;
    }

    let start = usize::try_from(u32::min(replace.start, replace.end)).ok()?;
    let end = usize::try_from(u32::max(replace.start, replace.end)).ok()?;
    if end > text.len() {
        return None;
    }
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }

    let raw = text.get(start..end)?;
    if raw.chars().all(|c| c.is_whitespace()) {
        return None;
    }
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c.is_whitespace())
    {
        return None;
    }

    let query: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if query.is_empty() {
        return None;
    }
    Some(query)
}

fn attach_primary_edits(output_replace: Span, items: &mut [CompletionItem]) {
    for item in items {
        if item.is_disabled {
            item.primary_edit = None;
            item.cursor = None;
            continue;
        }

        item.primary_edit = Some(TextEdit {
            range: output_replace,
            new_text: item.insert_text.clone(),
        });

        item.cursor = match &item.data {
            Some(CompletionData::Function { .. }) => {
                // Prefer placing the cursor inside `(...)` when we insert it.
                item.insert_text.find('(').map(|idx| {
                    output_replace
                        .start
                        .saturating_add((idx as u32).saturating_add(1))
                })
            }
            Some(CompletionData::PropExpr { .. }) => {
                // `prop("Name")`: place the cursor at the end.
                Some(
                    output_replace
                        .start
                        .saturating_add(item.insert_text.len() as u32),
                )
            }
            Some(CompletionData::PostfixMethod { .. }) => item.insert_text.find('(').map(|idx| {
                output_replace
                    .start
                    .saturating_add((idx as u32).saturating_add(1))
            }),
            _ => None,
        };
    }
}

/// Ranks items by query match quality, with deterministic tie-breaks.
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

/// Filters postfix-method items to matches, then ranks them (label match ignores the leading `.`).
pub(super) fn fuzzy_filter_and_rank_postfix_items(query: &str, items: &mut Vec<CompletionItem>) {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MatchClass {
        Exact,
        Contains { pos: usize },
        Fuzzy(FuzzyScore),
        None,
    }

    fn match_class_for_label(query_norm: &str, label: &str) -> MatchClass {
        let label_norm = normalize_for_match(label.trim_start_matches('.'));
        if label_norm.is_empty() {
            return MatchClass::None;
        }
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
            let label_norm_len = normalize_for_match(item.label.trim_start_matches('.'))
                .chars()
                .count();
            let class = match_class_for_label(&query_norm, &item.label);
            Ranked {
                original_idx: idx,
                label_norm_len,
                class,
                item,
            }
        })
        .filter(|r| r.class != MatchClass::None)
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
            _ => a.original_idx.cmp(&b.original_idx),
        })
    });

    *items = ranked.into_iter().map(|r| r.item).collect();
}

/// Picks “smart” item indices that match the query, up to `preferred_limit`.
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

/// Groups items by `CompletionKind` and reorders groups toward `expected_ty`.
pub(super) fn apply_type_ranking(
    items: &mut Vec<CompletionItem>,
    expected_ty: Option<semantic::Ty>,
    ctx: Option<&semantic::Context>,
) {
    let expected_ty = match expected_ty {
        Some(expected_ty) => expected_ty,
        None => return,
    };
    if matches!(expected_ty, semantic::Ty::Unknown) {
        return;
    }

    fn kind_index(kind: CompletionKind) -> usize {
        match kind {
            CompletionKind::Builtin => 0,
            CompletionKind::Property => 1,
            CompletionKind::Function => 2,
            CompletionKind::Operator => 3,
        }
    }

    fn kind_section_priority(kind: CompletionKind) -> u8 {
        match kind {
            CompletionKind::Builtin => 0,
            CompletionKind::Property => 1,
            CompletionKind::Function => 2,
            CompletionKind::Operator => 3,
        }
    }

    #[derive(Debug)]
    struct ScoredItem {
        original_idx: usize,
        score: i32,
        item: CompletionItem,
    }

    let mut buckets: [Vec<ScoredItem>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut best_score: [i32; 4] = [i32::MIN, i32::MIN, i32::MIN, i32::MIN];

    for (idx, item) in items.drain(..).enumerate() {
        let actual = item_result_ty(&item, ctx);
        let score = type_match_score(expected_ty.clone(), actual);
        let bucket = kind_index(item.kind);
        best_score[bucket] = best_score[bucket].max(score);
        buckets[bucket].push(ScoredItem {
            original_idx: idx,
            score,
            item,
        });
    }

    for bucket in buckets.iter_mut() {
        bucket.sort_by(|a, b| {
            let a_key = (a.item.is_disabled, -a.score, a.original_idx as i32);
            let b_key = (b.item.is_disabled, -b.score, b.original_idx as i32);
            a_key.cmp(&b_key)
        });
    }

    let mut order: Vec<usize> = (0..4).filter(|&i| !buckets[i].is_empty()).collect();
    order.sort_by(|&a, &b| {
        (-best_score[a]).cmp(&(-best_score[b])).then_with(|| {
            let a_kind = buckets[a][0].item.kind;
            let b_kind = buckets[b][0].item.kind;
            kind_section_priority(a_kind).cmp(&kind_section_priority(b_kind))
        })
    });

    for idx in order {
        items.extend(buckets[idx].drain(..).map(|s| s.item));
    }
}

fn item_result_ty(item: &CompletionItem, ctx: Option<&semantic::Context>) -> Option<semantic::Ty> {
    if let Some(data) = &item.data {
        let ctx = ctx?;
        return match data {
            CompletionData::Function { name } => ctx
                .functions
                .iter()
                .find(|func| func.name == *name)
                .map(|func| func.ret.clone()),
            CompletionData::PropExpr { property_name } => ctx.lookup(property_name),
            CompletionData::PostfixMethod { .. } => None,
        };
    }

    match item.kind {
        CompletionKind::Builtin => match item.label.as_str() {
            "true" | "false" | "not" => Some(semantic::Ty::Boolean),
            _ => None,
        },
        _ => None,
    }
}

fn type_match_score(expected: semantic::Ty, actual: Option<semantic::Ty>) -> i32 {
    if matches!(expected, semantic::Ty::Unknown) {
        return 1;
    }
    match actual {
        Some(semantic::Ty::Unknown) => 0,
        Some(actual_ty) if semantic::ty_accepts(&expected, &actual_ty) => 2,
        Some(_) => -1,
        None => 0,
    }
}

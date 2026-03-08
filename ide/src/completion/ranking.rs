//! Ranks and post-processes completion items.
//! Ranking uses an ASCII-ish normalized query (lowercased; `_` removed).
//! Spans/cursors are UTF-8 byte offsets; ranges are half-open `[start, end)`.

use std::cmp::Ordering;

use crate::completion::matchers::{FuzzyScore, fuzzy_score, fuzzy_score_cmp, normalize_for_match};
use crate::completion::{CompletionData, CompletionItem, CompletionKind, TextEdit};
use crate::context::PositionKind;
use analyzer::Span;
use analyzer::semantic;

fn kind_priority(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::FunctionGeneral => 0,
        CompletionKind::FunctionText => 1,
        CompletionKind::FunctionNumber => 2,
        CompletionKind::FunctionDate => 3,
        CompletionKind::FunctionPeople => 4,
        CompletionKind::FunctionList => 5,
        CompletionKind::FunctionSpecial => 6,
        CompletionKind::Builtin => 7,
        CompletionKind::Property => 8,
        CompletionKind::Operator => 9,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchClass {
    Exact,
    Contains { pos: usize },
    Fuzzy(FuzzyScore),
    None,
}

impl MatchClass {
    fn rank(self) -> u8 {
        match self {
            MatchClass::Exact => 0,
            MatchClass::Contains { .. } => 1,
            MatchClass::Fuzzy(_) => 2,
            MatchClass::None => 3,
        }
    }
}

fn match_class_for_norm_label(query_norm: &str, label_norm: &str) -> MatchClass {
    if label_norm == query_norm {
        return MatchClass::Exact;
    }
    if let Some(pos) = label_norm.find(query_norm) {
        return MatchClass::Contains { pos };
    }
    if let Some(score) = fuzzy_score(query_norm, label_norm) {
        return MatchClass::Fuzzy(score);
    }
    MatchClass::None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankMode {
    Normal,
    Postfix,
}

#[derive(Debug)]
struct RankedItem {
    original_idx: usize,
    label_norm_len: usize,
    class: MatchClass,
    item: CompletionItem,
}

fn cmp_ranked_items(a: &RankedItem, b: &RankedItem) -> Ordering {
    a.class
        .rank()
        .cmp(&b.class.rank())
        .then_with(|| match (a.class, b.class) {
            (MatchClass::Exact, MatchClass::Exact) => a
                .label_norm_len
                .cmp(&b.label_norm_len)
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::Contains { pos: ap }, MatchClass::Contains { pos: bp }) => ap
                .cmp(&bp)
                .then_with(|| a.label_norm_len.cmp(&b.label_norm_len))
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::Fuzzy(sa), MatchClass::Fuzzy(sb)) => fuzzy_score_cmp(sa, sb)
                .then_with(|| kind_priority(a.item.kind).cmp(&kind_priority(b.item.kind)))
                .then_with(|| a.original_idx.cmp(&b.original_idx)),
            (MatchClass::None, MatchClass::None) => a.original_idx.cmp(&b.original_idx),
            _ => a.original_idx.cmp(&b.original_idx),
        })
}

fn label_for_match(item: &CompletionItem, mode: RankMode) -> &str {
    let base = match mode {
        RankMode::Normal => item.label.as_str(),
        RankMode::Postfix => item.label.trim_start_matches('.'),
    };
    if item.kind.is_function() {
        return base.strip_suffix("()").unwrap_or(base);
    }
    base
}

fn apply_query_ranking(query_norm: &str, items: &mut Vec<CompletionItem>, mode: RankMode) {
    let mut ranked: Vec<RankedItem> = items
        .drain(..)
        .enumerate()
        .map(|(idx, item)| {
            let label = label_for_match(&item, mode);
            let label_norm = normalize_for_match(label);
            let label_norm_len = label_norm.chars().count();

            let class = match mode {
                RankMode::Normal
                    if !(item.kind.is_function() || item.kind == CompletionKind::Property) =>
                {
                    MatchClass::None
                }
                _ => match_class_for_norm_label(query_norm, &label_norm),
            };

            RankedItem {
                original_idx: idx,
                label_norm_len,
                class,
                item,
            }
        })
        .filter(|r| mode != RankMode::Postfix || r.class != MatchClass::None)
        .collect();

    ranked.sort_by(cmp_ranked_items);
    *items = ranked.into_iter().map(|r| r.item).collect();
}

/// Fills in `primary_edit` and `cursor` for each item based on the replace span.
pub(crate) fn attach_primary_edits(output_replace: Span, items: &mut [CompletionItem]) {
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

/// Sorts and filters items by a query string.
///
/// In `AfterDot` position, items that don't match the query are removed entirely.
/// In other positions, items are sorted by match quality but kept.
pub(crate) fn rank_by_query(
    query: &str,
    items: &mut Vec<CompletionItem>,
    position_kind: PositionKind,
) {
    let query_norm = normalize_for_match(query);
    let mode = if matches!(position_kind, PositionKind::AfterDot) {
        RankMode::Postfix
    } else {
        RankMode::Normal
    };
    apply_query_ranking(&query_norm, items, mode);
}

/// Picks "smart" item indices that match the query, up to `preferred_limit`.
pub(crate) fn preferred_indices(
    items: &[CompletionItem],
    query: &str,
    preferred_limit: usize,
) -> Vec<usize> {
    if preferred_limit == 0 {
        return Vec::new();
    }

    let query_norm = normalize_for_match(query);
    let mut out = Vec::with_capacity(preferred_limit);
    for (idx, item) in items.iter().enumerate() {
        if out.len() >= preferred_limit {
            break;
        }
        if item.is_disabled {
            continue;
        }
        let label = if item.kind.is_function() {
            item.label.strip_suffix("()").unwrap_or(&item.label)
        } else {
            &item.label
        };
        if (item.kind == CompletionKind::Property || item.kind.is_function())
            && match_class_for_norm_label(&query_norm, &normalize_for_match(label))
                != MatchClass::None
        {
            out.push(idx);
        }
    }
    out
}

/// Groups items by `CompletionKind` and reorders groups toward `expected_ty`.
pub(crate) fn apply_type_ranking(
    items: &mut Vec<CompletionItem>,
    expected_ty: Option<semantic::Ty>,
    ctx: &semantic::Context,
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
            CompletionKind::FunctionGeneral => 0,
            CompletionKind::FunctionText => 1,
            CompletionKind::FunctionNumber => 2,
            CompletionKind::FunctionDate => 3,
            CompletionKind::FunctionPeople => 4,
            CompletionKind::FunctionList => 5,
            CompletionKind::FunctionSpecial => 6,
            CompletionKind::Builtin => 7,
            CompletionKind::Property => 8,
            CompletionKind::Operator => 9,
        }
    }

    fn kind_section_priority(kind: CompletionKind) -> u8 {
        match kind {
            CompletionKind::Builtin => 0,
            CompletionKind::Property => 1,
            CompletionKind::FunctionGeneral => 2,
            CompletionKind::FunctionText => 3,
            CompletionKind::FunctionNumber => 4,
            CompletionKind::FunctionDate => 5,
            CompletionKind::FunctionPeople => 6,
            CompletionKind::FunctionList => 7,
            CompletionKind::FunctionSpecial => 8,
            CompletionKind::Operator => 9,
        }
    }

    #[derive(Debug)]
    struct ScoredItem {
        original_idx: usize,
        score: i32,
        item: CompletionItem,
    }

    let mut buckets: [Vec<ScoredItem>; 10] = std::array::from_fn(|_| Vec::new());
    let mut best_score: [i32; 10] = [i32::MIN; 10];

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

    let mut order: Vec<usize> = (0..10).filter(|&i| !buckets[i].is_empty()).collect();
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

fn item_result_ty(item: &CompletionItem, ctx: &semantic::Context) -> Option<semantic::Ty> {
    if let Some(data) = &item.data {
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

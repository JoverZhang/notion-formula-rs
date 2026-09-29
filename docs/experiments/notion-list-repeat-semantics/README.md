---
doc_id: experiments.notion-list-repeat-semantics
title: "Notion list nulls and repeat counts"
language: en
source_language: zh-CN
counterpart: ./README.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-09-29
---

# Notion list nulls and repeat counts

[简体中文](README.zh-CN.md)

The tested Notion database formulas skip null Number values in aggregates, remove null elements in `sort`, and preserve them in `flat`. Tested finite `repeat` counts behave as a nonnegative ceiling capped at 10,000 repetitions. These experiments inform the runtime semantics needed for [issue #53](https://github.com/JoverZhang/notion-formula-rs/issues/53).

Measured on 2026-09-29 with `Notion-Version: 2025-09-03`. The [initial results](results-2026-09-29.json) and [follow-up results](results-2026-09-29-followup.json) preserve typed responses from `GET /v1/pages/{page_id}`. All 39 rows were read twice; all 509 formula results agreed between reads. Of 217 submitted formula definitions, 43 were rejected at creation and recorded separately.

## Aggregates skip typed nulls

`sum([2, empty(), 4])` and its `mean`, `median`, `min`, and `max` counterparts were rejected with HTTP 400, `validation_error`, and `Type error with formula`. Their untyped `[]` and all-`empty()` arguments were also rejected. These are creation-time observations, not runtime null rules.

The follow-up used three accepted Number expressions with API null results: a blank Number property `N`, `if(false, 1, empty())`, and `find([1], current < 0)`. The stored property was read back as `number: null`. Each source retained its position in `[2, source, 4]`, whose length was 3.

| Function | `[2, null, 4]` | `[-2, null, -4]` | `[null]` | Typed empty list |
| --- | ---: | ---: | ---: | ---: |
| `sum` | 6 | -6 | 0 | 0 |
| `mean` | 3 | -3 | API null | API null |
| `median` | 3 | -3 | API null | API null |
| `min` | 2 | -4 | API null | API null |
| `max` | 4 | -2 | API null | API null |

Here `null` denotes the three typed sources, not a formula literal. The typed empty list was `filter([1], current < 0)`. All three sources agreed, and `F(2, source, 4)` matched the list form. Zero controls distinguish skipping from zero conversion: `mean([2, 0, 4])` and `median([2, 0, 4])` returned 2; `min([2, 0, 4])` returned 0.

With `Pattern="["`, `sum([2, if(test("abc", prop("Pattern")), 1, 1), 4])` returned API null, as did its `empty` and `format` wrappers. With valid pattern `"a"`, the aggregate returned 7. Thus this executed failure behaves differently from the ordinary null sources; the observation does not establish failure propagation for every function family.

## Sort removes nulls; flat preserves them

Structural observations use `length` and indexed `at` reads, corroborated by `join` and marked `map` results. The [earlier experiment](../notion-empty-semantics/README.md) explains why direct API list rendering alone is insufficient.

The `sort` cases use `empty()` elements. The `flat` cases use `empty()` and the list-typed null `if(false, [1], empty())`; these cases do not cover every possible typed null source.

| Expression | Structural observation |
| --- | --- |
| `sort([2, empty(), 1])` | Length 2, elements 1 and 2 |
| Null moved to the first or last position | Same sorted result |
| `sort([1, empty(), 0, -1, empty()])` | Length 3, elements -1, 0, 1; zero retained |
| `sort([empty(), empty()])` | Length 0 |
| `flat([1, empty(), 2])` | Length 3; joining with a vertical bar gives `"1\|\|2"` |
| `flat([[1, empty()], [2]])` | Length 3; indexed results are 1, API null, 2 |
| `flat([[empty()], []])` | Length 1; the element is empty |
| `flat([[], empty(), [2]])` | Length 2; an empty element precedes 2 |

The depth probe distinguishes one-level from recursive flattening: `length(flat([[[]]]))` returned 1, and `join(map(flat([[[]]]), length(current)), "|")` returned `"0"`. The retained element was an empty list. This differs from the repository's current recursive `flat` contract in [Builtin function signatures](../../specs/builtin-functions.md#signature-notation).

## Repeat clamps and rounds tested finite counts

Both literal counts and a Number property were tested. For text `"ab"`:

| Count | Observed result |
| --- | --- |
| -2, -1.9, -1, -0.9, -0.1, 0 | Empty string; length 0; `empty(...)` is true |
| 0.1, 0.9, 1 | `"ab"` |
| 1.1, 1.9 | `"abab"` |
| 2.9 | `"ababab"` |
| 9,999 | Length 19,998 |
| 10,000 | Length 20,000 |
| 10,001; 100,000; 1,000,000; 1,000,001 | Length 20,000 |

The follow-up tested `"x"` at 9,999.1, 10,000, 10,000.1, and 10,001: all returned exactly 10,000 `x` characters. At count 10,001, `"abc"` returned length 30,000 and the fox emoji returned 10,000 copies. Direct API strings agreed with the repetition counts and formula-level lengths. This supports a repetition-count cap rather than API truncation or a fixed output-length cap for these cases.

The tested finite inputs fit this empirical model:

```text
repetitions = min(10000, max(0, ceil(count)))
```

A blank Count property produced API null for the repeat length, `empty` check, and preview. Counts expressed as `0 / 0`, `1 / 0`, and `-1 / 0` did too; those cases do not isolate `repeat` from evaluation of the count expression. A bare `empty()` count was rejected at creation. Empty text produced an empty string for counts 3, -1, and 1,000,001.

The finite-count observations differ from the nonnegative constraint planned at the time of the experiment and the evaluator's truncation and 1,000,000-count limit at that time.
The subsequently adopted contract is [Planned repeat](../../specs/builtin-functions.md#planned-repeat); this experiment preserves the original observations and does not define the current contract.

## Reproduce and interpret

The [official function reference](https://www.notion.com/help/formula-syntax) does not define these edge cases. An API null result alone cannot distinguish an internal null, a nonfinite number, or a failure; this limits conclusions about the all-empty aggregates. See [formula property values](https://developers.notion.com/reference/page-property-values#formula).

Set `NOTION_TOKEN` and `NOTION_PARENT_PAGE_ID`, then run from the repository root. Without a token environment variable the probe prompts without echo; without a parent ID it searches only for the prior page titled `Notion empty semantics probe`.

```sh
python3 -B docs/experiments/notion-list-repeat-semantics/probe.py \
  --output /tmp/notion-list-repeat-initial.json
python3 -B docs/experiments/notion-list-repeat-semantics/probe.py \
  --cases docs/experiments/notion-list-repeat-semantics/followup-typed-nulls.json \
          docs/experiments/notion-list-repeat-semantics/followup-repeat-boundary.json \
  --output /tmp/notion-list-repeat-followup.json
```

Each invocation creates a separate experiment page and databases. Outputs contain expressions, submitted inputs, typed responses, creation errors, and repeat-read comparisons; the follow-up also records stored inputs read from the API. Credentials and private resource IDs are excluded. Existing outputs are refused; use new dated filenames for reruns and preserve these records.

---
doc_id: experiments.notion-empty-semantics
title: "Notion `empty()` and null behavior"
language: en
source_language: zh-CN
counterpart: ./README.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
translation_model: gpt-6-luna
translation_review_model: gpt-6-astra
last_verified: 2026-09-28
---

# Notion `empty()` and null behavior

[简体中文](README.zh-CN.md)

Calling `empty()` with no arguments returns null, with behavior similar to JavaScript's `null`. This experiment examines how null participates in evaluation and how its output differs from execution failures.

Measured on 2026-09-28 with Notion API version `2025-09-03`. The [results file](results-2026-09-28.json) contains the complete inputs and responses.

## Null values

Empty elements in a list occupy a position, and `map` retains that position when it returns an empty value:

```text
length([1, empty(), 2]) → 3
join(map([1, empty(), 2], index), ",") → "0,1,2"
join(map([1, 2, 3], if(current == 2, empty(), current)), ",") → "1,,3"
```

An empty value in a list-typed context can pass type checking and evaluates to empty:

```text
if(false, [], empty()).map(current) → null
if(false, [], empty()).map(current).length() → null
```

## Output representation

Direct API output comes from `properties[formula property name].formula` in `GET /v1/pages/{page_id}`. Based on observations, list output can be explained by this model:

```js
const values = arr.flat(Infinity).filter(value => value !== null);
return values.length === 0 ? null : values.join(",");
```

This is a hypothesis about Notion database API list output logic. JavaScript `null` corresponds to Notion's `empty()`; the model covers the values tested below and does not extend to string-conversion rules for other types.

| Array expression | API return value | Observation |
|---|---|---|
| `[[[1, 2]], 3]` | `"1,2,3"` | Recursive flattening |
| `[[1, empty(), 2]]` | `"1,2"` | Empty values are filtered |
| `[[[]]]` | `null` | No elements remain after flattening |
| `[[""], 1]` | `",1"` | Empty string is preserved |
| `[[0], [false]]` | `"0,false"` | Zero and false are preserved |

Explicit `format` and `join` preserve the separators corresponding to empty elements, unlike direct API output:

```text
[1, empty()]           → API: "1"
format([1, empty()])   → "1,"
join([1, empty()], ",") → "1,"
```

## Runtime failures

The formula `test("abc", prop("Pattern"))` passes type checking. With `Pattern="a"`, it returns `true`; after changing the input to the invalid regular expression `"["`, the API returns:

```json
{"type":"boolean","boolean":null}
```

Although the output is also null, a failure behaves differently from an ordinary empty value in later operations. The following examples all use `Pattern="["`:

```text
empty(empty()) → true
empty(test("abc", prop("Pattern"))) → null
format(empty()) → ""
format(test("abc", prop("Pattern"))) → null
```

Outer operations also affect how a failure propagates:

```text
if(false, test("abc", prop("Pattern")), true) → true
[test("abc", prop("Pattern"))].length() → null
[1].map(test("abc", prop("Pattern"))).length() → 1
[1].map(test("abc", prop("Pattern"))).join(",") → ""
```

Therefore, a null API output alone is not enough to distinguish an ordinary empty value from a runtime failure.

---

**Type checking.** `empty().map(current)` returns HTTP 400 `validation_error` when the formula is created, before evaluation begins. The `formula_creation_errors` field in the results file records these rejections separately.

**Reproduction.** Set `NOTION_TOKEN` and `NOTION_PARENT_PAGE_ID` (the ID of a parent page authorized for the integration), then run this from the repository root:

```sh
python3 -B docs/experiments/notion-empty-semantics/probe.py --output /tmp/notion-empty-results.json
```

The script creates a separate experiment page and database. `rows` records named input rows and their evaluation results; `list_rendering` rows use `operation` to distinguish output paths. The output contains no private resource identifiers or credentials. Use a new dated file for each rerun and preserve existing records.

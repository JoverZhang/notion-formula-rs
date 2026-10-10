---
doc_id: experiments.notion-ifs-arity
title: "Does Notion `ifs` require an else value?"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-10
---

# Does Notion `ifs` require an else value?

[简体中文](README.zh-CN.md)

**No: the tested Notion database formulas accepted `ifs` without a final else value.** Calls with two, four, and six arguments returned the first matching value. When no condition matched, the tested Number result behaved like an ordinary empty value under `empty`, `format`, and list observers.

Measured on 2026-10-10 with `Notion-Version: 2025-09-03`. The [raw results](results-2026-10-10.json) preserve creation errors, stored inputs, and typed formula responses from `GET /v1/pages/{page_id}`. Of 51 submitted definitions, 43 were accepted and eight rejected. All six rows were read twice; all 75 formula results agreed between reads. The [cases](cases.json) define the complete test matrix.

## Accepted calls without else

These expressions were accepted as database formula properties and evaluated successfully when a condition matched:

| Expression | Observed Number result |
| --- | ---: |
| `ifs(true, 11)` | 11 |
| `ifs(false, 11)` | API null |
| `ifs(true, 11, false, 22)` | 11 |
| `ifs(false, 11, true, 22)` | 22 |
| `ifs(true, 11, true, 22)` | 11 |
| `ifs(false, 11, false, 22)` | API null |
| `ifs(false, 11, false, 22, true, 33)` | 33 |
| `ifs(false, 11, false, 22, false, 33)` | API null |

Explicit fallback controls also worked: `ifs(false, 11, 99)`, `ifs(false, 11, false, 22, 99)`, and the seven-argument counterpart all returned 99. `ifs(true, 11, 99)` returned 11.

Property-driven conditions corroborated the literal cases. For `ifs(prop("Count") > 0, 11, prop("N") > 0, 22)`, inputs `(Count, N)` of `(1, 0)`, `(0, 1)`, `(1, 1)`, and `(0, 0)` returned 11, 22, 11, and API null respectively. Adding a fallback of 99 changed the no-match result to 99.

## No match behaves as an ordinary empty value in these observers

For `ifs(false, 11)`, the direct API response was:

```json
{"type":"number","number":null}
```

That response alone cannot distinguish an ordinary empty result from an evaluation failure. The following observers distinguish the tested cases; the format observer wraps `format(value)` in angle brackets so an empty string remains visible.

| Value expression | `empty(value)` | `"<" + format(value) + ">"` |
| --- | --- | --- |
| `ifs(false, 11)` | `true` | `"<>"` |
| `ifs(false, 11, empty())` | `true` | `"<>"` |
| `empty()` | `true` | `"<>"` |
| `0` | `true` | `"<0>"` |
| `test("abc", prop("Pattern"))`, with `Pattern="["` | API null | API null |

The invalid regular expression was an executed failure control. Its direct result was also API null, but its observers differed from the no-match `ifs` result. List observers further showed that the no-match value retained a position: `length([ifs(false, 11)])` returned 1, and `join([1, ifs(false, 11), 2], "|")` returned `"1||2"`.

Other no-match cases returned `string: null` for a Text value, `boolean: null` for a Boolean value, and `date: null` for a Date value. The list-valued expression `ifs(false, [11, 22])` was exposed as `{"type":"string","string":null}`. These API representations do not establish Notion's internal value types; the ordinary-empty observer comparison above tested the Number case.

## Creation rejections and branch controls

The following definitions were rejected when installing the formula, before any row evaluation:

| Definitions | HTTP response |
| --- | --- |
| `ifs()`, `ifs(true)`, `ifs(false)`, `ifs(42)`, `ifs("fallback")` | 400 `validation_error`, `Type error with formula` |
| `if(true, 11)`, `if(false, 11)` | 400 `validation_error`, `Type error with formula` |
| `ifs(true,)` | 400 `validation_error`, `Parse error with formula` |

Three-argument `if` controls returned the selected value. Thus the two-argument `if` rejections do not describe the accepted two-argument `ifs` calls.

Property-driven failure controls support skipping unselected expressions in the tested calls. With `Pattern="["`, `empty(ifs(prop("Count") > 0, test("abc", prop("Pattern"))))` returned `true` when Count was 0, but API null when Count was 1. The selected invalid-regex value therefore differed from the empty no-match result.

When Count was positive, both `ifs(prop("Count") > 0, true, test("abc", prop("Pattern")), false)` and `ifs(prop("Count") > 0, true, test("abc", prop("Pattern")))` returned `true` despite the invalid later condition or fallback. With Count 0, those expressions returned API null. Changing Pattern to `"a"` made the direct regex control return `true`; the later-condition and fallback cases then returned `false` and `true` respectively.

Incidental boundary cases were also accepted: `ifs(1, 11)` and `ifs("true", 11)` returned 11, and `ifs(false, 11, 42, 22)` returned 22. Mixed Number/Text value cases selected 11 or `"hit"` as their conditions changed. These samples do not define a general truthiness or type-conversion rule.

## Sources, limits, and reproduction

The [official function reference](https://www.notion.com/help/formula-syntax) describes first-true-condition selection and shows fallback-bearing examples. It does not explicitly require the fallback or describe omission. Formula acceptance and selected values in this experiment establish that omission worked in the tested database calls; this experiment did not inspect the formula editor's signature display or establish every possible arity and value type.

The [official formula property reference](https://developers.notion.com/reference/page-property-values#formula) documents typed API results and allows a supported result to be null when it has no value. API null alone does not explain the cause of a missing result, as the empty and failure controls above demonstrate. The results record API observations and do not infer Notion's internal representation or change this repository's runtime or specification.

Set `NOTION_TOKEN` and `NOTION_PARENT_PAGE_ID`, then run the [probe](probe.py) from the repository root:

```sh
python3 -B docs/experiments/notion-ifs-arity/probe.py \
  --output /tmp/notion-ifs-results.json
```

Without a token environment variable, the probe prompts without echo. Without a parent ID, it searches for the prior page titled `Notion empty semantics probe`. It uses the adjacent cases and the [shared database probe helpers](../notion-list-repeat-semantics/probe.py), creates a separate experiment page and two databases, installs formula definitions, and reads each row twice. The saved report excludes credentials and private resource IDs. Existing output files are refused; use a new dated filename for each rerun and preserve existing evidence.

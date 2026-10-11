---
doc_id: experiments.notion-lets-semantics
title: "Does Notion `lets` allow duplicate variable names?"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# Does Notion `lets` allow duplicate variable names?

[简体中文](README.zh-CN.md)

**Yes: the tested Notion database formulas allowed duplicate variable names in one `lets` call, with later lookup using the later binding.** The requested `lets(a, 1, label, "v", a, a + 1, label + format(a))` was accepted and returned `"v2"`. Its second `a` initializer read the earlier value, 1.

Measured on 2026-10-11 with `Notion-Version: 2025-09-03`. The [sanitized raw results](results-2026-10-11.json) preserve creation errors, stored inputs, and typed formula responses from `GET /v1/pages/{page_id}`. Of 14 submitted definitions, 12 were accepted and two rejected. All four rows were read twice; all 16 formula results agreed between reads. The [cases](cases.json) define the complete test matrix.

## Duplicate names, initializer lookup, and scope

All expressions below were accepted as database formula properties and evaluated to these results:

| Expression | Observed API result |
| --- | --- |
| `lets(a, 1, a, 2, a)` | Number `2` |
| `lets(a, 1, label, "v", a, a + 1, label + format(a))` | String `"v2"` |
| `lets(a, 1, a, a + 1, a, a * 3, a)` | Number `6` |
| `lets(a, 1, a, "v" + format(a), a)` | String `"v1"` |
| `lets(a, 1, before, a, a, 2, format(before) + ":" + format(a))` | String `"1:2"` |
| `let(a, 7, lets(a, 1, a, a + 1, a) + a)` | Number `9` |
| `let(a, 7, lets(a, a + 1, a))` | Number `8` |
| `lets(a, 1, b, a + 1, a + b)` | Number `3` |

The constant rebinding case establishes acceptance and final lookup separately from initializer lookup. The requested and repeated-self-reference cases show an initializer reading the previous binding. The Number-to-String case also permits a type change in the tested rebinding. `before` retaining 1 while `a` becomes 2 supports sequential shadowing rather than mutation of the earlier captured value. The two outer-`let` controls show access to an outer binding in an initializer and preservation of that outer value after the inner call.

The requested formula's direct API result was `{"type":"string","string":"v2"}`. Its no-rebinding control, `lets(a, 1, label, "v", label + format(a))`, returned `"v1"`. The nested control `let(a, 1, let(label, "v", let(a, a + 1, label + format(a))))` returned `"v2"`.

Property-driven controls corroborated those literal results. Both `lets(a, prop("Count"), label, prop("Text"), a, a + 1, label + format(a))` and `let(a, prop("Count"), let(label, prop("Text"), let(a, a + 1, label + format(a))))` returned the following Strings:

| Stored Count | Stored Text | Both results |
| ---: | --- | --- |
| 1 | `"v"` | `"v2"` |
| 0 | `"zero:"` | `"zero:1"` |
| -3 | `"数:"` | `"数:-2"` |

The unbound controls `lets(a, a + 1, a)` and `lets(a, b + 1, b, 2, a)` were rejected during formula installation with HTTP 400 `validation_error`, `Type error with formula`. They produced no evaluated row results. These rejections distinguish references without an available earlier or outer binding from the accepted duplicate-name cases.

## Official sources

The [official function reference](https://www.notion.com/help/formula-syntax), checked on 2026-10-11, describes `let` as assigning one variable for an expression and `lets` as assigning several. Its examples include `lets(a, "Hello", b, "world", a + " " + b)` returning `"Hello world"`, and a separate example with two Number variables.

Those examples establish ordinary multi-variable use; the reference does not specify duplicate names, whether an initializer can read a prior binding, or the order in which bindings become visible. It also does not explicitly define mixed-type sequential bindings, nested lexical scope, or equivalence to nested `let` calls. The requested expression therefore needs experimental evidence rather than an assumed general desugaring rule.

The [official formula property reference](https://developers.notion.com/reference/page-property-values#formula) describes read-only evaluated results with `string`, `number`, `boolean`, `date`, or `unsupported` types. Supported results can contain null when they have no value. These API result objects expose observations; they do not define `lets` name lookup or explain the cause of a null result.

## Limits and reproduction

These observations support sequential shadowing in the tested database formulas. Agreement with the literal and property-driven nested controls does not establish equivalence of every `lets` call to nested `let`. The experiment does not inspect the formula editor or establish every variable type or nesting context, and does not change this repository's runtime or specification.

Set `NOTION_TOKEN` and `NOTION_PARENT_PAGE_ID`, then run the [probe](probe.py) from the repository root:

```sh
python3 -B docs/experiments/notion-lets-semantics/probe.py \
  --output /tmp/notion-lets-results.json
```

Without a token environment variable, the probe prompts without echo. Without a parent ID, it uses the [shared database probe helpers](../notion-list-repeat-semantics/probe.py) to find the existing experiment parent. The adjacent [cases](cases.json) define literal and property-driven checks. The probe creates a separate test page and two databases, installs formula definitions, and reads each row twice. Saved results exclude credentials and private resource IDs. Existing output files are refused; use a new dated filename for each rerun and preserve existing evidence.

---
doc_id: changelog.20261005-appflowy-builtins
title: "Add native formula builtin compatibility"
language: en
source_language: en
counterpart: ./20261005-appflowy-builtins.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Add native formula builtin compatibility

[简体中文](20261005-appflowy-builtins.zh-CN.md)

- Type: Added
- Component: FormulaEngine, Draft analysis and builtin functions

## Summary

Native formulas now support lazy `and`/`or` calls and infix operators, sequential heterogeneous `lets`, plain-text
`style`/`unstyle`, and zero-based `index` in list callbacks. `ifs` accepts an omitted else,
`formatNumber` accepts omitted precision and `humanize`, and `formatDate` handles bracket
literals, week numbers, ordinals and weekday tokens. String addition now infers String.

## Compatibility notes

- Boolean and null rules remain those of native logical operators. Unmatched `ifs` without
  else returns ordinary null; style names are evaluated and checked, then ignored.
- `concat` still requires at least two lists; numeric reducers still require at least one
  argument. `link` remains unsupported; `not(true)` uses the existing prefix operator.
- Engine and WASM DTO shapes do not change. Draft property references retain original
  source order and spans through sequential-binding lowering.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin declarations](../specs/builtin-functions.md)
- [Language rules](../specs/formula-language.md)

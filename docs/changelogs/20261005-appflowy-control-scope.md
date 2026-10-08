---
doc_id: changelog.20261005-appflowy-control-scope
title: "Add native formula control and scope compatibility"
language: en
source_language: en
counterpart: ./20261005-appflowy-control-scope.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Add native formula control and scope compatibility

[简体中文](20261005-appflowy-control-scope.zh-CN.md)

- Type: Added
- Component: FormulaEngine, Draft analysis and builtin functions

## Summary

Native formulas now support lazy `and`/`or` calls and infix operators, sequential heterogeneous
`lets`, and zero-based `index` in list callbacks. `ifs` accepts an omitted else.

## Compatibility notes

- Boolean and null rules remain those of native logical operators. Unmatched `ifs` without
  else returns ordinary null.
- `concat` still requires at least two lists; numeric reducers still require at least one
  argument. `not(true)` uses the existing prefix operator.
- Engine and WASM DTO shapes do not change. Sequential-binding lowering retains original
  argument identities and source spans.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin declarations](../specs/builtin-functions.md)
- [Language rules](../specs/formula-language.md)

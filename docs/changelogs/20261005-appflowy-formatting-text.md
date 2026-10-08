---
doc_id: changelog.20261005-appflowy-formatting-text
title: "Add native formula formatting and text compatibility"
language: en
source_language: en
counterpart: ./20261005-appflowy-formatting-text.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Add native formula formatting and text compatibility

[简体中文](20261005-appflowy-formatting-text.zh-CN.md)

- Type: Added
- Component: FormulaEngine, Draft analysis and builtin functions

## Summary

Native formulas now support plain-text `style`/`unstyle`. `formatNumber` accepts omitted
precision and `humanize`, and `formatDate` handles bracket literals, week numbers,
ordinals and weekday tokens. String addition now infers String.

## Compatibility notes

- Style names are evaluated and checked, then ignored; ordinary null text remains null.
- `concat` still requires at least two lists; scalar text concatenation uses `+`.
  `link` remains unsupported.
- Engine and WASM DTO shapes do not change.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin declarations](../specs/builtin-functions.md)
- [Language rules](../specs/formula-language.md)

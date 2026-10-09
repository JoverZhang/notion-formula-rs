---
doc_id: changelog.20261005-appflowy-formulas
title: "Use native formulas in the AppFlowy example"
language: en
source_language: en
counterpart: ./20261005-appflowy-formulas.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Use native formulas in the AppFlowy example

[简体中文](20261005-appflowy-formulas.zh-CN.md)

- Type: Changed
- Component: AppFlowy example

## Summary

The pinned AppFlowy-Web example uses `@notion-formula/sdk` for saved formulas, editor
analysis and preview, filters, sorting, calculations, Rollup, history, and field conversion.
Its JavaScript formula parser and evaluator are removed.

## Compatibility notes

Yjs storage and field-ID references are retained. Existing formulas use the native semantics
selected by the build's SDK pin. The example's [integration guide](../../examples/appflowy-web/doc/NOTION_FORMULA.md)
records the compatibility decisions and editor behavior. Other AppFlowy clients are not migrated.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [AppFlowy integration PR](https://github.com/JoverZhang/AppFlowy-Web/pull/1)
- [Build and verify the example](../contributing/appflowy-example.md)

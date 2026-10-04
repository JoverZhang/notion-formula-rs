---
doc_id: changelog.20261004-demo-engine-drafts
title: "Evaluate saved formulas in the browser demo"
language: en
source_language: en
counterpart: ./20261004-demo-engine-drafts.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-04
---

# Evaluate saved formulas in the browser demo

[简体中文](20261004-demo-engine-drafts.zh-CN.md)

- Type: Changed
- Component: Browser demo

## Summary

The demo now analyzes Drafts asynchronously and evaluates saved formulas across all sample rows,
including shared dependencies. Save commits an expression and recalculates the table; Discard
restores the saved expression. Other editors retain their unsaved changes during a save.

The table displays actual values, ordinary nulls, NotReady formulas, and row errors with their
originating Formula IDs. Nested nulls, non-finite numbers, signed zero, and exact dates retain their
runtime representation.

## Compatibility notes

- Editing a Draft no longer changes the table until Save. This demo migration leaves the public
  Rust and WASM APIs unchanged.

## Links

- [Issue #64](https://github.com/JoverZhang/notion-formula-rs/issues/64)
- [Current WASM API](../specs/wasm-api.md)

---
doc_id: changelog.20261005-formula-editor-compatibility
title: "Expose property references and receiver-only formula methods"
language: en
source_language: en
counterpart: ./20261005-formula-editor-compatibility.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Expose property references and receiver-only formula methods

[简体中文](20261005-formula-editor-compatibility.zh-CN.md)

- Type: Changed
- Component: Formula analysis, evaluation, and editor services

## Summary

A receiver can supply the only argument to a builtin. Calls such as `[1, 2, 3].sum()`
and `"hello".length()` now evaluate like their prefix forms. Completion and signature
help use the same eligibility rule.

Draft snapshots now expose decoded property IDs and exact call/literal spans. Editors can retain stable property chips and replace IDs while preserving comments, without implementing another formula parser. Native spans use UTF-8; WASM spans use UTF-16. Complete references remain available when another part of the expression has an error.

## Compatibility notes

Previously rejected methods now work. Receiver types and remaining arguments still
follow the builtin declaration. Functions without a receiver slot, such as `now()`
and `id()`, and the special `prop()` syntax remain unavailable as methods.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin call contract](../specs/builtin-functions.md#calls-and-execution)
- [Draft reference contract](../specs/ide.md#formuladraft)

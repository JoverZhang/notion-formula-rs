---
doc_id: changelog.20261011-sequential-lets
title: "Add sequential lets bindings"
language: en
source_language: en
counterpart: ./20261011-sequential-lets.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# Add sequential lets bindings

[简体中文](20261011-sequential-lets.zh-CN.md)

- Type: Added
- Component: Formula analysis and evaluation

## Summary

`lets` accepts sequential bindings with independent value types. Later bindings can read and
shadow earlier names, and the final expression determines the result type.

## Compatibility notes

Previously unsupported `lets` calls now evaluate. Binding positions still require identifiers,
and calls require at least one complete binding pair followed by a result expression.

## Links

- [Compatibility request #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin function contract](../specs/builtin-functions.md#general)

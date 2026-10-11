---
doc_id: changelog.20261011-list-callback-index
title: "Add zero-based list callback index"
language: en
source_language: en
counterpart: ./20261011-list-callback-index.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# Add zero-based list callback index

[简体中文](20261011-list-callback-index.zh-CN.md)

- Type: Added
- Component: list callbacks

## Summary

`map`, `filter`, `find`, `findIndex`, `some`, `every`, and `count` now bind a zero-based `index` beside `current`. Nested callbacks shadow and restore both names, and null elements retain their input positions.

## Compatibility notes

- Callback `index` now shadows an outer lexical binding with that name; the outer binding is restored after the callback.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin function contract](../specs/builtin-functions.md#list)

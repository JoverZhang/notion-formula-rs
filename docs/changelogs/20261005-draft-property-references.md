---
doc_id: changelog.20261005-draft-property-references
title: "Expose structured Draft property references"
language: en
source_language: en
counterpart: ./20261005-draft-property-references.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Expose structured Draft property references

[简体中文](20261005-draft-property-references.zh-CN.md)

- Type: Added
- Component: FormulaDraft and WASM API

## Summary

Draft snapshots now expose decoded property IDs and exact call/literal spans. Editors can retain stable property chips and replace IDs while preserving comments, without implementing another formula parser. Native spans use UTF-8; WASM spans use UTF-16. Complete references remain available when another part of the expression has an error.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Draft reference contract](../specs/ide.md#formuladraft)

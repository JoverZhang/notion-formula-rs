---
doc_id: changelog.20260730-evaluator-builtin-goldens
title: "Add evaluator builtin golden fixtures"
language: en
source_language: en
counterpart: ./20260730-evaluator-builtin-goldens.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-30
---

# Evaluator builtin goldens

[简体中文](20260730-evaluator-builtin-goldens.zh-CN.md)

- Type: Changed
- Component: evaluator

## Summary

Evaluator runtime behavior is now documented and verified by readable golden fixtures for
every supported builtin. Fixtures can include typed property columns, row IDs, execution
masks, and a frozen runtime context alongside the formula and per-row result.

## Compatibility notes

- No public Rust interface or formula behavior changed.

## Tests

- `cargo test -p evaluator --test builtin_golden`

## Links

- [Builtin declaration implementation](../how/builtin_fn/README.md)
- [Evaluator implementation](../how/evaluator/README.md)

---
doc_id: changelog.20260924-formula-engine-definitions
title: "Add FormulaEngine definition management"
language: en
source_language: en
counterpart: ./20260924-formula-engine-definitions.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-09-24
---

# Add FormulaEngine definition management

[简体中文](20260924-formula-engine-definitions.zh-CN.md)

- Type: Added
- Component: FormulaEngine Rust API

## Summary

`FormulaEngine` now stores Input and Formula definitions, returns independent property snapshots,
and reports inferred output types, dependency readiness, and cycles after definition changes.

## Compatibility notes

- This Rust API is additive; existing APIs and data formats are unchanged. No migration is required.

## Links

- [Issue #50](https://github.com/JoverZhang/notion-formula-rs/issues/50)
- [PR #52](https://github.com/JoverZhang/notion-formula-rs/pull/52)
- [Current FormulaEngine specification](../specs/formula-engine.md)

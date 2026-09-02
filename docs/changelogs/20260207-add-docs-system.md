---
doc_id: changelog.20260207-add-docs-system
title: "Add the documentation system"
language: en
source_language: en
counterpart: ./20260207-add-docs-system.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-07
---

# 20260207-add-docs-system

[简体中文](20260207-add-docs-system.zh-CN.md)

- Type: Added
- Component: docs

## Summary

- Added a structured docs layout:
  - `docs/design/README.md` as the design/contract entry point.
  - Module-local READMEs next to code (`analyzer/`, `analyzer_wasm/`, `examples/vite/`).
  - Changelog entry guidelines + templates under `docs/`.
- Migrated the former analyzer overview doc into `docs/design/README.md` and module READMEs.

## Compatibility notes

- Docs-only change. No runtime behavior changes intended.

## Tests

- `cargo test -p analyzer`
- `cargo test -p analyzer_wasm`

## Links

- See `docs/README.md` for workflow and templates.

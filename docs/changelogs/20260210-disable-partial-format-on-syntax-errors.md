---
doc_id: changelog.20260210-disable-partial-format-on-syntax-errors
title: "Disable partial formatting on syntax errors"
language: en
source_language: en
counterpart: ./20260210-disable-partial-format-on-syntax-errors.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-10
---

# 20260210-disable-partial-format-on-syntax-errors

[简体中文](20260210-disable-partial-format-on-syntax-errors.zh-CN.md)

- Type: Fixed
- Component: analyzer_wasm

## Summary

Formatting behavior was tightened:

- syntax-invalid inputs no longer produce partial formatter output
- syntax-valid formatting remains available
- this laid groundwork for strict edit application and cursor rebasing

## Compatibility notes

- formatting on syntax errors is treated as failure instead of returning partial text
- this behavior is now part of the strict `format(..., cursor)` contract

## Tests

- `cargo test -p analyzer_wasm`
- `cargo test -p analyzer`

## Links

- [WASM boundary implementation](../how/analyzer_wasm/README.md)
- [Vite example implementation](../how/examples/vite/README.md)

---
doc_id: changelog.20261008-token-string-values
title: "Add SDK formula string codec"
language: en
source_language: en
counterpart: ./20261008-token-string-values.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-09
---

# Add SDK formula string codec

[简体中文](20261008-token-string-values.zh-CN.md)

- Type: Added
- Component: Browser SDK, Vite editor

## Summary

The SDK exports synchronous pure functions `decodeFormulaString` and `quoteFormulaString`.
Decoding accepts a complete double-quoted formula literal, including the empty string, with
`\\`, `\"`, `\n` and `\t` escapes; invalid quotes or escapes return `null`.
Vite property chips decode raw token text and exclude member calls and incomplete property calls.

## Compatibility notes

No breaking changes. WASM Token DTOs retain `kind`, raw `text`, and UTF-16 `span`.

## Links

- [WASM and SDK contract](../specs/wasm-api.md)

---
doc_id: changelog.20261008-token-string-values
title: "Expose decoded string token values"
language: en
source_language: en
counterpart: ./20261008-token-string-values.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-08
---

# Expose decoded string token values

[简体中文](20261008-token-string-values.zh-CN.md)

- Type: Added
- Component: Analyzer, WASM API, Browser SDK

## Summary

Native tokens expose `Token::string_value()`. WASM tokens include `string_value` for valid strings,
including empty strings; other tokens and invalid escapes return `null`.
The SDK exports `quoteFormulaString` for writing formula strings. Vite property chips use decoded
values and exclude member calls and incomplete property calls.

## Compatibility notes

Raw token spelling, `kind`, and byte/UTF-16 spans are unchanged. Code constructing WASM token DTOs
must provide the new required `string_value: string | null` field.

## Links

- [Lexical contract](../specs/formula-language.md#lexical-structure)
- [WASM and SDK contract](../specs/wasm-api.md)

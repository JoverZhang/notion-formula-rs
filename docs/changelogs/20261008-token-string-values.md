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
- Component: Analyzer, Browser SDK, Vite editor

## Summary

The SDK exports synchronous pure functions `encodeFormulaString` and `decodeFormulaString`.
Encoding quotes the value and escapes backslash, double quote, newline and tab.
Decoding accepts one complete, valid String token's raw text, including the empty string,
and decodes `\\`, `\"`, `\n` and `\t` once.
Vite property chips decode raw token text and exclude member calls and incomplete property calls.

## Compatibility notes

Illegal escapes stop lexical scanning without producing a String token; earlier tokens remain.
WASM Token DTOs retain `kind`, raw `text`, and UTF-16 `span`.

## Links

- [WASM and SDK contract](../specs/wasm-api.md)

---
doc_id: changelog.20260831-postfix-call-validation
title: "Validate postfix calls"
language: en
source_language: en
counterpart: ./20260831-postfix-call-validation.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-08-31
---

# Postfix call validation

[简体中文](20260831-postfix-call-validation.zh-CN.md)

- Type: Fixed
- Component: analyzer

## Summary

Semantic analysis now reports unsupported and unknown postfix member calls instead of silently
leaving their type as `Unknown`. Supported postfix calls continue to use the same inference and
validation rules as their prefix form.

## Compatibility notes

- Formulas such as `true.sum()` and `true.noSuchFn()` now produce semantic diagnostics.
- Prefix calls and supported postfix calls are unchanged.

## Tests

- `cargo test -p analyzer`

## Links

- [Analyzer implementation](../how/analyzer/README.md)

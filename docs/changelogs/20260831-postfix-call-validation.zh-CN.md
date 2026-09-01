---
doc_id: changelog.20260831-postfix-call-validation
title: "校验 postfix call"
language: zh-CN
source_language: en
counterpart: ./20260831-postfix-call-validation.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-08-31
---

# 校验 postfix call

[English](20260831-postfix-call-validation.md)

- Type: Fixed
- Component: analyzer

## Summary

Semantic analysis 开始为不受支持和未知的 postfix member call 报告 diagnostic，不再静默地把它们的 type
保留为 `Unknown`。受支持的 postfix call 仍然使用与 prefix form 相同的 inference 和 validation rule。

## Compatibility notes

- `true.sum()` 和 `true.noSuchFn()` 等公式现在会产生 semantic diagnostic。
- Prefix call 和受支持的 postfix call 均未改变。

## Tests

- `cargo test -p analyzer`

## Links

- `docs/design/analyzer.md`

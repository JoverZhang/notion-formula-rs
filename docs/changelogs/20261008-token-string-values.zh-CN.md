---
doc_id: changelog.20261008-token-string-values
title: "提供字符串 token 的解码值"
language: zh-CN
source_language: en
counterpart: ./20261008-token-string-values.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-08
---

# 提供字符串 token 的解码值

[English](20261008-token-string-values.md)

- Type: Added
- Component: Analyzer、WASM API、Browser SDK

## Summary

原生 token 提供 `Token::string_value()`。WASM token 通过 `string_value` 返回有效字符串的解码值，
包括空字符串；其他 token 或无效转义返回 `null`。
SDK 导出 `quoteFormulaString`，用于生成公式字符串。Vite 字段标签使用解码值，排除成员调用和残缺的字段调用。

## Compatibility notes

token 的原始拼写、`kind` 和字节/UTF-16 区间保持不变。手动构造 WASM token DTO 的代码需要提供新增的
必填字段 `string_value: string | null`。

## Links

- [词法约定](../specs/formula-language.zh-CN.md)
- [WASM 与 SDK 约定](../specs/wasm-api.zh-CN.md)

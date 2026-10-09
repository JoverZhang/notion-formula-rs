---
doc_id: changelog.20261008-token-string-values
title: "新增 SDK 公式字符串编解码函数"
language: zh-CN
source_language: en
counterpart: ./20261008-token-string-values.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-09
---

# 新增 SDK 公式字符串编解码函数

[English](20261008-token-string-values.md)

- Type: Added
- Component: Browser SDK、Vite 编辑器

## Summary

SDK 导出纯同步函数 `decodeFormulaString` 和 `quoteFormulaString`。
解码接受完整的双引号公式字面量，包括空字符串，支持 `\\`、`\"`、`\n` 和 `\t` 转义；
引号或转义无效时返回 `null`。
Vite 字段标签解码 token 的原始文本，排除成员调用和残缺的字段调用。

## Compatibility notes

没有破坏性变更。WASM Token DTO 保留 `kind`、原始 `text` 和 UTF-16 `span`。

## Links

- [WASM 与 SDK 约定](../specs/wasm-api.zh-CN.md)

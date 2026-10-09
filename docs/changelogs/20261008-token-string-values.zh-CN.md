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
- Component: Analyzer、Browser SDK、Vite 编辑器

## Summary

SDK 导出纯同步函数 `encodeFormulaString` 和 `decodeFormulaString`。
编码为值加上双引号，并转义反斜杠、双引号、换行和 tab。
解码接受一个完整、有效的 String token 的原始文本，包括空字符串，
只解码一次 `\\`、`\"`、`\n` 和 `\t`。
Vite 字段标签解码 token 的原始文本，排除成员调用和残缺的字段调用。

## Compatibility notes

遇到非法转义时，词法扫描停止，不生成该 String token，保留此前的 token。
WASM Token DTO 保留 `kind`、原始 `text` 和 UTF-16 `span`。

## Links

- [WASM 与 SDK 约定](../specs/wasm-api.zh-CN.md)

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

SDK 导出纯同步函数 `encodeFormulaString` 和 `decodeFormulaString`。
编码为值加上双引号，并转义反斜杠、双引号、换行和 tab。
解码接受一个完整 String token 的原始文本，包括空字符串，只解码一次：
`\n` 变为换行，`\t` 变为 tab，其他转义产生其后的字符本身。
Vite 字段标签解码 token 的原始文本，排除成员调用和残缺的字段调用。

## Compatibility notes

接受 `\q` 等原样字符转义，`\q` 解码为 `q`。
WASM Token DTO 保留 `kind`、原始 `text` 和 UTF-16 `span`。

## Links

- [WASM 与 SDK 约定](../specs/wasm-api.zh-CN.md)

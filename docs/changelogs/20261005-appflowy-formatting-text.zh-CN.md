---
doc_id: changelog.20261005-appflowy-formatting-text
title: "补齐原生公式格式化与文本兼容性"
language: zh-CN
source_language: en
counterpart: ./20261005-appflowy-formatting-text.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 补齐原生公式格式化与文本兼容性

[English](20261005-appflowy-formatting-text.md)

- Type: Added
- Component: FormulaEngine、Draft 分析与内置函数

## Summary

原生公式新增保留纯文本的 `style`/`unstyle`。`formatNumber` 允许省略 precision 并支持
`humanize`；`formatDate` 支持方括号字面量、周编号、序数与星期标记。字符串加法现推断为 String。

## Compatibility notes

- 样式名称照常求值并检查类型，随后忽略；普通 null 文本仍返回 null。
- `concat` 仍至少接受两个列表，标量文本拼接使用 `+`。`link` 仍不支持。
- Engine 与 WASM DTO 结构不变。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin 声明](../specs/builtin-functions.zh-CN.md)
- [语言规则](../specs/formula-language.zh-CN.md)

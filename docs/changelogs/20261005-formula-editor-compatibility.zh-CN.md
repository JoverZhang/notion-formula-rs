---
doc_id: changelog.20261005-formula-editor-compatibility
title: "支持仅由接收者提供实参的公式方法"
language: zh-CN
source_language: en
counterpart: ./20261005-formula-editor-compatibility.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 支持仅由接收者提供实参的公式方法

[English](20261005-formula-editor-compatibility.md)

- Type: Changed
- Component: 公式分析、求值与编辑服务

## Summary

接收者可以提供 builtin 的唯一实参。`[1, 2, 3].sum()`、`"hello".length()` 等调用
现在与对应的前缀形式等价。补全与签名帮助共用这条适用规则。

## Compatibility notes

此前被拒绝的这些方法现在可以使用。接收者类型与剩余实参仍须满足 builtin 声明。
`now()`、`id()` 等没有接收者槽位的函数，以及特殊语法 `prop()`，仍不能作为方法调用。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin 调用契约](../specs/builtin-functions.zh-CN.md#调用与执行)

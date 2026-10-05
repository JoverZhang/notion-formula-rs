---
doc_id: changelog.20261005-formula-editor-compatibility
title: "提供属性引用信息并支持仅由接收者提供实参的公式方法"
language: zh-CN
source_language: en
counterpart: ./20261005-formula-editor-compatibility.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 提供属性引用信息并支持仅由接收者提供实参的公式方法

[English](20261005-formula-editor-compatibility.md)

- Type: Changed
- Component: 公式分析、求值与编辑服务

## Summary

接收者可以提供 builtin 的唯一实参。`[1, 2, 3].sum()`、`"hello".length()` 等调用
现在与对应的前缀形式等价。补全与签名帮助共用这条适用规则。

Draft 快照现在提供解码后的属性 ID，以及调用和字符串字面量的准确区间。编辑器可以保持字段绑定，替换 ID 时保留注释，无需再实现公式解析器。原生区间采用 UTF-8，WASM 区间采用 UTF-16；表达式其他位置出错时，仍会返回完整的属性引用。

## Compatibility notes

此前被拒绝的这些方法现在可以使用。接收者类型与剩余实参仍须满足 builtin 声明。
`now()`、`id()` 等没有接收者槽位的函数，以及特殊语法 `prop()`，仍不能作为方法调用。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin 调用契约](../specs/builtin-functions.zh-CN.md#调用与执行)
- [Draft 引用契约](../specs/ide.zh-CN.md#formuladraft)

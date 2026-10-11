---
doc_id: changelog.20261011-sequential-lets
title: "新增 lets 顺序绑定"
language: zh-CN
source_language: en
counterpart: ./20261011-sequential-lets.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# 新增 lets 顺序绑定

[English](20261011-sequential-lets.md)

- Type: Added
- Component: 公式分析与求值

## Summary

`lets` 支持各值类型独立的顺序绑定。后续绑定可以读取或遮蔽已有名称，最终表达式的类型决定返回类型。

## Compatibility notes

原先不受支持的 `lets` 调用现在可以求值。绑定位置仍须使用标识符，调用须包含至少一组完整绑定，
并以结果表达式结尾。

## Links

- [兼容性需求 #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [内置函数契约](../specs/builtin-functions.zh-CN.md#general)

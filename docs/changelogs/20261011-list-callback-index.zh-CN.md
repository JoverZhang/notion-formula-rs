---
doc_id: changelog.20261011-list-callback-index
title: "列表回调新增从零开始的 index"
language: zh-CN
source_language: en
counterpart: ./20261011-list-callback-index.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# 列表回调新增从零开始的 index

[English](20261011-list-callback-index.md)

- Type: Added
- Component: 列表回调

## Summary

`map`、`filter`、`find`、`findIndex`、`some`、`every` 和 `count` 现在除 `current` 外还隐式绑定从零开始的 `index`。嵌套回调会遮蔽并恢复这两个名称，null 元素仍占据原输入位置。

## Compatibility notes

- 回调中的 `index` 现在会遮蔽外层同名词法绑定；回调结束后恢复外层绑定。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [内置函数契约](../specs/builtin-functions.zh-CN.md#list)

---
doc_id: changelog.20261005-appflowy-formulas
title: "AppFlowy 示例改用原生公式引擎"
language: zh-CN
source_language: en
counterpart: ./20261005-appflowy-formulas.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# AppFlowy 示例改用原生公式引擎

[English](20261005-appflowy-formulas.md)

- Type: Changed
- Component: AppFlowy 示例

## Summary

固定版本的 AppFlowy-Web 示例使用 `@notion-formula/sdk` 处理已保存公式、编辑器分析与预览、
筛选、排序、汇总、Rollup、历史视图及字段转换，并移除了原有的 JavaScript 公式解析器和求值器。

## Compatibility notes

保留 Yjs 存储和字段 ID 引用。已有公式统一使用当前构建所固定的 SDK 语义。
示例的[接入指南](../../examples/appflowy-web/doc/NOTION_FORMULA.md)记录兼容性决定及编辑器行为。
其他 AppFlowy 客户端不在本次迁移范围内。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [AppFlowy 接入 PR](https://github.com/JoverZhang/AppFlowy-Web/pull/1)
- [构建与验证示例](../contributing/appflowy-example.zh-CN.md)

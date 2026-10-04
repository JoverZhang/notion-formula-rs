---
doc_id: changelog.20261004-demo-engine-drafts
title: "浏览器演示计算已保存的公式"
language: zh-CN
source_language: en
counterpart: ./20261004-demo-engine-drafts.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-04
---

# 浏览器演示计算已保存的公式

[English](20261004-demo-engine-drafts.md)

- Type: Changed
- Component: Browser demo

## Summary

演示页面现在异步分析 Draft，并对全部样本行计算已保存的公式及其共享依赖。Save 保存表达式并重新计算表格；
Discard 恢复已保存的表达式。保存过程中，其他编辑器保留各自尚未保存的修改。

表格显示真实结果、普通 null、NotReady 公式，以及带有来源 Formula ID 的行错误。嵌套 null、非有限数、
带符号的零和精确日期保留各自的运行时表示。

## Compatibility notes

- 编辑 Draft 后，表格在 Save 时才更新。此次演示迁移不改变公开 Rust 和 WASM API。

## Links

- [Issue #64](https://github.com/JoverZhang/notion-formula-rs/issues/64)
- [当前 WASM API](../specs/wasm-api.zh-CN.md)

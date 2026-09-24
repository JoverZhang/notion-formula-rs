---
doc_id: changelog.20260924-formula-engine-definitions
title: "新增 FormulaEngine 定义管理"
language: zh-CN
source_language: en
counterpart: ./20260924-formula-engine-definitions.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-09-24
---

# 新增 FormulaEngine 定义管理

[English](20260924-formula-engine-definitions.md)

- Type: Added
- Component: FormulaEngine Rust API

## Summary

`FormulaEngine` 现在可以保存 Input 和 Formula 定义、返回独立的属性快照，并在定义变更后报告推断输出类型、
依赖就绪状态和依赖环。

## Compatibility notes

- 此次新增 Rust API 不改变现有 API 或数据格式，无需迁移。

## Links

- [Issue #50](https://github.com/JoverZhang/notion-formula-rs/issues/50)
- [PR #52](https://github.com/JoverZhang/notion-formula-rs/pull/52)
- [当前 FormulaEngine 规范](../specs/formula-engine.zh-CN.md)

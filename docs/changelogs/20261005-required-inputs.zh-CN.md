---
doc_id: changelog.20261005-required-inputs
title: "查询公式的输入依赖"
language: zh-CN
source_language: en
counterpart: ./20261005-required-inputs.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# 查询公式的输入依赖

[English](20261005-required-inputs.md)

- Type: Added
- Component: FormulaEngine、WASM 与浏览器 SDK

## Summary

`FormulaEngine::required_inputs`、WASM session 的 `required_inputs` 和浏览器 client 的
`requiredInputs` 提供所选公式的传递 Input 依赖，供宿主选择需要加载的外部数据。
Ready 公式的静态依赖集合完整；NotReady 公式只返回已知引用。

## Compatibility notes

查询是新增能力。求值仍要求提供全部已注册 Input 的列；无效查询目标复用
`EvaluateInputError` / `EVALUATE_INPUT` 契约。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current FormulaEngine](../specs/formula-engine.zh-CN.md)
- [Current WASM API](../specs/wasm-api.zh-CN.md)

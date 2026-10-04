---
doc_id: changelog.20261005-formula-sdk
title: "复用公式客户端并查询输入依赖"
language: zh-CN
source_language: en
counterpart: ./20261005-formula-sdk.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 复用公式客户端并查询输入依赖

[English](20261005-formula-sdk.md)

- Type: Added
- Component: FormulaEngine 与浏览器 SDK

## Summary

`@notion-formula/sdk` 打包 Engine/Draft client、module Worker、TypeScript 声明和 WASM 资源。
Vite 示例通过此包使用这些能力。仓库还以 submodule 固定 AppFlowy-Web 示例，并提供本地 SDK
复制与构建命令，支持后续迁移。

`FormulaEngine::required_inputs`、WASM session 的 `required_inputs` 和浏览器 client 的
`requiredInputs` 提供所选公式的传递 Input 依赖。Ready 公式的静态依赖集合完整；NotReady 公式只返回已知引用。

## Compatibility notes

- 从 `@notion-formula/sdk` 导入可复用的浏览器客户端与 Engine DTO，替代 `examples/vite` 下的文件路径。
  既有求值、FIFO、Draft 和资源释放行为保持不变。
- 查询是新增能力。求值仍要求提供全部已注册 Input 的列；无效查询目标复用
  `EvaluateInputError` / `EVALUATE_INPUT` 契约。
- 加入 AppFlowy submodule 不代表公式迁移完成；进度由 #67 跟踪。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current FormulaEngine](../specs/formula-engine.zh-CN.md)
- [Current WASM API](../specs/wasm-api.zh-CN.md)
- [构建 AppFlowy 示例](../contributing/appflowy-example.zh-CN.md)

---
doc_id: changelog.20261005-formula-sdk
title: "以浏览器包复用公式客户端"
language: zh-CN
source_language: en
counterpart: ./20261005-formula-sdk.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# 以浏览器包复用公式客户端

[English](20261005-formula-sdk.md)

- Type: Added
- Component: 浏览器 SDK

## Summary

`@notion-formula/sdk` 打包既有的 Engine/Draft client、module Worker、TypeScript 声明和 WASM 资源。
Vite 示例通过此包使用这些能力。仓库还以 submodule 固定 AppFlowy-Web 示例，并提供本地 SDK
复制与构建命令，支持后续迁移。

## Compatibility notes

- 从 `@notion-formula/sdk` 导入可复用的浏览器客户端与 Engine DTO，替代 `examples/vite` 下的文件路径。
  求值、FIFO、Draft 和资源释放行为保持不变。
- 加入 AppFlowy submodule 不代表公式迁移完成；进度由 #67 跟踪。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current WASM API](../specs/wasm-api.zh-CN.md)
- [构建 AppFlowy 示例](../contributing/appflowy-example.zh-CN.md)

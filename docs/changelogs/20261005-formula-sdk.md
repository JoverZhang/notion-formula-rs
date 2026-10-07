---
doc_id: changelog.20261005-formula-sdk
title: "Reuse formula clients as a browser package"
language: en
source_language: en
counterpart: ./20261005-formula-sdk.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# Reuse formula clients as a browser package

[简体中文](20261005-formula-sdk.zh-CN.md)

- Type: Added
- Component: Browser SDK

## Summary

`@notion-formula/sdk` packages the existing Engine/Draft client, module Worker,
TypeScript declarations, and WASM assets. The Vite example consumes this package.
The repository also pins AppFlowy-Web as an example submodule and provides local
SDK staging and build commands for its migration.

## Compatibility notes

- Import reusable browser clients and Engine DTOs from `@notion-formula/sdk` instead
  of files under `examples/vite`. Evaluation, FIFO, Draft, and resource-release behavior is preserved.
- Adding the AppFlowy submodule does not complete its formula migration; progress is tracked in #67.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current WASM API](../specs/wasm-api.md)
- [Build the AppFlowy example](../contributing/appflowy-example.md)

---
doc_id: changelog.20260210-disable-partial-format-on-syntax-errors
title: "在语法错误时禁用局部格式化"
language: zh-CN
source_language: en
counterpart: ./20260210-disable-partial-format-on-syntax-errors.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-10
---

# 在语法错误时禁用局部格式化

[English](20260210-disable-partial-format-on-syntax-errors.md)

- Type: Fixed
- Component: analyzer_wasm

## Summary

Formatting 行为变得更严格：

- 输入存在语法错误时，不再生成局部 formatter 输出；
- 语法有效时仍然可以格式化；
- 这为严格应用 edit 和 cursor rebasing 奠定了基础。

## Compatibility notes

- 对语法错误执行 formatting 时会失败，不再返回局部文本；
- 该行为成为严格 `format(..., cursor)` contract 的一部分。

## Tests

- `cargo test -p analyzer_wasm`
- `cargo test -p analyzer`

## Links

- [WASM boundary 实现](../how/analyzer_wasm/README.zh-CN.md)
- [Vite example 实现](../how/examples/vite/README.zh-CN.md)

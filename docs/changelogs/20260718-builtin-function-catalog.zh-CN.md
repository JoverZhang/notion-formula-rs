---
doc_id: changelog.20260718-builtin-function-catalog
title: "建立内置函数 catalog"
language: zh-CN
source_language: en
counterpart: ./20260718-builtin-function-catalog.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-18
---

# 建立内置函数 catalog

[English](20260718-builtin-function-catalog.md)

- Type: Changed
- Component: other

## Summary

`builtin_categories()` 开始公开完整且有序的 builtin catalog，`builtins_functions()` 则公开其中受支持的
semantic signature。Analyzer diagnostic 与 IDE Signature Help 开始共用 `resolve_call_signature()` 的解析
结果，包括不完整调用、repeat group、generic binding、分阶段 lambda inference 和 resolver 调整后的 return
type。

## Compatibility notes

- `builtin_categories()` 公开受支持及仅记录在案但不受支持的声明；`builtins_functions()` 仍然只公开可执行的
  semantic signature。
- `SemanticMap::builtin_calls` 保留最终的共享解析结果，供 downstream consumer 使用。
- `concat` 改为至少需要两个 repeat group；`id()` 是 catalog 定义的零参数 current-row 形式。
- `name`、`email`、`lets`、不可用的 rich/date-range type 和仅供 operator 使用的声明仍会记录，但不进入
  executable signature 或 completion。
- 推断 implicit lambda 的 Analyzer 入口开始接收 mutable expression，从而在 `SemanticMap` 中保留最终转换
  后的 call 及其 `ResolvedFunctionSig`。
- `BuiltinSigParser` 仍可作为独立 compatibility API 使用，但 production builtin declaration 不再于 runtime
  解析 signature string。

## Tests

- `cargo test -p builtin_fn`
- `cargo test -p analyzer`
- `cargo test -p ide`
- `cargo run -q -p builtin_fn --bin builtin_catalog -- --check`
- `just docker-test`

## Links

- [`docs/design/builtin-fn.md`](../design/builtin-fn.md)
- [`docs/design/evaluator.md`](../design/evaluator.md)
- [`docs/design/contracts.md`](../design/contracts.md)
- [`builtin_fn/README.md`](../../builtin_fn/README.md)

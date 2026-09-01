---
doc_id: changelog.20260720-builtin-function-runtime
title: "加入内置函数 runtime"
language: zh-CN
source_language: en
counterpart: ./20260720-builtin-function-runtime.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-20
---

# 加入内置函数 runtime

[English](20260720-builtin-function-runtime.md)

- Type: Added
- Component: evaluator

## Summary

全部 83 项受支持的 builtin declaration 开始通过同步的 prepared-input Evaluator 执行。Value function 会分别
保留 null、error 和 mask state，conditional 与 list lambda 则只对选中的 row mask 求值。Generated typed
argument 到 handwritten kernel boundary 时仍然保留具体类型，不会重新擦除为 dynamic column。

## Compatibility notes

- Public Evaluator API 和 catalog syntax 均未改变。
- `now()` 和 `today()` 使用 frozen `BuiltinRuntimeContext`，`id()` 使用当前 `RowBatch` 的 row ID。
- Catalog 中标记为 unsupported 的条目仍可供文档和 analysis 使用，但不承担 runtime dispatch obligation。

## Tests

- `cargo test -p evaluator`
- `cargo test -p evaluator --release`
- `cargo run -p builtin_fn --bin builtin_catalog -- --check`
- Workspace 和 Docker verification 记录在对应 pull request 中。

## Links

- [`docs/design/builtin-fn.md`](../design/builtin-fn.md)
- [`docs/design/evaluator.md`](../design/evaluator.md)
- [`docs/design/contracts.md`](../design/contracts.md)
- [`evaluator/README.md`](../../evaluator/README.md)

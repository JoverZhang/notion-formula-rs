---
doc_id: changelog.20260718-evaluator-generated-structure
title: "生成 Evaluator 结构"
language: zh-CN
source_language: en
counterpart: ./20260718-evaluator-generated-structure.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-18
---

# 生成 Evaluator 结构

[English](20260718-evaluator-generated-structure.md)

- Type: Changed
- Component: evaluator

## Summary

`prepare_formula()` 开始生成同步的 `PreparedFormula`，其中包含完整的 input dependency；
`EvalInputsBuilder` 则在 runtime 执行前校验调用方准备的 typed column。受支持的 builtin catalog 开始在
编译时生成类型明确且完备的 implementation 与 dispatch contract。

## Compatibility notes

- Async `Evaluator`/`Provider` boundary 替换为 `prepare_formula()`、
  `PreparedFormula::required_columns()`、`EvalInputsBuilder` 和同步的
  `PreparedFormula::evaluate()`。
- `EvalInputsBuilder::finish()` 会在 evaluation 开始前报告 input contract 中的缺失、重复、kind 错误、长度
  错误和 layout 错误。
- `RowBatch` 拥有 string 类型的 `RowId` value。Finalized input 中保存的 immutable
  `BuiltinRuntimeContext` 提供 evaluation time 和 timezone。
- `KernelColumn<K>` 把共享 storage 与 null `Validity` 分开；execution `Mask` 与逐行 `ok` 保持独立。
- Generated trait、marker、typed Args/Plans 和 dispatch 会让缺失实现或不兼容的 method signature 产生
  compile error。在对应行为落地之前，builtin implementation body 仍然是
  `evaluator/src/builtins/implementations.rs` 中有意保留的 `todo!()`。

## Tests

- `cargo test -p evaluator --test generated_contract`
- `cargo test -p evaluator --test runtime_structure`
- `cargo test -p evaluator`
- `cargo check --workspace`
- `cargo test --workspace`
- `just docker-test`

## Links

- [`docs/design/builtin-fn.md`](../design/builtin-fn.md)
- [`docs/design/evaluator.md`](../design/evaluator.md)
- [`docs/design/contracts.md`](../design/contracts.md)
- [`evaluator/README.md`](../../evaluator/README.md)

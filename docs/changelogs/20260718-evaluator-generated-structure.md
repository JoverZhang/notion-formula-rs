---
doc_id: changelog.20260718-evaluator-generated-structure
title: "Generate the evaluator structure"
language: en
source_language: en
counterpart: ./20260718-evaluator-generated-structure.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-18
---

# 20260718-evaluator-generated-structure

[简体中文](20260718-evaluator-generated-structure.zh-CN.md)

- Type: Changed
- Component: evaluator

## Summary

`prepare_formula()` now produces a synchronous `PreparedFormula` with complete input
dependencies, while `EvalInputsBuilder` validates caller-prepared typed columns before any
runtime work. The supported builtin catalog now generates exhaustive typed implementation
and dispatch contracts during compilation.

## Compatibility notes

- The async `Evaluator`/`Provider` boundary is replaced by `prepare_formula()`,
  `PreparedFormula::required_columns()`, `EvalInputsBuilder`, and synchronous
  `PreparedFormula::evaluate()`.
- `EvalInputsBuilder::finish()` reports missing, duplicate, wrong-kind, wrong-length, and
  wrong-layout input contract failures before evaluation starts.
- `RowBatch` owns string `RowId` values. Evaluation time and timezone are supplied through
  the immutable `BuiltinRuntimeContext` stored with finalized inputs.
- `KernelColumn<K>` separates shared storage from null `Validity`; execution `Mask` and row
  `ok` remain independent.
- Generated traits, markers, typed Args/Plans, and dispatch make missing implementations or
  incompatible method signatures compile errors. Builtin implementation bodies remain
  intentional `todo!()` entries in `evaluator/src/builtins/implementations.rs` until the
  behavior change lands.

## Tests

- `cargo test -p evaluator --test generated_contract`
- `cargo test -p evaluator --test runtime_structure`
- `cargo test -p evaluator`
- `cargo check --workspace`
- `cargo test --workspace`
- `just docker-test`

## Links

- [Builtin declaration implementation](../how/builtin_fn/README.md)
- [Evaluator implementation](../how/evaluator/README.md)
- [Builtin function specification](../specs/builtin-functions/README.md)
- [`evaluator/README.md`](../../evaluator/README.md)

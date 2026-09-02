---
doc_id: changelog.20260720-builtin-function-runtime
title: "Add the builtin function runtime"
language: en
source_language: en
counterpart: ./20260720-builtin-function-runtime.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-20
---

# 20260720-builtin-function-runtime

[简体中文](20260720-builtin-function-runtime.zh-CN.md)

- Type: Added
- Component: evaluator

## Summary

All 83 supported builtin declarations now execute through the synchronous prepared-input
evaluator. Value functions preserve independent null/error/mask state, while conditionals and
list lambdas evaluate only their selected row masks. Generated typed arguments remain typed at
the handwritten kernel boundary instead of being erased back to dynamic columns.

## Compatibility notes

- No public evaluator API or catalog syntax changed.
- `now()` and `today()` use the frozen `BuiltinRuntimeContext`, and `id()` uses the current
  `RowBatch` row IDs.
- Catalog entries marked unsupported remain available to documentation and analysis but do not
  have runtime dispatch obligations.

## Tests

These commands record verification at the landing date; they are not the Current test recipe.

- `cargo test -p evaluator`
- `cargo test -p evaluator --release`
- `cargo run -p builtin_fn --bin builtin_catalog -- --check`
- Workspace and Docker verification are recorded in the pull request.

## Links

- [Builtin declaration implementation](../how/builtin_fn/README.md)
- [Evaluator implementation](../how/evaluator/README.md)
- [Builtin function specification](../specs/builtin-functions/README.md)
- [`evaluator/README.md`](../../evaluator/README.md)

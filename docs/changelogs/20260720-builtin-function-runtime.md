# 20260720-builtin-function-runtime

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

- `cargo test -p evaluator`
- `cargo test -p evaluator --release`
- `cargo run -p builtin_fn --bin builtin_catalog -- --check`
- Workspace and Docker verification are recorded in the pull request.

## Links

- [`docs/design/builtin-fn.md`](../design/builtin-fn.md)
- [`docs/design/evaluator.md`](../design/evaluator.md)
- [`docs/design/contracts.md`](../design/contracts.md)
- [`evaluator/README.md`](../../evaluator/README.md)

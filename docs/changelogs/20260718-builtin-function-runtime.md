# 20260718-builtin-function-runtime

- Type: Changed
- Component: analyzer, evaluator, other

## Summary

Builtin functions now come from one compile-time category DSL that drives the catalog,
semantic signatures, shared call resolution, IDE presentation, generated evaluator
contracts, and the rendered builtin reference. The evaluator now prepares a complete input
manifest and executes typed row batches synchronously with lazy controlled builtins.

## Compatibility notes

- Breaking Rust evaluator API: the prototype `Evaluator`, async `Provider`, provider errors,
  and registry-based execution boundary were removed. Hosts now call `prepare_formula`, load
  every `PreparedFormula::required_columns()` entry, finalize an `EvalInputsBuilder`, and
  call `PreparedFormula::evaluate` or `evaluate_with_mask` with a `RowBatch`.
- Runtime system data is explicit and frozen per evaluation:
  `BuiltinRuntimeContext` supplies time/timezone, while `RowBatch` supplies row IDs for
  `id()`.
- Unsupported declarations remain documented in `builtin_categories()` but are excluded
  from `builtins_functions()` and evaluator generation. This currently includes
  `name`, `email`, `lets`, unavailable rich/date-range types, and operator-only forms.
- Analyzer consumers that need evaluator planning facts can use `SemanticMap`, which now
  retains final `ResolvedFunctionSig` records in addition to expression types.

## Tests

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets`
- `cargo test --workspace`
- `cargo run -q -p builtin_fn --bin builtin_catalog -- --check`
- `just docker-test`

## Links

- [`docs/design/builtin-fn.md`](../design/builtin-fn.md)
- [`docs/design/evaluator.md`](../design/evaluator.md)
- [`docs/design/contracts.md`](../design/contracts.md)
- [`builtin_fn/README.md`](../../builtin_fn/README.md)
- [`evaluator/README.md`](../../evaluator/README.md)

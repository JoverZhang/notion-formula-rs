# 20260718-builtin-function-catalog

- Type: Changed
- Component: other

## Summary

`builtin_categories()` now exposes the complete ordered builtin catalog, and
`builtins_functions()` exposes its supported semantic signatures. Analyzer diagnostics and
IDE Signature Help now use the same `resolve_call_signature()` result for incomplete calls,
repeat groups, generic binding, staged lambda inference, and resolver-refined return types.

## Compatibility notes

- `builtin_categories()` exposes supported and documented unsupported declarations;
  `builtins_functions()` continues to expose only executable semantic signatures.
- `SemanticMap::builtin_calls` retains the final shared resolution for downstream consumers.
- `concat` now requires at least two repeat groups, and `id()` is the zero-argument current-row
  form defined by the catalog.
- `name`, `email`, `lets`, unavailable rich/date-range types, and operator-only declarations
  remain documented but are excluded from executable signatures and completion.
- Analyzer entry points that infer implicit lambdas accept a mutable expression so the final
  transformed call and its `ResolvedFunctionSig` can be retained in `SemanticMap`.
- `BuiltinSigParser` remains available as a standalone compatibility API, but production
  builtin declarations no longer parse signature strings at runtime.

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

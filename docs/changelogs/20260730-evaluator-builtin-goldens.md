# Evaluator builtin goldens

- Type: Changed
- Component: evaluator

## Summary

Evaluator runtime behavior is now documented and verified by readable golden fixtures for
every supported builtin. Fixtures can include typed property columns, row IDs, execution
masks, and a frozen runtime context alongside the formula and per-row result.

## Compatibility notes

- No public Rust interface or formula behavior changed.

## Tests

- `cargo test -p evaluator --test builtin_golden`

## Links

- `docs/design/builtin-fn.md`
- `docs/design/evaluator.md`

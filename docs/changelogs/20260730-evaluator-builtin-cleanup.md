# 20260730-evaluator-builtin-cleanup

- Type: Changed
- Component: evaluator

## Summary

Evaluator builtin dispatch support now centralizes parameter lookup and debug contract
checking behind focused internal modules. Generated builtin contracts and formula results
remain unchanged.

## Compatibility notes

- No builtin DSL, generated ABI, public Rust API, catalog order, or runtime behavior changed.

## Tests

- `cargo test -p evaluator`
- `cargo test -p evaluator --release`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p evaluator --no-deps`
- `just docker-test`

## Links

- `docs/design/builtin-fn.md`
- `docs/design/evaluator.md`

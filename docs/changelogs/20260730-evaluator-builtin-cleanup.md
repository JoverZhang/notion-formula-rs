---
doc_id: changelog.20260730-evaluator-builtin-cleanup
title: "Clean up evaluator builtin internals"
language: en
source_language: en
counterpart: ./20260730-evaluator-builtin-cleanup.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-30
---

# 20260730-evaluator-builtin-cleanup

[简体中文](20260730-evaluator-builtin-cleanup.zh-CN.md)

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

- [Builtin declaration implementation](../how/builtin_fn/README.md)
- [Evaluator implementation](../how/evaluator/README.md)

---
doc_id: changelog.20260730-evaluator-builtin-cleanup
title: "整理 Evaluator 的 builtin 内部实现"
language: zh-CN
source_language: en
counterpart: ./20260730-evaluator-builtin-cleanup.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-30
---

# 整理 Evaluator 的 builtin 内部实现

[English](20260730-evaluator-builtin-cleanup.md)

- Type: Changed
- Component: evaluator

## Summary

Evaluator 的 builtin dispatch support 开始通过专门的内部模块集中处理 parameter lookup 和 debug contract
checking。Generated builtin contract 和 formula result 均未改变。

## Compatibility notes

- Builtin DSL、generated ABI、public Rust API、catalog 顺序和 runtime 行为均未改变。

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

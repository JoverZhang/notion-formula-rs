---
doc_id: changelog.20260730-evaluator-builtin-goldens
title: "加入 Evaluator builtin golden fixture"
language: zh-CN
source_language: en
counterpart: ./20260730-evaluator-builtin-goldens.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-07-30
---

# 加入 Evaluator builtin golden fixture

[English](20260730-evaluator-builtin-goldens.md)

- Type: Changed
- Component: evaluator

## Summary

每一项受支持 builtin 的 Evaluator runtime 行为开始由易于阅读的 golden fixture 记录和验证。除了 formula
和逐行 result，fixture 还可以包含 typed property column、row ID、execution mask 和 frozen runtime
context。

## Compatibility notes

- Public Rust interface 和 formula 行为均未改变。

## Tests

- `cargo test -p evaluator --test builtin_golden`

## Links

- [Builtin declaration 实现](../how/builtin_fn/README.zh-CN.md)
- [Evaluator 实现](../how/evaluator/README.zh-CN.md)

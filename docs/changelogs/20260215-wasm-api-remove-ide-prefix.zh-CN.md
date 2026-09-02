---
doc_id: changelog.20260215-wasm-api-remove-ide-prefix
title: "移除 WASM API 的 IDE 前缀"
language: zh-CN
source_language: en
counterpart: ./20260215-wasm-api-remove-ide-prefix.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-15
---

# 移除 WASM API 的 IDE 前缀

[English](20260215-wasm-api-remove-ide-prefix.md)

- Type: Changed
- Component: analyzer_wasm, examples/vite, docs

## Summary

Stateful WASM `Analyzer` 的 method 从 `ide_*` 改为不带前缀的 `format`、`apply_edits` 和 `help`。Vite
demo wrapper 也同步更新，使 proxy function 的名称直接对应底层 WASM method。

## Compatibility notes

- Breaking WASM API：
  - `Analyzer.ide_format(...)` 改为 `Analyzer.format(...)`；
  - `Analyzer.ide_apply_edits(...)` 改为 `Analyzer.apply_edits(...)`；
  - `Analyzer.ide_help(...)` 改为 `Analyzer.help(...)`。
- Breaking demo wrapper API：
  - `analyzeSource` 改为 `analyze`；
  - `formatSource` 改为 `format`；
  - `applyEditsSource` 改为 `apply_edits`；
  - `helpSource` 改为 `help`。

## Tests

- `pnpm -C examples/vite wasm:build`
- `cargo test -p analyzer_wasm`
- `wasm-pack test --node analyzer_wasm`
- `pnpm -C examples/vite -s run test -- tests/unit/wasm_errors.test.ts tests/unit/signature_help_instantiated.test.ts tests/unit/completion_preferred_indices.test.ts`
- `pnpm -C examples/vite -s run check`

## Links

- `analyzer_wasm/src/lib.rs`
- `analyzer_wasm/tests/analyze.rs`
- `examples/vite/src/analyzer/wasm_client.ts`
- `examples/vite/src/vm/app_vm.ts`
- `examples/vite/src/ui/formula_panel_view.ts`

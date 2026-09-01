---
doc_id: changelog.20260213-refactor-analyzer-ide-wasm-entrypoints
title: "重构 Analyzer、IDE 与 WASM 入口"
language: zh-CN
source_language: en
counterpart: ./20260213-refactor-analyzer-ide-wasm-entrypoints.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-13
---

# 重构 Analyzer、IDE 与 WASM 入口

[English](20260213-refactor-analyzer-ide-wasm-entrypoints.md)

- Type: Changed
- Component: analyzer, analyzer_wasm, examples/vite, docs

## Summary

入口分层经过重构：Analyzer 负责 syntax、semantic 和 IDE 逻辑，WASM 只处理 JSON/DTO 以及 UTF-16/UTF-8
转换。WASM exports 改为 `analyze`、`ide_format`、`ide_apply_edits` 和 `ide_help`。

## Compatibility notes

- Breaking Rust API：`analyzer::analyze(text)` 替换为：
  - `analyzer::analyze_syntax(text) -> SyntaxResult`；
  - `analyzer::analyze(text, ctx) -> AnalyzeResult`。
- Breaking WASM API：删除 `format`、`apply_edits` 和 `complete`，替换为：
  - `ide_format(source, cursor_utf16)`；
  - `ide_apply_edits(source, edits, cursor_utf16)`；
  - `ide_help(source, cursor_utf16, context_json)`。
- Breaking DTO：`CompletionOutput` 替换为：
  - `CompletionResult { items, replace, preferred_indices }`；
  - `HelpResult { completion, signature_help }`。
- `format`/`apply_edits` 的 core behavior 移入 `analyzer::ide`（`analyzer/src/ide/edit.rs`）；WASM
  在坐标转换后转发调用。

## Tests

- `cargo test -p analyzer`
- `cargo test -p analyzer_wasm`
- `wasm-pack test --node analyzer_wasm`
- `pnpm -C examples/vite -s run wasm:build`
- `pnpm -C examples/vite -s run test`
- `pnpm -C examples/vite -s run test:e2e`

## Links

- `analyzer/src/lib.rs`
- `analyzer/src/ide/mod.rs`
- `analyzer/src/ide/edit.rs`
- `analyzer_wasm/src/lib.rs`
- `analyzer_wasm/src/dto/v1.rs`

---
doc_id: changelog.20260211-decouple-analyze-format-quick-fixes
title: "拆分 analysis、formatting 与 quick fix"
language: zh-CN
source_language: en
counterpart: ./20260211-decouple-analyze-format-quick-fixes.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-11
---

# 拆分 analysis、formatting 与 quick fix

[English](20260211-decouple-analyze-format-quick-fixes.md)

- Type: Changed
- Component: analyzer + analyzer_wasm + examples/vite

## Summary

这次修改对 editing 与 boundary API 做了 breaking refactor：

- Core diagnostic 直接携带 action：
  - `Diagnostic.actions: Vec<CodeAction>`；
  - 使用 byte coordinate 的 `CodeAction { title, edits: Vec<TextEdit> }`。
- Parser 开始生成 quick fix，并把它们写入 diagnostic action。
- `ParseOutput` 不再携带独立的 quick-fix list。
- WASM API 开始导出：
  - `analyze(source, context_json)`；
  - `format(source, cursor_utf16)`；
  - `apply_edits(source, edits, cursor_utf16)`；
  - `complete(source, cursor_utf16, context_json)`。
- `format` 和 `apply_edits` 失败时会抛出错误，成功时总是返回 `{ source, cursor }`。
- Vite quick-fix 流程开始从 `AnalyzeResult.diagnostics[].actions` 取得 action。

## Compatibility notes

- DTO 变化：
  - `DiagnosticView` 增加 `actions`；
  - `DiagnosticView` 增加供 UI 列表使用的 1-based `line` 和 `col`；
  - 新增 `ApplyResultView`。
- 删除旧 boundary endpoint 和旧 line/column endpoint。
- Completion、action 和 edit application 开始共用 `TextEdit` / `TextEditView` model。

## Tests

- `cargo test -p analyzer`
- `cargo test -p analyzer_wasm`
- `pnpm -C examples/vite test`

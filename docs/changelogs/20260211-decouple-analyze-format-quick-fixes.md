---
doc_id: changelog.20260211-decouple-analyze-format-quick-fixes
title: "Decouple analysis, formatting, and quick fixes"
language: en
source_language: en
counterpart: ./20260211-decouple-analyze-format-quick-fixes.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-11
---

# 20260211-decouple-analyze-format-quick-fixes

[简体中文](20260211-decouple-analyze-format-quick-fixes.zh-CN.md)

- Type: Changed
- Component: analyzer + analyzer_wasm + examples/vite

## Summary

Breaking refactor of editing/boundary APIs:

- Core diagnostics now carry actions directly:
  - `Diagnostic.actions: Vec<CodeAction>`
  - `CodeAction { title, edits: Vec<TextEdit> }` in byte coordinates.
- Parser quick-fix generation now populates diagnostic actions.
- `ParseOutput` no longer carries a separate quick-fix list.
- WASM API now exposes:
  - `analyze(source, context_json)`
  - `format(source, cursor_utf16)`
  - `apply_edits(source, edits, cursor_utf16)`
  - `complete(source, cursor_utf16, context_json)`
- `format` and `apply_edits` now throw on failure and always return `{ source, cursor }` on success.
- Vite quick-fix flow now derives actions from `AnalyzeResult.diagnostics[].actions`.

## Compatibility notes

- DTO changes:
  - `DiagnosticView` adds `actions`.
  - `DiagnosticView` now includes `line` and `col` (1-based) for UI lists.
  - Added `ApplyResultView`.
- Removed old boundary endpoints and old line/column endpoint.
- Edit model is unified as `TextEdit` / `TextEditView` across completion/actions/apply edits.

## Tests

- `cargo test -p analyzer`
- `cargo test -p analyzer_wasm`
- `pnpm -C examples/vite test`

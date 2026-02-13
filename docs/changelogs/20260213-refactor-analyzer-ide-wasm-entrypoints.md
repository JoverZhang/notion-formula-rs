# 20260213-refactor-analyzer-ide-wasm-entrypoints

- Type: Changed
- Component: analyzer, analyzer_wasm, examples/vite, docs

## Summary

Refactored entry-point layering so analyzer owns syntax/semantic/IDE logic and wasm only handles JSON/DTO plus UTF-16/UTF-8 conversion. WASM exports are now `analyze`, `ide_format`, `ide_apply_edits`, and `ide_help`.

## Compatibility notes

- Breaking (Rust API): `analyzer::analyze(text)` was replaced by:
  - `analyzer::analyze_syntax(text) -> SyntaxResult`
  - `analyzer::analyze(text, ctx) -> AnalyzeResult`
- Breaking (WASM API): removed `format`, `apply_edits`, `complete`; replaced by:
  - `ide_format(source, cursor_utf16)`
  - `ide_apply_edits(source, edits, cursor_utf16)`
  - `ide_help(source, cursor_utf16, context_json)`
- Breaking (DTO): replaced `CompletionOutput` with:
  - `CompletionResult { items, replace, preferred_indices }`
  - `HelpResult { completion, signature_help }`
- `format`/`apply_edits` core behavior moved into `analyzer::ide` (`analyzer/src/ide/edit.rs`); wasm now forwards after coordinate conversion.

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

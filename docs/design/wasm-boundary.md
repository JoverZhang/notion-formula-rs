# WASM boundary (`analyzer_wasm`)

This layer translates between editor coordinates (UTF-16) and core analyzer coordinates (UTF-8 bytes), and defines JS-facing DTOs.

## Exports

- `analyze(source, context_json) -> AnalyzeResult`
- `ide_format(source, cursor_utf16) -> ApplyResult`
- `ide_apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `ide_help(source, cursor_utf16, context_json) -> HelpResult`

Rust signatures (wasm-bindgen):
- `analyze(source: String, context_json: String) -> Result<JsValue, JsValue>`
- `ide_format(source: String, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `ide_apply_edits(source: String, edits: JsValue, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `ide_help(source: String, cursor_utf16: usize, context_json: String) -> Result<JsValue, JsValue>`

## Hard rules

- Core (`analyzer`) uses UTF-8 byte offsets only.
- WASM boundary is the only UTF-16 ↔ byte conversion layer.
- JS/WASM DTO spans and edits are UTF-16 code units.
- Half-open ranges everywhere: `[start, end)`.

## DTOs (v1)

- `AnalyzeResult { diagnostics, tokens, output_type }`
- `Diagnostic { kind, message, span, line, col, actions }`
- `CodeAction { title, edits }`
- `TextEdit { range, new_text }`
- `ApplyResult { source, cursor }`
- `CompletionResult { items, replace, preferred_indices }`
- `HelpResult { completion, signature_help }`

Diagnostics expose quick-fix actions directly as `actions`.
Diagnostics include 1-based `line`/`col` for UI lists. These are
computed from diagnostic byte offsets through `analyzer::SourceMap::line_col`.

## Formatting and edit application

- `ide_format(...)`:
  - validates UTF-16 cursor and converts to byte cursor
  - forwards to core `analyzer::ide_format(...)`
  - maps byte cursor in result back to UTF-16

- `ide_apply_edits(...)`:
  - accepts UTF-16 `TextEdit[]`
  - converts to byte edits
  - validates UTF-16 bounds + UTF-8 char boundaries
  - forwards to core `analyzer::ide_apply_edits(...)`
  - returns updated source + rebased cursor

Core edit behavior is implemented in `analyzer/src/ide/edit.rs`:
- syntax-error gating for format
- edit sorting and overlap checks
- shared byte-edit apply + cursor rebasing

## Validation rules (`ide_apply_edits`)

- each edit range must be inside UTF-16 document bounds
- converted byte ranges must be valid UTF-8 char boundaries

## Error model

- `analyze` and `ide_help` throw on invalid context JSON.
- `ide_format` and `ide_apply_edits` throw on operation failure (not encoded in payload).
- error messages are minimal and deterministic (`Format error`, `Invalid edits`, `Invalid edit range`, `Overlapping edits`, `Invalid cursor`).

## Context JSON contract

- non-empty valid JSON
- unknown top-level fields rejected
- schema: `{ properties: Property[], completion?: { preferred_limit?: number } }`

## Source pointers

- exports: `analyzer_wasm/src/lib.rs`
- conversion helpers: `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`
- DTOs: `analyzer_wasm/src/dto/v1.rs`
- core edit pipeline: `analyzer/src/ide/edit.rs`, `analyzer/src/text_edit.rs`

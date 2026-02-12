# WASM boundary (`analyzer_wasm`)

This layer translates between editor coordinates (UTF-16) and core analyzer coordinates (UTF-8 bytes), and defines JS-facing DTOs.

## Exports

- `analyze(source, context_json) -> AnalyzeResult`
- `format(source, cursor_utf16) -> ApplyResult`
- `apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `complete(source, cursor_utf16, context_json) -> CompletionOutput`

Rust signatures (wasm-bindgen):
- `analyze(source: String, context_json: String) -> Result<JsValue, JsValue>`
- `format(source: String, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `apply_edits(source: String, edits: JsValue, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `complete(source: String, cursor_utf16: usize, context_json: String) -> Result<JsValue, JsValue>`

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

Diagnostics expose quick-fix actions directly as `actions`.
Diagnostics include 1-based `line`/`col` for UI lists. These are
computed from diagnostic byte offsets through `analyzer::SourceMap::line_col`.

## Formatting and edit application

- `format(...)`:
  - fails on any lex/parse diagnostic (`Err("Format error")`)
  - computes canonical formatted text
  - validates cursor against the input source
  - builds one full-document byte `TextEdit`
  - applies through the shared byte-edit pipeline
  - returns updated source + rebased UTF-16 cursor

- `apply_edits(...)`:
  - accepts UTF-16 `TextEdit[]`
  - converts to byte edits
  - validates ranges/boundaries/overlaps
  - applies with the shared byte-edit pipeline
  - returns updated source + rebased cursor

## Validation rules (`apply_edits`)

- each edit range must be inside UTF-16 document bounds
- converted byte ranges must be valid UTF-8 char boundaries
- edits are sorted by `(start, end)`
- overlapping edits are rejected

## Error model

- `analyze` and `complete` throw on invalid context JSON.
- `format` and `apply_edits` throw on operation failure (not encoded in payload).
- error messages are minimal and deterministic (`Format error`, `Invalid edits`, `Invalid edit range`, `Overlapping edits`, `Invalid cursor`).

## Context JSON contract

- non-empty valid JSON
- unknown top-level fields rejected
- schema: `{ properties: Property[], completion?: { preferred_limit?: number } }`

## Source pointers

- exports: `analyzer_wasm/src/lib.rs`
- conversion helpers: `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`
- DTOs: `analyzer_wasm/src/dto/v1.rs`
- shared edit pipeline: `analyzer_wasm/src/text_edit.rs`

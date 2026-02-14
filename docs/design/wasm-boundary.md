# WASM boundary (`analyzer_wasm`)

This layer translates between editor coordinates (UTF-16) and core analyzer coordinates (UTF-8 bytes), and defines JS-facing DTOs.

## Exports

- `new Analyzer(config: AnalyzerConfig)`
- `Analyzer.analyze(source) -> AnalyzeResult`
- `Analyzer.ide_format(source, cursor_utf16) -> ApplyResult`
- `Analyzer.ide_apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `Analyzer.ide_help(source, cursor_utf16) -> HelpResult`

Rust signatures (wasm-bindgen):
- `Analyzer::new(config: JsValue) -> Result<Analyzer, String>`
- `Analyzer::analyze(&self, source: String) -> Result<JsValue, JsValue>`
- `Analyzer::ide_format(&self, source: String, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `Analyzer::ide_apply_edits(&self, source: String, edits: JsValue, cursor_utf16: u32) -> Result<JsValue, JsValue>`
- `Analyzer::ide_help(&self, source: String, cursor_utf16: u32) -> Result<JsValue, JsValue>`

## Hard rules

- Core (`analyzer`) uses UTF-8 byte offsets only.
- WASM boundary is the only UTF-16 ↔ byte conversion layer.
- JS/WASM DTO spans and edits are UTF-16 code units.
- Half-open ranges everywhere: `[start, end)`.

## DTOs (v1)

- `AnalyzerConfig { properties, preferred_limit }`
- `Property { name, type }`
- `Ty = Number | String | Boolean | Date | List<Ty>`
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

Offset conversion helpers are centralized in `analyzer_wasm/src/offsets.rs`:
- `utf16_to_8_offset`
- `utf8_to_16_offset`
- `utf16_to_8_cursor`
- `utf16_to_8_text_edits`

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

- `Analyzer::new` returns `Err("Invalid analyzer config")` for invalid config shape.
- `analyze` and `ide_help` throw only on serialization failures.
- `ide_format` and `ide_apply_edits` throw on operation failure (not encoded in payload).
- error messages are minimal and deterministic (`Format error`, `Invalid edits`, `Invalid edit range`, `Overlapping edits`, `Invalid cursor`).

## Analyzer config contract

- object input only
- unknown top-level fields rejected
- schema: `{ properties?: Property[], preferred_limit?: number | null }`
- `preferred_limit = null` means default `5`

## Source pointers

- exports: `analyzer_wasm/src/lib.rs`
- conversion helpers: `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`
- DTOs: `analyzer_wasm/src/dto/v1.rs`
- core edit pipeline: `analyzer/src/ide/edit.rs`, `analyzer/src/text_edit.rs`

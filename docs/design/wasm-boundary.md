# WASM boundary (`analyzer_wasm`)

This layer translates between editor coordinates (UTF-16) and core analyzer coordinates (UTF-8
bytes), and defines the JS-facing DTOs.

## Exports

Defined in `analyzer_wasm/src/lib.rs`:

- `analyze(source, context_json) -> AnalyzeResult`
- `complete(source, cursor_utf16, context_json) -> CompletionOutputView`
- `pos_to_line_col(source, pos_utf16) -> LineColView`

Rust signatures (wasm-bindgen surface):

- `analyze(source: String, context_json: String) -> Result<JsValue, JsValue>`
- `complete(source: String, cursor_utf16: usize, context_json: String) -> Result<JsValue, JsValue>`
- `pos_to_line_col(source: String, pos_utf16: u32) -> JsValue`

## Encoding boundary (hard rule)

- Core analyzer (`analyzer/`): UTF-8 byte offsets.
- JS/WASM boundary: UTF-16 code unit offsets.
- Half-open spans everywhere: `[start, end)`.
- Conversion lives only in WASM:
  - `analyzer_wasm/src/offsets.rs`
  - `analyzer_wasm/src/span.rs`
  - `analyzer_wasm/src/converter.rs`

Helper APIs (WASM):

- `utf16_offset_to_byte(source, utf16_pos)`
- `byte_offset_to_utf16_offset(source, byte_pos)`
- `byte_span_to_utf16_span(source, span)`

## DTOs (v1)

- Definitions: `analyzer_wasm/src/dto/v1.rs`
- Spans/offsets are UTF-16 code units, half-open `[start, end)`.

Key types:

- `AnalyzeResult { diagnostics, tokens, formatted, quick_fixes, output_type }`
  - `formatted` is empty whenever lex/parse diagnostics are present.
  - `quick_fixes` contains structured UTF-16 edits converted from core
    `analyzer::quick_fixes(&diagnostics)` (insert/replace delimiters, comma insertion/removal).
  - `output_type` is non-null (`string`): unknown/error uses `"unknown"`.
- `Span { start, end }`
- `SpanView { range: Span }`
- `LineColView { line, col }` (1-based; `col` is a Rust `char` count, not UTF-16)
- `TextEditView { range: Span, new_text }`
- `CompletionItemView { label, kind, insert_text, primary_edit, cursor, additional_edits, detail, is_disabled, disabled_reason }`
- `CompletionItemKind` includes function-specific kinds:
  - `FunctionGeneral`, `FunctionText`, `FunctionNumber`, `FunctionDate`, `FunctionPeople`,
    `FunctionList`, `FunctionSpecial`
  - plus `Builtin`, `Property`, `Operator`
- `CompletionOutputView { items, replace, signature_help, preferred_indices }`

## context_json contract

Parsed by `Converter::parse_context` (`analyzer_wasm/src/converter.rs`):

- Must be non-empty and valid JSON.
- Unknown top-level fields are rejected (`deny_unknown_fields`).
- Current schema:
  - `{ properties: Property[], completion?: { preferred_limit?: number } }`
- `functions` come from Rust builtins (`builtins_functions()`); JS cannot supply them.

Tests: `analyzer_wasm/tests/analyze.rs`

## Error model

- Invalid/empty/unknown-field `context_json` throws `JsValue("Invalid context JSON")`.
- Parse/semantic issues do not throw; they return diagnostics in the payload.
  - Code: `analyzer_wasm/src/lib.rs`, `analyzer_wasm/src/converter.rs`

## Line/column handling

- DTO `SpanView` does not store line/column.
- Line/column is derived on demand via:
  - `pos_to_line_col(source, pos_utf16) -> LineColView` (1-based)
- `pos_to_line_col` takes a UTF-16 offset at the boundary and maps through byte offsets internally.

Code:
- core mapping: `analyzer/src/source_map.rs`
- boundary conversion: `analyzer_wasm/src/converter.rs`

## Text edits / cursor rebasing

- Core `TextEdit` ranges are byte offsets.
- DTO `TextEditView` ranges are UTF-16.
- `analyzer_wasm/src/text_edit.rs`:
  - applies byte edits in descending start-offset order
  - can rebase a byte cursor through edits
- Completion DTO `cursor` (when present) is a UTF-16 offset in the updated document.

JS never deals with byte offsets.
Rust core never deals with UTF-16 offsets.

Quick-fix derivation lives in core (`analyzer/src/ide/quick_fix.rs`); WASM only converts byte
ranges to UTF-16 DTO ranges.

# analyzer_wasm

WASM/JS boundary for `analyzer/` using `wasm-bindgen`.

This crate:
- Parses `context_json` (properties + completion config).
- Bridges UTF-16 offsets/spans (editors) ↔ UTF-8 byte offsets (Rust core).
- Converts Rust analyzer outputs into DTOs (`dto::v1`) and serializes them to JS.

## Coordinates (hard rule)

- Rust core (`analyzer/`) uses **UTF-8 byte offsets** (`analyzer::Span`).
- JS/WASM boundary uses **UTF-16 code unit offsets** (`dto::v1::Span`).
- Half-open spans everywhere: `[start, end)`.
- Conversion is clamped/floored to valid boundaries and lives only here:
  - `analyzer_wasm/src/offsets.rs`
  - `analyzer_wasm/src/span.rs`
  - tests: `analyzer_wasm/tests/analyze.rs`

## WASM exports

Defined in `analyzer_wasm/src/lib.rs`:

- `analyze(source, context_json) -> AnalyzeResult`
- `complete(source, cursor_utf16, context_json) -> CompletionOutputView`
- `pos_to_line_col(source, pos_utf16) -> LineColView`

DTO definitions: `analyzer_wasm/src/dto/v1.rs`

Notes:
- `pos_to_line_col` returns 1-based `(line, col)` where `col` is a Rust `char` count (Unicode scalar values), not UTF-16.
- `complete` takes a UTF-16 cursor offset and returns edits/spans in UTF-16.

### Payload shape (DTO v1)

- `AnalyzeResult`:
  - `diagnostics`: `DiagnosticView[]`
  - `tokens`: `TokenView[]` (non-trivia tokens only; trivia is filtered out in the converter)
  - `formatted`: `string`
  - `output_type`: `string` (semantic root type rendered by Rust, e.g. `"number | string"`)
    - never nullable; unknown/error uses `"unknown"`
- `CompletionOutputView`:
  - `items`: `CompletionItemView[]`
    - `CompletionItemView.kind` is function-specific for builtins:
      `FunctionGeneral | FunctionText | FunctionNumber | FunctionDate | FunctionPeople | FunctionList | FunctionSpecial`
    - `CompletionItemView` no longer carries a separate `category` field.
  - `replace`: `Span` (original doc, UTF-16)
  - `signature_help`: optional structured segments
  - `preferred_indices`: `number[]`

Conversion happens in `analyzer_wasm/src/converter.rs`.

### Error model

- Invalid/empty/unknown-field `context_json` throws `JsValue("Invalid context JSON")`.
- Parse/semantic issues do not throw; they are returned as diagnostics in the payload.
  - See `analyzer_wasm/src/lib.rs`.

## `context_json` contract

Parsed by `Converter::parse_context` (`analyzer_wasm/src/converter.rs`):

- Must be a non-empty JSON string.
- Unknown top-level fields are rejected (`deny_unknown_fields`).
- Current schema:
  - `{ properties: Property[], completion?: { preferred_limit?: number } }`
- `functions` are not provided by JS; they come from Rust builtins (`builtins_functions()`).

Coverage: `analyzer_wasm/tests/analyze.rs`

## TS DTO export

`cargo run -p analyzer_wasm --bin export_ts` writes:

- `examples/vite/src/analyzer/generated/wasm_dto.ts`

Implementation: `analyzer_wasm/src/bin/export_ts.rs`

## Testing

```bash
cargo test -p analyzer_wasm
```

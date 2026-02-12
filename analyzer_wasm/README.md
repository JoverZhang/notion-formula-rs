# analyzer_wasm

WASM/JS boundary for `analyzer`.

## Responsibility

This crate owns all UTF-16 ↔ UTF-8 byte conversion. Core analyzer stays byte-only.

## Exports

Defined in `analyzer_wasm/src/lib.rs`:

- `analyze(source, context_json) -> AnalyzeResult`
- `format(source, cursor_utf16) -> ApplyResult`
- `apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `complete(source, cursor_utf16, context_json) -> CompletionOutput`

## DTOs (`dto::v1`)

- `AnalyzeResult { diagnostics, tokens, output_type }`
- `Diagnostic { kind, message, span, line, col, actions }`
- `CodeAction { title, edits }`
- `TextEdit { range, new_text }`
- `ApplyResult { source, cursor }`
- `CompletionOutput { items, replace, signature_help, preferred_indices }`

All spans/offsets in DTOs are UTF-16 code units and half-open `[start, end)`.
`Diagnostic.line`/`col` are 1-based values derived from core byte spans via
`analyzer::SourceMap::line_col` (`col` is Unicode scalar count).

## Error model

- `analyze`: throws only for invalid context JSON / serialization errors.
- `format`: throws on syntax-invalid input (`Format error`).
- `apply_edits`: throws on invalid edits / invalid cursor / overlaps.
- `complete`: throws on invalid context JSON / serialization errors.

## Edit application rules

`apply_edits` validation is strict:
- UTF-16 ranges must be within the document
- converted byte ranges must be UTF-8 char boundaries
- edits must be sorted by `(start, end)` and non-overlapping

- `apply_edits` rebases cursor through the shared byte-edit pipeline in
  `analyzer_wasm/src/text_edit.rs`.
- `format` validates the incoming cursor against the original source, builds one
  full-document byte `TextEdit`, and applies it through the same shared byte-edit
  pipeline used by `apply_edits`.

## `context_json` contract

- non-empty JSON string
- unknown top-level fields rejected
- current schema:
  - `{ properties: Property[], completion?: { preferred_limit?: number } }`

## Testing

```bash
cargo test -p analyzer_wasm
wasm-pack test --node analyzer_wasm
```

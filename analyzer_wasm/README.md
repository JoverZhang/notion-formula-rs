# analyzer_wasm

WASM/JS boundary for `analyzer`.

## Responsibility

This crate owns all UTF-16 ↔ UTF-8 byte conversion. Core analyzer stays byte-only.

## Exports

Defined in `analyzer_wasm/src/lib.rs`:

- `analyze(source, context_json) -> AnalyzeResult`
- `ide_format(source, cursor_utf16) -> ApplyResult`
- `ide_apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `ide_help(source, cursor_utf16, context_json) -> HelpResult`

## DTOs (`dto::v1`)

- `AnalyzeResult { diagnostics, tokens, output_type }`
- `Diagnostic { kind, message, span, line, col, actions }`
- `CodeAction { title, edits }`
- `TextEdit { range, new_text }`
- `ApplyResult { source, cursor }`
- `CompletionResult { items, replace, preferred_indices }`
- `HelpResult { completion, signature_help }`

All spans/offsets in DTOs are UTF-16 code units and half-open `[start, end)`.
`Diagnostic.line`/`col` are 1-based values derived from core byte spans via
`analyzer::SourceMap::line_col` (`col` is Unicode scalar count).

## Error model

- `analyze`: throws only for invalid context JSON / serialization errors.
- `ide_format`: throws on syntax-invalid input (`Format error`).
- `ide_apply_edits`: throws on invalid edits / invalid cursor / overlaps.
- `ide_help`: throws on invalid context JSON / serialization errors.

## Edit application rules

`ide_apply_edits` validates UTF-16 ranges strictly before forwarding to core:
- UTF-16 ranges must be within the document
- converted byte ranges must be UTF-8 char boundaries

Core edit application (sorting, overlap checks, cursor rebasing, full-document format edit) now
lives in `analyzer/src/ide/edit.rs`. WASM only converts UTF-16 ↔ UTF-8 and serializes DTOs.

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

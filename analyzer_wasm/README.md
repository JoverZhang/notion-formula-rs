# analyzer_wasm

WASM/JS boundary for `analyzer`.

## Responsibility

This crate owns all UTF-16 ↔ UTF-8 byte conversion. Core analyzer stays byte-only.
`analyze` forwards to `analyzer`; IDE operations (`format` / `apply_edits` / `help`) forward to
the `ide` crate.

## Exports

Defined in `analyzer_wasm/src/lib.rs`:

- `new Analyzer(config: AnalyzerConfig)`
- `Analyzer.analyze(source) -> AnalyzeResult`
- `Analyzer.format(source, cursor_utf16) -> ApplyResult`
- `Analyzer.apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `Analyzer.help(source, cursor_utf16) -> HelpResult`

## DTOs (`dto::v1`)

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

All spans/offsets in DTOs are UTF-16 code units and half-open `[start, end)`.
`Diagnostic.line`/`col` are 1-based values derived from core byte spans via
`analyzer::SourceMap::line_col` (`col` is Unicode scalar count).

Offset conversion helpers are centralized in `analyzer_wasm/src/offsets.rs`:
- `utf16_to_8_offset`
- `utf8_to_16_offset`
- `utf16_to_8_cursor`
- `utf16_to_8_text_edits`

## Error model

- `Analyzer::new`: returns `Err("Invalid analyzer config")` for invalid config shape.
- `analyze`: throws only for serialization errors.
- `format`: throws on syntax-invalid input (`Format error`).
- `apply_edits`: throws on invalid edits / invalid cursor / overlaps.
- `help`: throws only for serialization errors.

## Edit application rules

`apply_edits` validates UTF-16 ranges strictly before forwarding to core:
- UTF-16 ranges must be within the document
- converted byte ranges must be UTF-8 char boundaries

Core edit application (sorting, overlap checks, cursor rebasing, full-document format edit) now
lives in `ide/src/edit.rs`. WASM only converts UTF-16 ↔ UTF-8 and serializes DTOs.

## `AnalyzerConfig` contract

- object shape only (constructor argument)
- unknown top-level fields rejected
- schema:
  - `{ properties?: Property[], preferred_limit?: number | null }`
- `preferred_limit = null` uses default `5`

## Testing

```bash
cargo test -p analyzer_wasm
wasm-pack test --node analyzer_wasm
```

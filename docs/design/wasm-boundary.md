# analyzer_wasm (Design)

Design rationale for the `analyzer_wasm` crate.
For implementation details, see `analyzer_wasm/README.md`.

## Purpose

JS/TS-facing WASM facade. Translates between editor coordinates (UTF-16) and core coordinates (UTF-8 bytes), defines DTOs, and forwards calls to `analyzer` and `ide`.

## Pipeline

```
  JS caller (UTF-16 offsets)
       |
       v
  Analyzer struct ──> new / analyze / format /
       |               apply_edits / help
       |               (analyzer_wasm/src/lib.rs)
       v
  offsets.rs ──> utf16_to_8 / utf8_to_16
       |          (analyzer_wasm/src/offsets.rs)
       v
  Core analyzer + ide (UTF-8 byte offsets)
       |
       v
  converter/*.rs ──> Core types -> DTO types
       |              (analyzer_wasm/src/converter/)
       v
  dto/v1.rs ──> Serialized to JsValue
                (analyzer_wasm/src/dto/v1.rs)
```

## Key types (DTOs)

| DTO | Fields |
| --- | --- |
| `AnalyzerConfig` | `properties, preferred_limit` |
| `Property` | `name, type` |
| `Ty` | `Number, String, Boolean, Date, List<Ty>` |
| `AnalyzeResult` | `diagnostics, tokens, output_type` |
| `Diagnostic` | `kind, message, span, line, col, actions` |
| `CodeAction` | `title, edits` |
| `TextEdit` | `range, new_text` |
| `ApplyResult` | `source, cursor` |
| `CompletionResult` | `items, replace, preferred_indices` |
| `HelpResult` | `completion, signature_help` |

All DTO spans are UTF-16 code units, half-open `[start, end)`.
`Diagnostic.line`/`col` are 1-based, derived from byte spans via `analyzer::SourceMap::line_col`.

## Exports

JS-facing API:

- `new Analyzer(config: AnalyzerConfig)`
- `Analyzer.analyze(source) -> AnalyzeResult`
- `Analyzer.format(source, cursor_utf16) -> ApplyResult`
- `Analyzer.apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `Analyzer.help(source, cursor_utf16) -> HelpResult`

## Contracts

- Core (`analyzer`) uses UTF-8 byte offsets only.
- WASM boundary is the only UTF-16 <-> byte conversion layer.
- Half-open ranges everywhere: `[start, end)`.
- Only expose APIs and coordinate conversions actually needed; avoid extra logic.
- See `docs/design/contracts.md` for full cross-crate contract listing.

### Format and edit application

- `format(...)`: validates UTF-16 cursor, converts to byte, forwards to `ide::format(...)`, maps back.
- `apply_edits(...)`: accepts UTF-16 `TextEdit[]`, converts to byte edits, validates UTF-16 bounds + UTF-8 char boundaries, forwards to `ide::apply_edits(...)`.
- Core edit behavior (sorting, overlap checks, cursor rebasing) lives in `ide/src/edit.rs`.

### Validation rules (apply_edits)

- Each edit range must be inside UTF-16 document bounds.
- Converted byte ranges must be valid UTF-8 char boundaries.

### Error model

- `Analyzer::new`: `Err("Invalid analyzer config")` for invalid config shape.
- `analyze` / `help`: throw only on serialization failures.
- `format` / `apply_edits`: throw on operation failure (not encoded in payload).
- Error messages are minimal and deterministic: `Format error`, `Invalid edits`, `Invalid edit range`, `Overlapping edits`, `Invalid cursor`.

### AnalyzerConfig contract

- Object input only.
- Unknown top-level fields rejected.
- Schema: `{ properties?: Property[], preferred_limit?: number | null }`.
- `preferred_limit = null` means default `5`.

## Why: thin boundary layer

- WASM only converts coordinates and forwards calls.
- No semantic logic in this crate; all analysis lives in `analyzer` and `ide`.
- DTO v1 is the stable serialization boundary; internal types can evolve independently.

## Source pointers

- Exports: `analyzer_wasm/src/lib.rs`
- Conversion helpers: `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`
- DTOs: `analyzer_wasm/src/dto/v1.rs`
- Converters: `analyzer_wasm/src/converter/` (analyze, completion, shared)
- Core edit pipeline: `ide/src/edit.rs`, `ide/src/text_edit.rs`

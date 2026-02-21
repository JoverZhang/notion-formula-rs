# Contracts (Hard Rules)

This file records stable contracts across crates.
If a contract changes, it needs tests + docs + changelog.

## Spans and offsets

Rule: Core spans/offsets use UTF-8 bytes, with half-open ranges `[start, end)`.
Rule: DTO spans/edits use UTF-16 code units, with half-open ranges `[start, end)`.
Rule: Coordinate conversion only happens at the WASM boundary layer.
Rule: `Diagnostic.line`/`col` are computed from byte offsets during WASM conversion.
Where: `analyzer/src/lexer/token.rs`, `analyzer_wasm/src/dto/v1.rs`, `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`, `analyzer_wasm/src/converter/shared.rs`.

## Token stream

Rule: The token stream includes trivia (`DocComment`, `Newline`) and explicit `Eof`.
Rule: `TokenQuery` is the trivia-aware source-of-truth API.

## AST + syntax invariants

Rule: `ExprKind` is the closed set of expression forms.
Rule: The parser recovers with `ExprKind::Error` and keeps parsing.
Rule: Member access cannot be used bare; it must be `receiver.method(...)`.

## Diagnostics determinism

Rule: Diagnostics with the same span are deduped by priority.
Rule: `format_diagnostics` output order is stable (span, priority, message).

## Actions and edits

Rule: Quick fixes are exposed as `Diagnostic.actions: Vec<CodeAction>`.
Rule: The core edit model is `TextEdit { range, new_text }` in byte coordinates.
Rule: The WASM edit model uses UTF-16 coordinates.
Rule: `ide::format` and `ide::apply_edits` take a byte cursor and return `{ source, cursor }`.
Rule: WASM `format/apply_edits` take a UTF-16 cursor and return a UTF-16 cursor.
Rule: `ide::format` goes through the same byte-edit pipeline via one full-document `TextEdit`.
Rule: WASM only converts coordinates and forwards calls; failures are returned as `Err`.

## Signature help

Rule: Parameter shape follows `ParamShape { head, repeat, tail }`.
Rule: Output is structured `DisplaySegment[]`, rendered directly by the UI.
Rule: Active-parameter behavior is defined in `docs/signature-help.md`.

## WASM `AnalyzerConfig`

Rule: Constructor argument must be an object.
Rule: Unknown top-level fields are rejected.
Rule: Current schema is `{ properties?: Property[], preferred_limit?: number | null }`.
Rule: `preferred_limit = null` uses default `5`.
Rule: `functions` come from Rust built-ins; JS does not provide them.

# Design (notion-formula-rs)

Stable architecture and contracts for the workspace.

## Pipeline

```text
&str (UTF-8)
  -> lexer (tokens + diagnostics)
  -> parser (AST + diagnostics)
  -> semantic (optional)
  -> IDE helpers (format / completion)
  -> WASM boundary (DTO v1 in UTF-16)
```

## Source tree

- `analyzer/`: Rust core
- `analyzer_wasm/`: WASM boundary
- `examples/vite/`: demo UI
- `docs/`: contracts and design notes

## Public entry points

### Rust

- `analyze(text) -> ParseOutput`
- `semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)`
- `format_expr(expr, source, tokens) -> String`
- `completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput`
- `format_diagnostics(source, diags) -> String`

### WASM

- `analyze(source, context_json) -> AnalyzeResult`
- `format(source, cursor_utf16) -> ApplyResultView`
- `apply_edits(source, edits, cursor_utf16) -> ApplyResultView`
- `complete(source, cursor_utf16, context_json) -> CompletionOutputView`

## Key contracts (hard rules)

- Core analyzer uses UTF-8 byte offsets only.
- WASM boundary is the only UTF-16 ↔ byte conversion layer.
- JS/WASM spans/edits are UTF-16, half-open `[start, end)`.
- Unified edit model: `TextEdit { range, new_text }`.
- Diagnostic quick-fix actions are diagnostic-level payload (`actions`).
- `format` and `apply_edits` always accept cursor and always return cursor.
- `format`/`apply_edits` failures throw (`Err`) instead of payload-encoded result enums.

## Data model highlights

- `ParseOutput { expr, diagnostics, tokens }`
- `Diagnostic { kind, code, message, span, labels, notes, actions }`
- `CodeAction { title, edits: Vec<TextEdit> }`
- `AnalyzeResult { diagnostics, tokens, output_type }`
- `DiagnosticView { kind, message, span, line, col, actions }`
- `ApplyResultView { source, cursor }`

## Design philosophy

- Keep core byte-based and deterministic.
- Keep boundary conversion centralized.
- Keep DTO surface small and explicit.
- Keep `apply_edits` on one canonical edit-application pipeline; keep `format`
  cursor mapping explicit and stable for editor UX.

## Deep dives

- `docs/design/wasm-boundary.md`
- `docs/design/completion.md`
- `docs/design/tokens-spans.md`
- `docs/design/demo-vite.md`
- `docs/design/testing.md`

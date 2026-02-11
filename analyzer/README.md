# analyzer

Core analyzer for a Notion-like formula language.

## Coordinates (hard rule)

- `Span { start, end }` is UTF-8 byte offsets into source.
- Half-open everywhere: `[start, end)`.
- UTF-16 conversion does not happen in this crate.

## Entry points

- `analyzer::analyze(text) -> Result<ParseOutput, Diagnostic>`
- `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)`
- `analyzer::format_expr(expr, source, tokens) -> String`
- `analyzer::completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput`
- `analyzer::format_diagnostics(source, diags) -> String`

## Key output types

- `ParseOutput { expr, diagnostics, tokens }`
- `Diagnostic { kind, code, message, span, labels, notes, actions }`
- `CodeAction { title, edits: Vec<TextEdit> }`
- `TextEdit { range, new_text }`

Quick fixes are represented as diagnostic actions, not as a separate parse-output list.

## Module map

| Path | Owns |
|---|---|
| `analyzer/src/lexer/` | Tokens + trivia + EOF + lex diagnostics |
| `analyzer/src/parser/` | Pratt parser, AST, recovery |
| `analyzer/src/diagnostics.rs` | Diagnostic model + deterministic formatting |
| `analyzer/src/analysis/` | Type inference + semantic diagnostics |
| `analyzer/src/ide/format.rs` | Formatter |
| `analyzer/src/ide/completion/` | Completion + signature help |
| `analyzer/src/text_edit.rs` | Unified edit model |

## Invariants

- Parser and semantic diagnostics are deterministic and stable.
- Completion edits and diagnostic actions use byte ranges.
- No UTF-16 offsets are stored in core data structures.

## Testing

```bash
cargo test -p analyzer
BLESS=1 cargo test -p analyzer
```

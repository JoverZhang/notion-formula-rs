# analyzer

Core analyzer for a Notion-like formula language.

IDE/editor helpers (formatter/completion/signature help/edit application) live in the sibling
`ide` crate.

## Coordinates (hard rule)

- `Span { start, end }` is UTF-8 byte offsets into source.
- Half-open everywhere: `[start, end)`.
- UTF-16 conversion does not happen in this crate.

## Entry points

- `analyzer::analyze_syntax(text) -> SyntaxResult` (`lex + parse`)
- `analyzer::analyze(text, ctx) -> AnalyzeResult` (`lex + parse + sema`)
- `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)`
- `analyzer::infer_expr_with_map(expr, ctx, map) -> Ty`
- `analyzer::format_diagnostics(source, diags) -> String`

## Key output types

- `ParseOutput { expr, diagnostics, tokens }`
- `AnalyzeResult { diagnostics, tokens, output_type }`
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
| `analyzer/src/text_edit.rs` | Core `TextEdit` model (byte ranges) |

## Invariants

- Parser and semantic diagnostics are deterministic and stable.
- Diagnostic actions use byte ranges.
- No UTF-16 offsets are stored in core data structures.

## Testing

```bash
cargo test -p analyzer
BLESS=1 cargo test -p analyzer
```

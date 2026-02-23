# ide

IDE/editor helpers for notion-formula-rs.

This crate owns formatter, completion, signature help, and byte-edit application.
It depends on `analyzer` for core syntax/semantic structures and analysis entry points.

## Coordinates (hard rule)

- `Span { start, end }` is UTF-8 byte offsets into source.
- Half-open everywhere: `[start, end)`.
- UTF-16 conversion stays in `analyzer_wasm`.

## Entry points

- `ide::help(source, cursor_byte, ctx, config) -> HelpResult`
- `ide::format(source, cursor_byte) -> Result<ApplyResult, IdeError>`
- `ide::apply_edits(source, edits, cursor_byte) -> Result<ApplyResult, IdeError>`

## Help architecture

- `ide::help` is the orchestration entry.
- `src/context.rs` detects call context, position kind, replace span, and query.
- `src/signature.rs` computes signature help from call context.
- `src/completion/items.rs` builds raw completion candidates by position kind.
- `src/completion/ranking.rs` applies edits, query ranking, and preferred indices.

## Dependencies on analyzer

- Methods: `analyzer::analyze_syntax`, `analyzer::analyze`, `analyzer::infer_expr_with_map`
- Structures: `ast`, `Span`, `Token`, `TextEdit`, diagnostics/semantic model types

## Testing

```bash
cargo test -p ide
```

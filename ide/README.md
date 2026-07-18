# ide

IDE/editor helpers for notion-formula-rs.

Design rationale: [`docs/design/ide.md`](../docs/design/ide.md).
Cross-crate contracts: [`docs/design/contracts.md`](../docs/design/contracts.md).

This crate owns formatter, completion, signature help, and byte-edit application.
It depends on `analyzer` for core syntax/semantic structures and analysis entry points.

## Coordinates

- `Span { start, end }` is UTF-8 byte offsets into source.
- Half-open everywhere: `[start, end)`.
- UTF-16 conversion stays in `analyzer_wasm`.
- See `docs/design/contracts.md` for full span/offset rules.

## Entry points

- `ide::help(source, cursor_byte, ctx, config) -> HelpResult`
- `ide::format(source, cursor_byte) -> Result<ApplyResult, IdeError>`
- `ide::apply_edits(source, edits, cursor_byte) -> Result<ApplyResult, IdeError>`

## Help architecture

- `ide::help` is the orchestration entry.
- `src/context.rs` detects call context, position kind, replace span, and query.
- `src/signature/` adapts shared resolved-call projection into Signature Help.
- `src/completion/items.rs` builds raw completion candidates by position kind.
- `src/completion/ranking.rs` applies edits, query ranking, and preferred indices.

## Signature Help boundary

IDE supplies incomplete argument observations to `builtin_fn::resolve_call_signature` via
Analyzer's semantic model. `ResolvedFunctionSig::projection` drives displayed slots,
repeat-group numbering, expected types, and the active parameter. IDE code owns only editor
presentation concerns such as cursor position, postfix receiver insertion, labels, and
display segments; it does not maintain independent repeat-shape or generic-binding logic.

## Dependencies on analyzer

- Methods: `analyzer::analyze_syntax`, `analyzer::analyze`, and semantic inference entry
  points used to observe incomplete arguments
- Structures: `ast`, `Span`, `Token`, `TextEdit`, diagnostics/semantic model types

## Testing

```bash
cargo test -p ide
```

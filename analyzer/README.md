# analyzer

Core analyzer for a Notion-like formula language.

Design rationale: [`docs/design/analyzer.md`](../docs/design/analyzer.md).
Cross-crate contracts: [`docs/design/contracts.md`](../docs/design/contracts.md).

IDE/editor helpers (formatter/completion/signature help/edit application) live in the sibling
`ide` crate.

Builtin function signatures and the shared builtin type model live in the sibling
`builtin_fn` crate and are re-exported through `analyzer::semantic`.

## Coordinates

- `Span { start, end }` is UTF-8 byte offsets into source.
- Half-open everywhere: `[start, end)`.
- UTF-16 conversion does not happen in this crate.
- See `docs/design/contracts.md` for full span/offset rules.

## Entry points

- `analyzer::analyze_syntax(text) -> SyntaxResult` (`lex + parse`)
- `analyzer::analyze(text, ctx) -> AnalyzeResult` (`lex + parse + sema`)
- `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)`
- `analyzer::semantic::analyze_expr_with_semantic_map(expr, ctx) -> (Ty, SemanticMap, Vec<Diagnostic>)`
- `analyzer::infer_expr_with_map(expr, ctx, map) -> Ty`
- `analyzer::infer_expr_with_semantic_map(expr, ctx, map) -> Ty`
- `analyzer::format_diagnostics(source, diags) -> String`

## Key output types

- `ParseOutput { expr, diagnostics, tokens }`
- `AnalyzeResult { diagnostics, tokens, output_type }`
- `Diagnostic { kind, code, message, span, labels, notes, actions }`
- `CodeAction { title, edits: Vec<TextEdit> }`
- `TextEdit { range, new_text }`
- `SemanticMap { expression_types, builtin_calls }`

Quick fixes are represented as diagnostic actions, not as a separate parse-output list.

`SemanticMap::builtin_calls` retains the final `ResolvedFunctionSig` for each executable
builtin call. Lambda calls keep only the final resolution after body inference. The shared
`builtin_fn::resolve_call_signature` engine owns shape projection, generic binding,
argument compatibility, staged lambda inference, and return resolvers; Analyzer diagnostics
consume that result instead of maintaining another resolver. The evaluator Planner consumes
the same final records and never rebinds calls from runtime values.

## Module map

| Path | Owns |
|---|---|
| `analyzer/src/span.rs` | Core `Span`/`Spanned` byte-range types |
| `analyzer/src/lexer/` | Tokens + trivia + EOF + lex diagnostics |
| `analyzer/src/parser/` | Pratt parser, AST, recovery |
| `analyzer/src/diagnostics.rs` | Diagnostic model + deterministic formatting |
| `analyzer/src/analysis/` | Type inference, final resolved-call handoff, semantic diagnostics, and `builtin_fn` re-exports |
| `analyzer/src/text_edit.rs` | Core `TextEdit` model (byte ranges) |

## Invariants

See `docs/design/contracts.md` for full contract listing.

## Testing

```bash
cargo test -p analyzer
BLESS=1 cargo test -p analyzer
```

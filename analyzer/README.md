# analyzer

Frontend tooling for a Notion-like formula language: lexing, parsing, diagnostics, formatting, semantic checks, completion, and signature help.

## Coordinates (hard rule)

- `Span { start, end }` is **UTF-8 byte offsets** into the original source.
- Half-open: `[start, end)`.
- WASM/UTF-16 conversion does not happen here (see `analyzer_wasm/`).
  - Source: `analyzer/src/lexer/token.rs`

## Entry points

- Parse frontend:
  - `analyzer::analyze(text: &str) -> Result<ParseOutput, Diagnostic>` (`analyzer/src/lib.rs`)
  - `ParseOutput { expr, diagnostics, tokens }` (`analyzer/src/parser/mod.rs`)
- Semantic pass (optional):
  - `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)` (`analyzer/src/analysis/mod.rs`)
- Formatting:
  - `analyzer::format_expr(expr, source, tokens) -> String` (`analyzer/src/ide/format.rs`)
- Editor features:
  - `analyzer::completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput` (`analyzer/src/ide/completion/mod.rs`)
- Diagnostics rendering:
  - `analyzer::format_diagnostics(source, diags) -> String` (`analyzer/src/diagnostics.rs`)

## Completion + signature help model (Rust)

- Coordinates: byte offsets (`Span`), half-open `[start, end)`.
- `CompletionOutput`:
  - `items`: `CompletionItem[]`
  - `replace`: byte span to replace in the original text
  - `signature_help`: optional structured signature info (no frontend parsing)
  - `preferred_indices`: UI “smart picks” (bounded by `CompletionConfig.preferred_limit`)
  - Source: `analyzer/src/ide/completion/mod.rs`
- Ranking/matching lives in:
  - `analyzer/src/ide/completion/rank.rs`
  - tests: `analyzer/src/tests/ide/test_completion_ranking.rs`
- Signature help uses `ParamShape` rules (spec): `../docs/signature-help.md`

## Module map

| Path | Owns |
|---|---|
| `analyzer/src/lexer/` | Tokens + trivia + spans; emits explicit `Eof` |
| `analyzer/src/parser/` | Pratt parser; AST; recovery; `TokenCursor`/`TokenQuery` |
| `analyzer/src/diagnostics.rs` | Diagnostic model + deterministic formatting |
| `analyzer/src/analysis/` | `Ty` model; builtins; `ParamShape`; type inference + validation |
| `analyzer/src/ide/display.rs` | Canonical UI display formatting (signature help segments, `Ty` rendering) |
| `analyzer/src/ide/format.rs` | Formatter (comment/trivia-aware) |
| `analyzer/src/ide/completion/` | Completion + signature help (byte offsets) |
| `analyzer/src/source_map.rs` | byte offset → 1-based `(line, col)` (col is Rust `char` count) |

## Syntax (current implementation)

Language spec lives in [`docs/design/README.md` → “Language contract (spec)”](../docs/design/README.md#language-contract-spec).
This section documents current lexer/parser behavior.

### Literals and identifiers

- numbers: ASCII digits only (no decimals) (`analyzer/src/lexer/mod.rs`)
- strings: double-quoted, no escapes (`analyzer/src/lexer/mod.rs`)
- identifiers: ASCII letters/`_` plus any non-ASCII codepoint (`analyzer/src/lexer/mod.rs`)

### Expressions

Expressions (`ExprKind`, `analyzer/src/parser/ast.rs`):
- unary: `!`, `not`, `-`
- binary: `< <= == != >= > && || + - * / % ^`
  - `^` is right-associative (`infix_binding_power` in `analyzer/src/parser/ast.rs`)
- ternary: `cond ? then : otherwise`
- grouping: `(expr)` is preserved as `ExprKind::Group`
- list literals: `[a, b, c]` (`ExprKind::List`; trailing comma is rejected)
- calls: `ident(...)`
- member-call: `receiver.method(...)` (member access without `(...)` is rejected)

### Parser recovery

- Best-effort: emits diagnostics and produces an AST with `ExprKind::Error` placeholders.
- Diagnostics are deconflicted by span using priority (see `analyzer/src/diagnostics.rs`).
- Delimited comma-separated sequences share one recovery routine (`analyzer/src/parser/expr.rs`):
  - missing commas between items emit a diagnostic and parsing continues as if a comma was inserted
  - missing items around commas produce `ExprKind::Error` entries
  - missing/mismatched closing delimiters emit labels + fix-its (`insert ')'` / `insert ']'`)
- Ternary recovery avoids consuming closing delimiters (so surrounding `)`/`]` can still parse).
- Member-call recovery skips redundant dots (`a..if(b,c)`), emits a diagnostic, and still parses.

## Key contracts / invariants

Tokens:
- Token stream includes trivia (`DocComment`, `Newline`) and an explicit `Eof` token:
  - `analyzer/src/lexer/token.rs`
- `TokenQuery` is the canonical trivia-aware scan API used by formatter/utilities:
  - `analyzer/src/parser/tokenstream.rs`

AST:
- `ExprKind` is the closed set of expression forms:
  - `analyzer/src/parser/ast.rs`

Diagnostics:
- Deconflicted by span using `DiagnosticCode::priority()`; output ordering is deterministic:
  - `analyzer/src/diagnostics.rs`

Signatures:
- Builtins use `FunctionSig` + `ParamShape { head, repeat, tail }` with determinism invariants:
  - `analyzer/src/analysis/signature.rs`
  - Signature-help spec: `../docs/signature-help.md`

Semantic analysis:
- `Context` = `{ properties, functions }`; `prop("Name")` is special-cased and validated against `Context.properties`:
  - `analyzer/src/analysis/mod.rs`

## Testing

Rust:

```bash
cargo test -p analyzer
```

Golden snapshots (formatter + diagnostics):

```bash
BLESS=1 cargo test -p analyzer
```

Locations:
- unit tests: `analyzer/src/tests/`
- golden runners: `analyzer/tests/diagnostics_golden.rs`, `analyzer/tests/format_golden.rs`

## Notes / known gaps

- Completion operator suggestions do not cover the full parsed operator set (`analyzer/src/ide/completion/items.rs`).
- Builtin support is still partial for some spec signatures (for example rich text/`DateRange`, lambda-based list APIs, binder semantics, and depth-sensitive `flat` typing):
  - `docs/builtin_functions/README.md`

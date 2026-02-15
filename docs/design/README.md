# Design (notion-formula-rs)

Start here for stable architecture and cross-crate contracts.
Implementation details that change frequently should live next to code (`analyzer/README.md`,
`analyzer_wasm/README.md`, `examples/vite/README.md`).

## Table of contents

- [Pipeline](#pipeline)
- [Source tree](#source-tree)
- [Data flow](#data-flow)
- [Public entry points](#public-entry-points)
- [Language contract (current implementation)](#language-contract-current-implementation)
- [Key design decisions](#key-design-decisions)
- [Contracts (hard rules)](#contracts-hard-rules)
- [Design philosophy](#design-philosophy)
- [Deep dives](#deep-dives)
- [Drift tracker / open questions](#drift-tracker--open-questions)

## Pipeline

```text
&str (UTF-8)
  -> lexer: tokens (incl. trivia + EOF) + diagnostics
  -> parser: ParseOutput (AST + parse diagnostics)
  -> semantic analysis (optional; needs Context) -> (Ty + semantic diagnostics)
  -> IDE helpers: formatter / completion / signature help
  -> WASM bridge: DTO v1 (UTF-16 spans/offsets)
```

## Source tree

```text
.
├─ analyzer/          Rust core analyzer (lexer/parser/AST/diagnostics/IDE helpers)
├─ analyzer_wasm/     wasm-bindgen boundary + UTF-16 conversion + DTO v1
├─ evaluator/         stub (currently unused)
├─ examples/vite/     Vite + CodeMirror demo consuming analyzer_wasm
└─ docs/              design docs + changelog guidance (start at docs/README.md)
```

Primary docs:

| Path | Primary doc |
|---|---|
| `analyzer/` | [`analyzer/README.md`](../../analyzer/README.md) |
| `analyzer_wasm/` | [`analyzer_wasm/README.md`](../../analyzer_wasm/README.md) |
| `examples/vite/` | [`examples/vite/README.md`](../../examples/vite/README.md) |
| `docs/` | [`docs/README.md`](../README.md) |

## Data flow

Stable boundary view:

- `&str` (UTF-8) -> lexer -> `Tokens` (incl. trivia + EOF) + lex diagnostics
- `Tokens` -> parser -> `ParseOutput { expr, diagnostics, tokens }`
- `ParseOutput.expr` + `Context` -> semantic analysis -> `(Ty, semantic diagnostics)`
- `analyze(source, ctx)` -> `AnalyzeResult { diagnostics, tokens, output_type }`
- `(expr, tokens, source)` -> formatter -> `String`
- `(source, cursor, Context)` -> IDE help -> `HelpResult { completion, signature_help }`
- Parser diagnostics may include `actions: Vec<CodeAction>` in byte coordinates.
- WASM conversion maps core byte spans/edits to UTF-16 DTO spans/edits.

## Public entry points

Rust (`analyzer/`):

- `analyzer::analyze_syntax(text: &str) -> SyntaxResult`
- `analyzer::analyze(text: &str, ctx: &Context) -> AnalyzeResult`
- `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)`
- `analyzer::format_expr(expr, source, tokens) -> String`
- `analyzer::completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput`
- `analyzer::ide_help(source, cursor_byte, ctx, config) -> HelpResult`
- `analyzer::ide_format(source, cursor_byte) -> Result<IdeApplyResult, IdeError>`
- `analyzer::ide_apply_edits(source, edits, cursor_byte) -> Result<IdeApplyResult, IdeError>`
- `analyzer::format_diagnostics(source, diags) -> String`

WASM (`analyzer_wasm/`):

- `new Analyzer(config: AnalyzerConfig)`
- `Analyzer.analyze(source) -> AnalyzeResult`
- `Analyzer.format(source, cursor_utf16) -> ApplyResult`
- `Analyzer.apply_edits(source, edits, cursor_utf16) -> ApplyResult`
- `Analyzer.help(source, cursor_utf16) -> HelpResult`

Tooling:

- TS DTO export: `cargo run -p analyzer_wasm --bin export_ts`

## Language contract (current implementation)

This section documents current lexer/parser behavior (code-backed):

- Identifiers: ASCII letters/`_` and non-ASCII codepoints are accepted.
- Numbers: ASCII digit integers only (no decimals yet).
- Strings: double-quoted strings, no escapes yet.
- Lists: trailing comma is rejected (`[1, 2,]` is a parse error).
- Operators:
  - `%` is modulo.
  - `^` is power and right-associative.
- Member access without a call is rejected (`receiver.method` must be `receiver.method(...)`).

## Key design decisions

These are the hard edges other code relies on:

- Spans/offsets: core spans are UTF-8 byte offsets, half-open `[start, end)`.
- WASM boundary encoding: DTO spans/offsets are UTF-16 code units, half-open `[start, end)`.
- Determinism: diagnostics deconfliction + formatting order are deterministic.
- Signature help is structured: UIs render segments, they do not parse signature strings.
- `AnalyzerConfig` is strict: object input; unknown top-level fields rejected.
- Semantic and DTO payloads avoid nullable "unknown" values where explicit domain values exist
  (`Ty::Unknown`, `"unknown"`).

## Contracts (hard rules)

These are stability guarantees. Contract changes require docs + tests + changelog updates.

### Spans and offsets

- Core spans are UTF-8 byte offsets (`analyzer/src/lexer/token.rs`).
- DTO spans/edits are UTF-16 code units (`analyzer_wasm/src/dto/v1.rs`).
- Conversion lives only in WASM (`analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`).
- `Diagnostic.line`/`col` are computed from byte offsets via `SourceMap::line_col` during
  WASM conversion (`analyzer_wasm/src/converter/shared.rs`).

### Token stream

- Token stream includes trivia (`DocComment`, `Newline`) and explicit `Eof`.
- `TokenQuery` is the canonical trivia-aware span/token neighbor API.

### AST + syntax invariants

- `ExprKind` is the closed set of core expression forms.
- Parser recovers best-effort with `ExprKind::Error` nodes and diagnostics.
- Member access without call is rejected.

### Diagnostics determinism

- Diagnostics deconflict by identical span with priority (`DiagnosticCode::priority`).
- One diagnostic survives per span unless equal-priority/equal-message merge rules apply.
- `format_diagnostics` output ordering is stable by span, priority, and message.

### Actions and edits

- Quick fixes are diagnostic-level payloads: `Diagnostic.actions: Vec<CodeAction>`.
- Core edit model is unified: `TextEdit { range, new_text }` in byte coordinates.
- WASM edit model is unified: `TextEdit` in UTF-16 coordinates.
- Core `ide_format` and `ide_apply_edits` accept byte cursor and return `{ source, cursor }`.
- WASM `format` and `apply_edits` accept UTF-16 cursor and return UTF-16 cursor.
- Core `ide_format` uses one full-document `TextEdit` through the same byte-edit pipeline as
  core `ide_apply_edits`.
- WASM forwards to core after UTF-16 ↔ byte conversion; failures throw `Err` (not payload enums).

### Signature help

- Signature shape uses `ParamShape { head, repeat, tail }` invariants.
- Signature help output is structured (`DisplaySegment[]`) for direct UI rendering.
- Active parameter mapping follows `docs/signature-help.md`.

### WASM `AnalyzerConfig`

- Constructor config must be an object.
- Unknown top-level fields are rejected.
- Current schema: `{ properties?: Property[], preferred_limit?: number | null }`.
- `preferred_limit = null` uses default `5`.
- `functions` are sourced from Rust builtins; JS does not provide them.

## Design philosophy

- Contracts-first: keep hard edges explicit, test-backed, and small.
- Best-effort parsing: return useful AST + diagnostics instead of failing fast.
- Determinism by default: stable ordering and tie-break rules for predictable UI behavior.
- Clear boundary: Rust core stays UTF-8; JS-facing boundary stays UTF-16.

## Deep dives

- Spans/tokens/trivia: [`docs/design/tokens-spans.md`](tokens-spans.md)
- `TokenQuery`: [`docs/design/tokenquery.md`](tokenquery.md)
- Builtins + types + postfix sugar: [`docs/design/builtins-and-types.md`](builtins-and-types.md)
- Completion + signature help behavior: [`docs/design/completion.md`](completion.md)
- WASM boundary + DTO v1: [`docs/design/wasm-boundary.md`](wasm-boundary.md)
- Demo integration (`examples/vite`): [`docs/design/demo-vite.md`](demo-vite.md)
- Test inventory: [`docs/design/testing.md`](testing.md)

Related:

- Signature help spec: [`docs/signature-help.md`](../signature-help.md)
- Builtins spec list: [`docs/builtin_functions/README.md`](../builtin_functions/README.md)

## Drift tracker / open questions

- Language spec gaps to formalize:
  - numeric grammar (decimals, edge cases)
  - string escape grammar
  - identifier character classes and normalization policy
- Semantic parity gap to track:
  - postfix validation still has narrower behavior than postfix inference/completion in some
    call-shape paths (`docs/design/builtins-and-types.md`).

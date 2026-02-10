# Design (notion-formula-rs)

Start here for the stable architecture and contracts.
Implementation details that change frequently (lexer/parser quirks, ranking heuristics, etc.) should live in
module READMEs next to code (for example, `analyzer/README.md`).

## Table of contents

- [Pipeline](#pipeline)
- [Source tree](#source-tree)
- [Data flow](#data-flow)
- [Public entry points](#public-entry-points)
- [Language contract (spec)](#language-contract-spec)
- [Key design decisions](#key-design-decisions)
- [Contracts (hard rules)](#contracts-hard-rules)
- [Design philosophy](#design-philosophy)
- [Deep dives](#deep-dives)
- [Drift tracker / open questions](#drift-tracker--open-questions)
- [Former overview → new homes](#former-overview--new-homes)

## Pipeline

```text
&str (UTF-8)
  -> lexer: Tokens (incl. trivia + EOF) + diagnostics
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

Informal, but stable at the boundaries:

- `&str` (UTF-8) → lexer → `Tokens` (incl. trivia + EOF) + lex diagnostics
- `Tokens` → parser → `ParseOutput { expr, tokens, diagnostics }`
- `ParseOutput.expr` + `Context` → semantic analysis → `(Ty, semantic diagnostics)`
- `(expr, tokens, source)` → formatter → `String`
- `(source, cursor, Context)` → completion/signature help → `CompletionOutput` (structured; no UI string parsing)
- (WASM only) Rust core outputs (UTF-8 spans) → converter → DTO v1 (UTF-16 spans/offsets)

See also:
- tokens/spans: [`docs/design/tokens-spans.md`](tokens-spans.md)
- completion/signature help behavior: [`docs/design/completion.md`](completion.md) + [`docs/signature-help.md`](../signature-help.md)
- WASM boundary + DTO v1: [`docs/design/wasm-boundary.md`](wasm-boundary.md)

## Public entry points

Rust (`analyzer/`):

- `analyzer::analyze(text: &str) -> Result<ParseOutput, Diagnostic>` (`analyzer/src/lib.rs`)
- `analyzer::semantic::analyze_expr(expr, ctx) -> (Ty, Vec<Diagnostic>)` (`analyzer/src/analysis/mod.rs`)
- `analyzer::format_expr(expr, source, tokens) -> String` (`analyzer/src/ide/format.rs`)
- `analyzer::completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput` (`analyzer/src/ide/completion/mod.rs`)
- `analyzer::format_diagnostics(source, diags) -> String` (`analyzer/src/diagnostics.rs`)

WASM (`analyzer_wasm/`):

- `analyze(source, context_json) -> AnalyzeResult` (`analyzer_wasm/src/lib.rs`, `analyzer_wasm/src/dto/v1.rs`)
- `complete(source, cursor_utf16, context_json) -> CompletionOutputView` (`analyzer_wasm/src/lib.rs`, `analyzer_wasm/src/dto/v1.rs`)
- `pos_to_line_col(source, pos_utf16) -> LineColView` (`analyzer_wasm/src/lib.rs`, `analyzer_wasm/src/dto/v1.rs`)

Tooling:

- TS DTO export: `cargo run -p analyzer_wasm --bin export_ts` (`analyzer_wasm/src/bin/export_ts.rs`)

## Language contract (spec)

This section is the *intended* stable language surface. For “what the current lexer/parser accept today”,
see [`analyzer/README.md` → “Syntax (current implementation)”](../../analyzer/README.md#syntax-current-implementation).

- Identifiers: non-ASCII identifiers are supported.
  - TODO: specify identifier character classes precisely (Unicode categories and normalization policy).
- Numbers: decimal numeric literals are part of the intended language surface.
  - TODO: write the full numeric grammar (and ensure analyzer behavior matches).
- Strings: double-quoted strings with standard escapes are part of the intended language surface.
  - TODO: write the full escape/spec (and ensure analyzer behavior matches).
- Lists: trailing comma is rejected (`[1, 2,]` is a parse error).
- Operators:
  - `%` is modulo.
  - `^` is power and is right-associative.

## Key design decisions

These are the “hard edges” that other code relies on:

- **Spans and offsets**: core spans are UTF-8 byte offsets, half-open `[start, end)`.
- **WASM boundary encoding**: DTO spans/offsets at the JS boundary are UTF-16 code units, half-open `[start, end)`.
- **Determinism**: diagnostics formatting + deconfliction must be deterministic (stable ordering and priority rules).
- **Signature help is structured**: UIs render `DisplaySegment[]`; they do not parse signature/type strings.
- **`context_json` is strict**: it must be non-empty JSON, and unknown top-level fields are rejected.

## Contracts (hard rules)

These constraints are stability guarantees. If a change breaks any of them, update design docs + tests and
call it out as a contract change.

### Spans and offsets

- Core spans are **UTF-8 byte offsets**, half-open `[start, end)`:
  - `analyzer/src/lexer/token.rs` (`Span`)
- WASM DTO spans/offsets are **UTF-16 code units**, half-open `[start, end)`:
  - `analyzer_wasm/src/dto/v1.rs` (`Span`)
- Encoding conversion lives only in the WASM layer and floors/clamps to valid boundaries:
  - `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`
  - tests: `analyzer_wasm/tests/analyze.rs`
- `pos_to_line_col` returns 1-based `(line, col)`; `col` is a Rust `char` count (Unicode scalar values), not UTF-16:
  - `analyzer/src/source_map.rs`
  - `analyzer_wasm/src/converter.rs`

### Token stream

- Token stream includes trivia + explicit EOF:
  - trivia: `TokenKind::DocComment(..)` and `TokenKind::Newline`
  - EOF: `TokenKind::Eof` with empty span
  - `analyzer/src/lexer/token.rs`
- `TokenQuery` is the canonical span→token-range + trivia-aware neighbor API:
  - `analyzer/src/parser/tokenstream.rs`

### AST

- `ExprKind` is the closed set of expression forms:
  - `analyzer/src/parser/ast.rs`
- Member access without a call is rejected (`receiver.method` must be `receiver.method(...)`):
  - `analyzer/src/parser/expr.rs`

### Diagnostics determinism

- Diagnostics deconflict by span using a priority order; formatting is deterministic:
  - `analyzer/src/diagnostics.rs` (`DiagnosticCode::priority`, `Diagnostics::push`, `format_diagnostics`)
  - `format_diagnostics` ordering: `(span.start, span.end, priority desc, message)` (then stable label ordering)
  - Deconfliction: one diag per span; higher priority wins; equal priority + identical message merges labels/notes

### Signatures and signature help

- Function signatures use `ParamShape { head, repeat, tail }` and enforce determinism invariants:
  - `analyzer/src/analysis/signature.rs` (`ParamShape::new`)
  - `analyzer/src/analysis/param_shape.rs`
- Signature help is structured (display segments + active parameter mapping):
  - Spec: [`docs/signature-help.md`](../signature-help.md)

### WASM `context_json`

- `context_json` must be a non-empty JSON string.
- Unknown top-level fields are rejected (`deny_unknown_fields`).
- Current schema is `{ properties: Property[], completion?: { preferred_limit?: number } }`.
- `functions` are not supplied by JS; they come from Rust builtins.
  - `analyzer_wasm/src/converter.rs` (`parse_context`)
  - tests: `analyzer_wasm/tests/analyze.rs`

## Design philosophy

- Contracts-first: keep the “hard edges” few, explicit, and test-backed.
- Best-effort parsing: produce an AST plus diagnostics (avoid cascading failures where possible).
- Determinism everywhere: stable output ordering and deconfliction rules to keep UIs predictable.
- Clear UI boundary: Rust core stays in UTF-8; editor-facing DTOs are UTF-16; UIs render structured segments.

## Deep dives

- Spans/tokens/trivia: [`docs/design/tokens-spans.md`](tokens-spans.md)
- `TokenQuery`: [`docs/design/tokenquery.md`](tokenquery.md)
- Builtins + types + sugar: [`docs/design/builtins-and-types.md`](builtins-and-types.md)
- Completion + signature help behavior: [`docs/design/completion.md`](completion.md)
- WASM boundary + DTO v1: [`docs/design/wasm-boundary.md`](wasm-boundary.md)
- Demo integration (`examples/vite`): [`docs/design/demo-vite.md`](demo-vite.md)
- Test inventory: [`docs/design/testing.md`](testing.md)

Related:

- Signature help spec: [`docs/signature-help.md`](../signature-help.md)
- Builtins spec list (doc-driven; sync-checked by tests): [`docs/builtin_functions/README.md`](../builtin_functions/README.md)

## Drift tracker / open questions

- Language spec TODOs:
  - numeric grammar (decimals, formats, edge cases)
  - string escapes + exact rules
  - identifier character classes + normalization policy
- Repo housekeeping:
  - `rustc/` directory intent is unclear (document it with a README, or remove it).

## Former overview → new homes

`docs/overview-analyzer.md` was removed. Every prior section has an explicit replacement:

| Former overview section | New home | Notes |
|---|---|---|
| Repository layout (workspace) | [`docs/design/README.md`](#source-tree) | Workspace + demo are listed under “Source tree”. |
| analyzer module map | [`analyzer/README.md`](../../analyzer/README.md#entry-points) + [`analyzer/README.md`](../../analyzer/README.md#module-map) | Code-backed entry points + module ownership map. |
| Tokens, spans, and token ranges | [`docs/design/tokens-spans.md`](tokens-spans.md) | Deep dive (Span, trivia, EOF, TokenRange). |
| Core span model (Rust) | [`docs/design/README.md`](#spans-and-offsets) + [`docs/design/tokens-spans.md`](tokens-spans.md#span-core) | Core = UTF-8 byte offsets, half-open `[start,end)`. |
| Span → TokenRange | [`docs/design/tokens-spans.md`](tokens-spans.md#tokenrange-and-tokens_in_span) | `tokens_in_span` half-open token ranges + insertion-point rules. |
| TokenQuery (canonical token neighbor API) | [`docs/design/tokenquery.md`](tokenquery.md) | API surface + trivia scanning intent. |
| Implemented syntax (lexer/parser) | [`analyzer/README.md`](../../analyzer/README.md#syntax-current-implementation) | Current implementation details live next to code. |
| Parser error recovery (high-level) | [`analyzer/README.md`](../../analyzer/README.md#parser-recovery) | Recovery rules live under “Syntax (current implementation)”. |
| Known gaps (completion operator list, etc.) | [`analyzer/README.md`](../../analyzer/README.md#notes--known-gaps) | Keep “known gaps” close to the implementation. |
| Builtins + sugar (where they live) | [`docs/design/builtins-and-types.md`](builtins-and-types.md) | Signatures, `ParamShape`, type model, sugar. |
| Completion model + ranking | [`docs/design/completion.md`](completion.md) | Replace span rules, query/type ranking, preferred indices. |
| Signature help rendering + mapping rules | [`docs/signature-help.md`](../signature-help.md) + [`docs/design/completion.md`](completion.md#signature-help) | Structured segments; no frontend parsing. |
| WASM boundary invariants | [`docs/design/wasm-boundary.md`](wasm-boundary.md) + [`analyzer_wasm/README.md`](../../analyzer_wasm/README.md) | DTO boundary + conversion rules. |
| Encoding boundary design | [`docs/design/wasm-boundary.md`](wasm-boundary.md#encoding-boundary-hard-rule) | UTF-8 core ↔ UTF-16 boundary. |
| Line / column handling | [`docs/design/wasm-boundary.md`](wasm-boundary.md#linecolumn-handling) | Derived via `pos_to_line_col`; not stored in spans. |
| Text edits / cursor rebasing | [`docs/design/wasm-boundary.md`](wasm-boundary.md#text-edits--cursor-rebasing) | UTF-16 ranges in DTOs; rebasing rules. |
| Tests (what exists and where) | [`docs/design/testing.md`](testing.md) | Rust + WASM + demo tests; snapshot update flow. |
| Vite demo tests + regression coverage | [`docs/design/testing.md`](testing.md#vite-demo-tests-examplesvite) | Includes the “run wasm:build once” note and coverage list. |
| Vite demo architecture (current) | [`examples/vite/README.md`](../../examples/vite/README.md#architecture) + [`docs/design/demo-vite.md`](demo-vite.md#where-the-integration-lives) | Demo file map + UI-owned behavior notes. |
| Cursor placement invariants (demo) | [`docs/design/demo-vite.md`](demo-vite.md#cursor-placement-invariants) | Byte→UTF-16 conversion + cursor-after-edit rules. |
| Playwright host configuration | [`docs/design/demo-vite.md`](demo-vite.md#playwright-host-config) | `PW_HOST` override; `PW_PORT` optional (stable worktree-derived port when unset). |
| Current architectural invariants | [`docs/design/README.md`](#key-design-decisions) + [`docs/design/README.md`](#contracts-hard-rules) | Contracts-first summary + full contract list. |

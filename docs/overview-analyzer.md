# Notion Formula Analyzer (Rust)

This document is a code-backed overview of what is implemented today (parser/formatter/diagnostics/completion + a WASM bridge), with file paths for the source of truth. When behavior changes, update this doc (rewrite/remove outdated parts; don’t append-only).

## Repository layout (workspace)

Rust workspace members (from `Cargo.toml`):

- `analyzer/`: frontend engine (lex → parse → diagnostics → format; plus completion + a small semantic pass)
- `analyzer_wasm/`: `wasm-bindgen` bridge exporting `analyze(...)`, `complete(...)`, and `pos_to_line_col(...)` to JS/TS (UTF-16 boundary)
- `evaluator/`: currently a stub crate

Non-workspace directories:

- `examples/vite/`: Vite + CodeMirror demo consuming `analyzer_wasm`
- `docs/`: design notes / specs (this file lives here)

---

## `analyzer` module map

Entry points:

- `analyzer/src/lib.rs`:
  - `analyze(text: &str) -> Result<ParseOutput, Diagnostic>`
  - `analyze_with_context(text: &str, ctx: Context) -> Result<ParseOutput, Diagnostic>`
  - re-exports: `complete(...)`, `complete_with_context(...)`, `format_expr(...)`, `format_diagnostics(...)`, core token/span types

Core modules:

- `analyzer/src/lexer/mod.rs`: lexer producing `Token`s + lex diagnostics (strings, numbers, comments, operators)
- `analyzer/src/parser/mod.rs`: Pratt parser plumbing + binding power tables; builds `ParseOutput { expr, diagnostics, tokens }`
- `analyzer/src/parser/expr.rs`: expression grammar (primary/prefix/infix/ternary/call/member-call) + recovery
- `analyzer/src/ast.rs`: AST node types (preserves explicit grouping via `ExprKind::Group`)
- `analyzer/src/format.rs`: formatter for `Expr` using tokens/source for comment attachment; enforces width/indent rules; uses `TokenQuery`
- `analyzer/src/diagnostics.rs`: `Diagnostic` model + stable `format_diagnostics(...)` output (sorted by span/message)
- `analyzer/src/semantic/mod.rs`: minimal type checking driven by `Context { properties, functions }`
- `analyzer/src/completion.rs`: byte-offset completion + signature help for editor integrations
- `analyzer/src/source_map.rs`: byte offset → `(line,col)` plus `byte_offset_to_utf16(...)`
- `analyzer/src/token.rs`: token kinds, `Span` (byte offsets), trivia classification, `tokens_in_span(...)`
- `analyzer/src/tokenstream.rs`: `TokenCursor` (parser) + `TokenQuery` (span/token/trivia scanning)

---

## Tokens, spans, and token ranges

### Core span model (Rust)

- `Span { start: u32, end: u32 }` in `analyzer/src/token.rs`
- Represents **byte offsets**
- Half-open semantics: `[start, end)`

### Span → TokenRange

- `tokens_in_span(tokens, span: Span) -> TokenRange`
- Returns half-open token index range `[lo, hi)`
- Handles:
  - empty spans
  - stable “insertion point” behavior for empty spans
  - trivia tokens (comments/newlines) and EOF (which has an empty span)

---

## TokenQuery (canonical token neighbor API)

Location:

- `analyzer/src/tokenstream.rs`

`TokenQuery<'a>` centralizes **span → token range → trivia scanning**, replacing ad-hoc loops in formatter and utilities.

Key capabilities:

- `range_for_span(span)` → TokenRange
- `prev_nontrivia(idx)`
- `next_nontrivia(idx)`
- `first_in_range(range)`
- `last_in_range(range)`
- `leading_trivia_before(idx)`
- `trailing_trivia_until_newline_or_nontrivia(idx)`
- `bounds_usize(range)`

Design intent:

- Single authoritative place for trivia / neighbor scanning
- Reduce duplicated index arithmetic in formatter and comment logic
- Token index semantics are always half-open `[lo, hi)`

---

## Implemented syntax (what the lexer/parser accept)

Literals and identifiers:

- numbers: integer digits only (`analyzer/src/lexer/mod.rs`)
- strings: double-quoted, no escapes (“v1” behavior)
- identifiers: ASCII letters/`_` plus any non-ASCII codepoint

Expression forms (AST):

- unary: `!expr`, `-expr`
- binary: `< <= == != >= > && || + - * / % ^`
- ternary: `cond ? then : otherwise`
- grouping: `(expr)` preserved as `ExprKind::Group`
- calls: `ident(arg1, arg2, ...)`
- member-call: `receiver.method(arg1, ...)`
  (member access without `(...)` is rejected)

Known gaps:

- boolean literals (`true` / `false`) lex as identifiers (the lexer does not emit `LitKind::Bool` today)
- `not` is suggested by completion but is not a lexer/parser operator today
- completion operator list does not include every parsed operator

---

## Builtins + sugar (where they live)

Semantic analysis (`analyzer/src/semantic/mod.rs`):

- `Context` is `{ properties: Vec<Property>, functions: Vec<FunctionSig> }`.
- Builtin function signatures are defined in `builtins_functions()` (this list is the source of truth and is larger than a couple of functions).
- `prop("Name")` is **special-cased** in the semantic analyzer (it is not a `FunctionSig`):
  - expects exactly 1 argument
  - argument must be a string literal
  - property name must exist in `Context.properties` (else a diagnostic is emitted)
- `if(condition, then, else)` is special-cased for type checking:
  - expects exactly 3 arguments
  - `condition` must be boolean (if known)
  - result type is a join of `then`/`else` (currently `Unknown` if they differ)
- Postfix sugar typing:
  - `condition.if(then, else)` is treated like `if(condition, then, else)` **for typing only** when `if` exists in `Context.functions`.

Completion (`analyzer/src/completion.rs`):

- Cursor and `replace` spans are **byte offsets** in the core analyzer.
- Completion item kinds: `Function`, `Builtin`, `Property`, `Operator`.
- Builtin completion items include `true`, `false`, `not` (note: today these still lex/parse as identifiers; `not` is not an operator).
- Postfix completion: `.if()` is offered after an atom when `if` exists in context.
- Property completion items insert `prop("Name")` and can be disabled via `Property.disabled_reason` (disabled items have no `primary_edit`/cursor).
- When `CompletionOutput.replace` is non-empty, the analyzer derives a “query” from the source substring covered by the replace span (lowercased; whitespace/underscores removed). If the normalized query is empty, no fuzzy ranking is applied and `preferred_indices` is `[]`.
- With a non-empty query, completion items are fuzzy-ranked by **subsequence match** on `CompletionItem.label` (case-insensitive). Ranking prefers: prefix matches, fewer gaps / longer contiguous runs, earlier matches, and shorter labels; ties are deterministic and use kind priority (`Function` > `Builtin` > `Property` > `Operator`) then original index.
- `CompletionOutput.preferred_indices` is the analyzer-provided “smart picks” for UI default selection / recommendation: indices of up to `preferred_limit` matched+enabled items (high-score first, then lower-score matches). `preferred_limit` defaults to `5`, is configurable via `context_json.completion.preferred_limit`, and `0` disables preferred computation (always returns `[]`).
- Signature help is computed only when the cursor is inside a call and uses `Context.functions`.

---

## WASM boundary invariants (`analyzer_wasm`)

Exported functions (`analyzer_wasm/src/lib.rs`):

- `analyze(source: String, context_json: String) -> Result<JsValue, JsValue>`
- `complete(source: String, cursor_utf16: usize, context_json: String) -> Result<JsValue, JsValue>`
- `pos_to_line_col(source: String, pos_utf16: u32) -> JsValue`

`context_json` invariants (enforced by `Converter::parse_context(...)` and covered by WASM tests):

- must be non-empty and valid JSON
- unknown top-level fields are rejected (`deny_unknown_fields`)
- schema today is: `{ "properties": Property[], "completion"?: { "preferred_limit"?: number } }`
- `Context.functions` is populated from Rust `builtins_functions()` (JS cannot supply functions today)

---

### Encoding boundary design

**Core analyzer (Rust):**

- All spans and cursor positions are **byte offsets**

**JS / WASM boundary:**

- All exposed spans use **UTF-16 code unit offsets**
- Conversion occurs only in the WASM layer

DTO types (`analyzer_wasm/src/dto/v1.rs`):

```ts
Span { start: u32, end: u32 } // UTF-16 code units, half-open [start, end)
SpanView { range: Span }
LineColView { line: u32, col: u32 }
TextEditView { range: Span, new_text: string }
CompletionOutputView { items: CompletionItemView[], replace: Span, signature_help: SignatureHelpView | null, preferred_indices: number[] }
```

Byte ↔ UTF-16 conversion lives in WASM

Files:

- `analyzer_wasm/src/offsets.rs`
- `analyzer_wasm/src/span.rs`
- `analyzer_wasm/src/converter.rs`
- `analyzer_wasm/src/text_edit.rs`

Helpers:

- `utf16_offset_to_byte(source, utf16_pos)`
- `byte_offset_to_utf16_offset(source, byte_pos)`
- `byte_span_to_utf16_span(source, span)`

JS never deals with byte offsets.
Rust core never deals with UTF-16 offsets.

---

### Line / Column handling (now derived, not stored)

- `DTO SpanView` no longer stores line or col
- `Line/column` is computed lazily via: `pos_to_line_col(source, pos_utf16) -> LineColView` (1-based)
- `pos_to_line_col` takes a **UTF-16** offset at the JS boundary and maps through byte offsets internally.

Text edits / cursor rebasing:

- Analyzer `TextEdit` ranges are **byte offsets**.
- WASM DTO `TextEditView` ranges are **UTF-16**.
- `analyzer_wasm/src/text_edit.rs` applies byte edits in **descending start offset order** and can rebase a byte cursor through edits.
- The completion DTO’s optional `cursor` is expressed in **UTF-16 in the updated document**.

---

Tests (what exists and where)

Rust unit tests (analyzer/src/tests/)

Coverage:

- lexer
- parser
- span invariants
- token-in-span
- TokenQuery behavior
- formatter
- completion DSL
- UTF-16 helpers
- semantic checks

---

Rust golden tests

Runners:

- `analyzer/tests/format_golden.rs`
- `analyzer/tests/diagnostics_golden.rs`

Fixtures:

- `analyzer/tests/format/*.formula → *.snap`
- `analyzer/tests/diagnostics/*.formula → *.snap`

Snapshots updated via:

```bash
BLESS=1 cargo test -p analyzer
```

WASM tests

- analyzer_wasm/tests/analyze.rs
- Validates:
  - UTF-16 span correctness
  - token span integrity
  - diagnostics mapping
  - `context_json` validation rules (non-empty JSON; unknown fields rejected)

Vite demo tests (TypeScript)

- Unit tests: examples/vite/tests/unit/ (Vitest)
- E2E tests: examples/vite/tests/e2e/ (Playwright)

Regression coverage:

- token highlighting
- diagnostics propagation
- chip spans
- UI behavior
- completion cursor placement (including UTF-16 text)
- completion list scroll-into-view behavior

### Vite demo completion UI

The Vite demo renders completions returned by the WASM `complete(...)` export entirely on the TypeScript side.

Primary files:

- `examples/vite/src/analyzer/wasm_client.ts`: calls `wasm.complete(...)` via `completeSource(...)`
- `examples/vite/src/ui/formula_panel_view.ts`: renders the completion panel and applies selected completions

Rendering behavior:

- Completions are displayed as a list under the “Suggestions” panel.
- Function completions are grouped by `category` (UI-owned grouping; no WASM changes).
- Non-function completions are grouped by consecutive `kind` changes (UI-owned grouping).
- Selection and navigation operate over a `completionRows` model (headers + items):
  - Headers are not selectable.
  - Arrow key navigation skips over header rows.
- Applying a completion maps the selected row back to the underlying `CompletionItem` index.

Styling:

- Group headers use `.completion-group-header` in `examples/vite/src/style.css`.

Cursor placement invariants

- Analyzer core (`analyzer/`) computes completion cursors as **byte offsets**.
- The analyzer’s optional `CompletionItem.cursor` is intended to represent the desired cursor position
  **in the updated document after applying the primary edit** (e.g., `if()` => inside the `(`).
- The WASM bridge (`analyzer_wasm/`) converts completion edit ranges and cursor values to **UTF-16**
  for JS/editor usage, and accounts for shifts from `additional_edits` that occur before the primary edit.
- The Vite demo uses `item.cursor` when present; otherwise it falls back to `primary_edit` end plus any
  shifts from additional edits before the primary edit (`examples/vite/src/ui/formula_panel_view.ts`).

Playwright host configuration

- The Playwright suite in `examples/vite/` boots Vite via the `webServer` setting in
  `examples/vite/playwright.config.ts`.
- Host/port can be overridden with:
  - `PW_HOST` (defaults to `127.0.0.1`)
  - `PW_PORT` (defaults to `5173`)

Current architectural invariants

- Analyzer core uses byte offsets only
- JS/WASM boundary uses UTF-16 offsets only
- Span semantics are half-open [start, end) everywhere
- TokenQuery is the canonical trivia/token neighbor API
- Token ranges derive from spans via tokens_in_span
- Line/column is derived lazily, not stored
- Encoding conversion lives only in WASM

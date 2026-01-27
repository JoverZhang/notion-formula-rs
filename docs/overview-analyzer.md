# Notion Formula Analyzer (Rust)

This document is a code-backed overview of what is implemented today (parser/formatter/diagnostics/completion + a WASM bridge), with file paths for the source of truth.

## Repository layout (workspace)

Rust workspace members (from `Cargo.toml`):

- `analyzer/`: frontend engine (lex → parse → diagnostics → format; plus completion + a small semantic pass)
- `analyzer_wasm/`: `wasm-bindgen` bridge exporting `analyze(...)` and `complete(...)` to JS/TS
- `evaluator/`: currently a stub crate

Non-workspace directories:

- `examples/vite/`: Vite + CodeMirror demo consuming `analyzer_wasm`
- `docs/`: design notes / specs (this file lives here)

---

## `analyzer` module map

Entry points:

- `analyzer/src/lib.rs`:
  - `analyze(text)`
  - `analyze_with_context(text, ctx)`
  - `complete_with_context(text, cursor_byte, ctx)`

Core modules:

- `analyzer/src/lexer/mod.rs`: lexer producing `Token`s + lex diagnostics (strings, numbers, comments, operators)
- `analyzer/src/parser/mod.rs`: Pratt parser plumbing + binding power tables; builds `ParseOutput { expr, diagnostics, tokens }`
- `analyzer/src/parser/expr.rs`: expression grammar (primary/prefix/infix/ternary/call/member-call) + recovery
- `analyzer/src/ast.rs`: AST node types (preserves explicit grouping via `ExprKind::Group`)
- `analyzer/src/format.rs`: formatter for `Expr` using tokens/source for comment attachment; enforces width/indent rules; uses `TokenQuery`
- `analyzer/src/diagnostics.rs`: `Diagnostic` model + stable `format_diagnostics(...)` output (sorted by span/message)
- `analyzer/src/semantic/mod.rs`: minimal type checking driven by `Context { properties, functions }`
- `analyzer/src/completion.rs`: byte-offset completion + signature help for editor integrations
- `analyzer/src/source_map.rs`: byte offset → `(line,col)` and byte offset ↔ UTF-16 helpers
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
  - EOF insertion points
  - trivia overlap
  - out-of-bounds spans

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

- boolean literals (`true` / `false`) lex as identifiers
- completion operator list does not include every parsed operator

---

## Builtins + sugar (where they live)

Semantic builtins:

- builtin function list: `builtins_functions()` → `if`, `sum`
- `if()` checks condition is boolean
- property lookup: `prop("Name")`
  - arity = 1
  - argument must be string literal
  - property must exist in `Context.properties`
- postfix sugar typing:
  - `cond.if(a, b)` treated like `if(cond, a, b)` for typing only

Completion builtins and sugar:

- expression-start keywords: `not`, `true`, `false`
- postfix completion: `.if()` offered when `if` exists in context

---

## WASM boundary invariants (`analyzer_wasm`)

Exported functions (`analyzer_wasm/src/lib.rs`):

- `analyze(source: String, context_json?: String)`
- `complete(source: String, cursor_utf16: usize, context_json?: String)`
- `utf16_pos_to_line_col(source: String, pos_utf16: u32)`

---

### Encoding boundary design

**Core analyzer (Rust):**

- All spans and cursor positions are **byte offsets**

**JS / WASM boundary:**

- All exposed spans use **UTF-16 code unit offsets**
- Conversion occurs only in the WASM layer

DTO types (`analyzer_wasm/src/dto/v1.rs`):

```ts
Utf16Span { start: u32, end: u32 } // half-open [start, end)
SpanView { range: Utf16Span }
LineColView { line: u32, col: u32 }
TextEditView { range: Utf16Span, new_text: string }
```

Byte ↔ UTF-16 conversion lives in WASM

Files:

- `analyzer_wasm/src/offsets.rs`
- `analyzer_wasm/src/span.rs`

Helpers:

- `utf16_offset_to_byte(source, utf16_pos)`
- `byte_offset_to_utf16_offset(source, byte_pos)`
- `byte_span_to_utf16_span(source, span)`

JS never deals with byte offsets.
Rust core never deals with UTF-16 offsets.

---

### Line / Column handling (now derived, not stored)

- `DTO SpanView` no longer stores line or col
- `Line/column` is computed lazily via: `utf16_pos_to_line_col(source, pos_utf16) -> LineColView`
- Design intent:
  - range is canonical
  - line/col is derived only when needed
  - Avoid storing redundant positional data

- Design intent:
  - range is canonical
  - line/col is derived only when needed
  - Avoid storing redundant positional data

Text edits

- Multiple edits applied in descending offset order
- Prevents offset shifting bugs
- All edit ranges are UTF-16 at JS boundary

---

Token stream utilities

- TokenCursor — parser cursor over token stream
- TokenQuery — span → range → trivia neighbor API

Both live in:

- `analyzer/src/tokenstream.rs`

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
- Items are grouped **purely in the UI** (no WASM changes) by inserting a header row whenever the
  completion item’s `kind` changes (consecutive grouping).
  - This relies on the incoming `items` array order being meaningful/stable.
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
  for JS/editor usage, and must account for any `additional_edits` that occur before the primary edit.
  - Cursor shifting must only account for edits that are actually applied (invalid ranges or non-UTF-8
    boundaries are skipped by the edit applier).

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

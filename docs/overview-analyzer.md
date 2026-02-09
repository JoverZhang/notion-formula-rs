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
  - re-exports: `complete(text, cursor, ctx, config)`, `CompletionConfig`, `format_expr(...)`, `format_diagnostics(...)`, core token/span types

Core modules:

- `analyzer/src/lexer/mod.rs`: lexer producing `Token`s + lex diagnostics (strings, numbers, comments, operators)
- `analyzer/src/lexer/token.rs`: token model (`TokenKind`, `Token`, `Span`, `tokens_in_span(...)`, etc.)
- `analyzer/src/parser/mod.rs`: Pratt parser plumbing + binding power tables; builds `ParseOutput { expr, diagnostics, tokens }`
- `analyzer/src/parser/expr.rs`: expression grammar (primary/prefix/infix/ternary/call/member-call) + recovery
- `analyzer/src/parser/ast.rs`: AST node types (preserves explicit grouping via `ExprKind::Group`)
- `analyzer/src/parser/tokenstream.rs`: `TokenCursor` (parser) + `TokenQuery` (span/token/trivia scanning)
- `analyzer/src/ide/display.rs`: canonical UI display formatting (e.g. `format_ty(...)` used by signature help)
- `analyzer/src/ide/format.rs`: formatter for `Expr` using tokens/source for comment attachment; enforces width/indent rules; uses `TokenQuery`
- `analyzer/src/diagnostics.rs`: `Diagnostic { code, message, span, labels }` model + stable `format_diagnostics(...)` output (sorted by span, then priority, then message; deconflicts diagnostics with identical spans)
- `analyzer/src/analysis/mod.rs`: minimal type checking driven by `Context { properties, functions }`
- `analyzer/src/ide/completion/mod.rs`: byte-offset completion + signature help for editor integrations (pipeline/position/items/signature + ranking/matchers)
- `analyzer/src/source_map.rs`: byte offset → `(line,col)`

---

## Tokens, spans, and token ranges

### Core span model (Rust)

- `Span { start: u32, end: u32 }` in `analyzer/src/lexer/token.rs`
- Represents **UTF-8 byte offsets** into the original source string (safe to slice as `&source[start..end]`).
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

- `analyzer/src/parser/tokenstream.rs`

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

- unary: `!expr`, `not expr`, `-expr`
- binary: `< <= == != >= > && || + - * / % ^`
- ternary: `cond ? then : otherwise`
- grouping: `(expr)` preserved as `ExprKind::Group`
- list literal: `[expr (, expr)*]` preserved as `ExprKind::List { items: Vec<Expr> }`
  - empty list `[]` is allowed
  - trailing comma is a parse error (e.g. `[1,2,]`)
- calls: `ident(arg1, arg2, ...)`
- member-call: `receiver.method(arg1, ...)`
  (member access without `(...)` is rejected)

Known gaps:

- completion operator list does not include every parsed operator

Parser error recovery (high-level):

- The parser is best-effort: it emits diagnostics and produces an AST with `ExprKind::Error` placeholders to avoid cascading errors.
- Diagnostics are deconflicted by span using a priority order (so “missing closing delimiter” at EOF suppresses secondary “missing expression” errors at the same insertion point).
- Delimited comma-separated sequences (call args `(...)` and list literals `[...]`) share a single recovery routine (`analyzer/src/parser/expr.rs`):
  - missing commas between items emit `expected ',' or ...` and parsing continues as if a comma was inserted
  - missing items around commas (e.g. `f(,a)`, `f(1, ,2)`) produce `ExprKind::Error` entries in the sequence
  - missing/mismatched closing delimiters emit a label on the opening delimiter plus a fix-it label (`insert ')'` / `insert ']'` at EOF; `replace ...` when a different closing delimiter is present)
- Ternary recovery handles missing `then`/`else` expressions without consuming closing delimiters (so surrounding `)`/`]` parsing can still succeed).
- Member-call recovery skips redundant dots (e.g. `a..if(b,c)`), emitting a diagnostic but still parsing the call.

---

## Builtins + sugar (where they live)

Semantic analysis (`analyzer/src/analysis/mod.rs`):

- `Context` is `{ properties: Vec<Property>, functions: Vec<FunctionSig> }`.
- Builtin function signatures are defined in `builtins_functions()`.
- Builtin signatures are declared via a small macro DSL in `analyzer/src/analysis/builtins/macros.rs` and used from `analyzer/src/analysis/functions.rs`.
- `FunctionSig` includes required `category: FunctionCategory` and required `detail: String` (used by completion/signature help).
- `FunctionSig` models parameters via `params: ParamShape { head, repeat, tail }`:
  - `head`: fixed prefix params (appear once)
  - `repeat`: repeating group params (appear 1+ times when non-empty)
  - `tail`: fixed suffix params (appear once after the repeat group)
- `ParamSig` is `{ name: String, ty: Ty, optional: bool }`.
- `ParamShape::new(...)` enforces invariants for builtin declarations:
  - repeat params must not be optional
  - tail params may be optional but must be suffix-only (once an optional tail param appears, no required tail params may follow)
- `FunctionSig` can declare `generics: Vec<GenericParam>`; a `GenericParam` is `{ id: GenericId, kind: GenericParamKind }` (no display name; UI renders `T0`, `T1`, ...).
- Semantic analysis is inference-first:
  - `infer_expr_with_map(expr, ctx, &mut TypeMap)` computes a `TypeMap` of `ExprId`/`NodeId -> Ty`.
  - Ternary inference (`cond ? then : otherwise`) joins branch types:
    - if either branch is `Unknown`, the ternary type is `Unknown`
    - otherwise, the ternary type is a deterministic union of both branch types (`normalize_union`)
  - `analyze_expr` returns the inferred root type and emits diagnostics by comparing inferred argument types to builtin signatures (arity + expected types).
    - Validation is arity/shape-first: if a call has an arity/shape error, the analyzer emits that single diagnostic and **does not** emit additional per-argument type mismatch diagnostics for the same call.
  - Type acceptance (`ty_accepts` in `analyzer/src/analysis/mod.rs`):
    - `Unknown` is permissive only on the **actual** side (when inference cannot determine a type, it does not produce additional mismatch noise).
    - `Ty::Generic(_)` is only a wildcard on the **expected** side (generic _inferred actuals_ do not silently pass validation).
    - `Ty::Union(...)` uses containment semantics:
      - `expected = Union(E1|E2|...)` accepts `actual = Union(A1|A2|...)` iff each `Ai` is accepted by `expected`
      - `expected = T` accepts `actual = Union(A1|...)` iff `T` accepts each `Ai`
    - `Ty::List(E)` accepts `Ty::List(A)` iff `E` accepts `A` (directional/covariant).
- `prop("Name")` is **special-cased** in the semantic analyzer (it is not a `FunctionSig`):
  - expects exactly 1 argument
  - argument must be a string literal
  - property name must exist in `Context.properties` (else a diagnostic is emitted)
- Generic inference is driven by `FunctionSig.generics` + `Ty::Generic` (see `analyzer/src/analysis/infer.rs`):
  - `Plain` generics accumulate permissively (conflicts form a union); `Unknown` does not bind.
  - `if<T: Variant>(condition: boolean, then: T, else: T) -> T`
    - `Variant` generics:
      - if **any** participating actual type is `Unknown`, the instantiated generic becomes `Unknown` (Unknown propagates)
      - otherwise, all participating concrete types are accumulated into a deterministic union
  - `ifs<T: Variant>([condition: boolean, value: T]..., default: T) -> T`
    - same `Variant` rules as `if`
- Postfix sugar typing/inference:
  - For postfix-capable builtins, `receiver.fn(arg1, ...)` is treated like `fn(receiver, arg1, ...)` (fixed-arity signatures only).

Completion (`analyzer/src/ide/completion/mod.rs`, ranking/matching in `analyzer/src/ide/completion/rank.rs` + `analyzer/src/ide/completion/matchers.rs`):

- Public entrypoint: `completion::complete(text: &str, cursor: usize, ctx: Option<&semantic::Context>, config: CompletionConfig) -> CompletionOutput`.
- Cursor and `replace` spans are **byte offsets** in the core analyzer.
- Completion item kinds: `Function`, `Builtin`, `Property`, `Operator`.
- Builtin completion items include the reserved keywords `true`, `false`, `not` (`not` is a unary operator; `true`/`false` are boolean literals).
- Postfix completion is driven by a single builtin-derived allowlist (`postfix_capable_builtin_names()` in `analyzer/src/analysis/mod.rs`), defined as builtins with a **flat parameter list** that has more than one parameter (so there is at least one non-receiver argument):
  - after an atom: `.if()` is offered (inserts the leading `.`)
  - after `.` with a receiver atom: `.if` is offered and inserts `if()` (the `.` is already in the source)
- Property completion items insert `prop("Name")` and can be disabled via `Property.disabled_reason` (disabled items have no `primary_edit`/cursor).
- At an identifier boundary (cursor at the end of an identifier token), the analyzer treats completion as “prefix editing” only if the prefix can be extended by something in-scope:
  - builtins `true`/`false`/`not` via case-insensitive prefix match
  - context functions/properties via case-insensitive prefix match (excluding exact matches)
- When `CompletionOutput.replace` is non-empty, the analyzer derives a “query” from the source substring covered by the replace span. If the substring contains any non-identifier characters (identifier-like = ASCII alnum + `_` + whitespace), no fuzzy ranking is applied and `preferred_indices` is `[]`. Otherwise the query is normalized (lowercased; whitespace/underscores removed); if the normalized query is empty, fuzzy ranking is also skipped.
- With a non-empty query, completion ranking applies to `CompletionKind::Function` and `CompletionKind::Property` labels (the identifiers users type). In member-access (after-dot) prefix completion, postfix-method items are **filtered** to only those matching the query and then ranked using the same normalization/matching (matching is computed on the label **without** the leading `.`). Query and label are normalized by lowercasing and removing `_`. Items are ranked by:
  1. exact match (`label_norm == query_norm`)
  2. substring contains (`label_norm` contains `query_norm`)
  3. fuzzy subsequence match (existing subsequence scoring)
     Within exact/contains, shorter normalized labels rank first (and for contains, earlier substring occurrence breaks ties); fuzzy ties use the subsequence score; all remaining ties are deterministic by original index. Other completion kinds are left in original relative order after matched items.
- When type ranking is applied (cursor at expr-start inside a call with a known expected argument type), items are grouped into contiguous runs by `CompletionKind` _before_ query ranking. When query ranking applies, it may reorder across kinds.
  - Type ranking is skipped when the expected argument type is `Unknown` or `Generic(_)` (wildcard-ish and not informative).
- `CompletionOutput.preferred_indices` is the analyzer-provided “smart picks” for UI default selection / recommendation: indices of up to `preferred_limit` enabled items that matched the query (in the already-ranked order). `preferred_limit` defaults to `5`, is configurable via `context_json.completion.preferred_limit`, and `0` disables preferred computation (always returns `[]`).
- Signature help is computed only when the cursor is inside a call and uses `Context.functions`.
  - Signature help is **call-site instantiated**:
    - it best-effort infers argument expression types from the source
    - it instantiates the `FunctionSig` using the same unification/substitution logic as semantic inference (`instantiate_sig` in `analyzer/src/analysis/infer.rs`)
    - type strings are formatted via `analyzer/src/ide/display.rs` (`format_ty(...)`); `List(Union(...))` renders as `(A | B)[]`
    - instantiated `Unknown` is rendered as `unknown` (including unconstrained generics)
    - parameters prefer per-argument inferred (actual) types when the argument expression is non-empty **and** the inferred type is helpful/compatible (e.g. for generic and union-typed slots); empty argument slots fall back to instantiated expected types
  - Signature help output is structured (no frontend parsing):
    - `signatures[n].segments`: `DisplaySegment[]` (punctuation split into its own segments; params carry `param_index`)
    - `active_signature`: selected overload index (currently always `0`)
    - `active_parameter`: selected parameter index (matches `DisplaySegment.param_index`; excludes `...` and any receiver prefix)
  - For postfix calls `<receiver>.<callee>(...)` where `<callee>` is a postfix-capable builtin:
    - the receiver slot is rendered as a prefix segment sequence: `(<receiver_param>).` before the function name
    - receiver segments have `param_index = None` and are never highlighted
  - Repeat-group shapes (a `ParamShape` with non-empty `repeat`) are pretty-printed as a pattern (see `docs/signature-help.md` for the spec):
    - head params once
    - repeat params for each entered repeat group (numbered: `condition1/value1`, `condition2/value2`, `condition3/value3`, ...)
    - `...`
    - tail params once
    - Example: `ifs(condition1: boolean, value1: number, condition2: boolean, value2: number, ..., default: number) -> number`

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
- Note: some unit tests import the generated WASM glue from `examples/vite/src/pkg/`.
  Ensure `pnpm -C examples/vite wasm:build` has been run at least once (or otherwise provide
  `examples/vite/src/pkg/analyzer_wasm.js`) before running `pnpm -C examples/vite test`.

Regression coverage:

- token highlighting
- diagnostics propagation
- chip spans
- UI behavior
- editor undo/redo keybindings
- editor auto height growth
- completion cursor placement (including UTF-16 text)
- completion list scroll-into-view behavior

### Vite demo architecture (current)

The example app keeps analyzer-facing behavior in a thin WASM client and leaves the rest as UI wiring.

Primary files:

- `examples/vite/src/analyzer/wasm_client.ts`:
  - **only** place that imports `examples/vite/src/pkg/analyzer_wasm.js`
  - raw exports: `initWasm`, `analyzeSource`, `completeSource`, `posToLineCol`
  - thin typed wrappers:
    - `safeBuildCompletionState(...)`
    - `applyCompletionItem(...)`
- `examples/vite/src/vm/app_vm.ts`:
  - debounced analyze loop (`DEBOUNCE_MS = 80`) for `FORMULA_IDS = ["f1", "f2"]`
- `examples/vite/src/ui/formula_panel_view.ts`:
  - panel orchestration, CodeMirror setup, completion rendering, active-panel visibility, and debug bridge wiring
- `examples/vite/src/model/diagnostics.ts`:
  - shared diagnostics helpers (Analyzer -> CodeMirror diagnostics, chip-range merge, diagnostics rows)
- `examples/vite/src/model/completions.ts`:
  - completion row planning + selection helpers (flat row shape with flags)
- `examples/vite/src/model/signature.ts`:
  - signature token planning (flat token shape) + popover side/wrap decisions
- `examples/vite/src/ui/signature_popover.ts`:
  - signature help popover rendering from model-provided flat tokens

UI behavior that remains intentionally TypeScript-side:

- Completion list rendering under the “Completions” panel.
- Completion items grouped by contiguous `kind` labels.
- Recommended section controlled by analyzer-provided `preferred_indices`.
- Completion/signature UI is shown for the focused formula panel and hidden for other panels.
- Keyboard navigation across item rows only (headers are skipped).
- Selected completion rows are scrolled into view (`nearest`) after keyboard/mouse selection updates.
- Signature help uses analyzer-provided display segments via model-planned flat tokens (UI does not parse type strings).
- Analyzer diagnostics are mirrored into CodeMirror lint diagnostics for in-editor lint ranges/tooltips.
- Formula editor auto-grows with content via `.editor .cm-editor .cm-scroller`.

Editor keybindings / history:

- CodeMirror history is enabled with `history()` and `historyKeymap` from `@codemirror/commands`.

Cursor placement invariants:

- Analyzer core (`analyzer/`) computes completion cursors as **byte offsets**.
- WASM (`analyzer_wasm`) converts completion edit ranges/cursors to **UTF-16**.
- Vite completion application uses analyzer cursor when present; fallback cursor computation
  (including pre-primary additional-edit offset) lives in `examples/vite/src/analyzer/wasm_client.ts`.

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
- Vite demo UI modules do not import WASM glue directly; `examples/vite/src/analyzer/wasm_client.ts` is the only JS/WASM boundary

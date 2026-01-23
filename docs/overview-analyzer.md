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

## `analyzer` module map

Entry points:

- `analyzer/src/lib.rs`: `analyze(text)`, `analyze_with_context(text, ctx)`, `complete_with_context(text, cursor_byte, ctx)`

Core modules:

- `analyzer/src/lexer/mod.rs`: lexer producing `Token`s + lex diagnostics (strings, numbers, comments, operators)
- `analyzer/src/parser/mod.rs`: Pratt parser plumbing + binding power tables; builds `ParseOutput { expr, diagnostics, tokens }`
- `analyzer/src/parser/expr.rs`: expression grammar (primary/prefix/infix/ternary/call/member-call) + recovery
- `analyzer/src/ast.rs`: AST node types (preserves explicit grouping via `ExprKind::Group`)
- `analyzer/src/format.rs`: formatter for `Expr` using tokens/source for comment attachment; enforces width/indent rules
- `analyzer/src/diagnostics.rs`: `Diagnostic` model + stable `format_diagnostics(...)` output (sorted by span/message)
- `analyzer/src/semantic/mod.rs`: minimal type checking driven by `Context { properties, functions }`
- `analyzer/src/completion.rs`: byte-offset completion + signature help for editor integrations
- `analyzer/src/source_map.rs`: byte offset → (line,col) and byte offset → UTF-16 helpers
- `analyzer/src/token.rs`: token kinds, `Span` (byte offsets), trivia classification
- `analyzer/src/tokenstream.rs`: `TokenCursor` wrapper used by the parser

## Implemented syntax (what the lexer/parser accept)

Literals and identifiers:

- numbers: integer digits only (`analyzer/src/lexer/mod.rs`)
- strings: double-quoted, no escapes (“v1” behavior; `analyzer/src/lexer/mod.rs`)
- identifiers: ASCII letters/`_` plus any non-ASCII codepoint (`analyzer/src/lexer/mod.rs`)

Expression forms (AST):

- unary: `!expr`, `-expr` (`analyzer/src/parser/expr.rs`, `analyzer/src/ast.rs`)
- binary: `< <= == != >= > && || + - * / % ^` (precedence in `analyzer/src/parser/mod.rs`)
- ternary: `cond ? then : otherwise` (`analyzer/src/parser/expr.rs`)
- grouping: `(expr)` preserved as `ExprKind::Group` (`analyzer/src/parser/expr.rs`, `analyzer/src/ast.rs`)
- calls: `ident(arg1, arg2, ...)` (callee must be an identifier; `analyzer/src/parser/expr.rs`)
- member-call: `receiver.method(arg1, ...)` (member *access* without `(...)` is rejected; `analyzer/src/parser/expr.rs`)

Known gaps (explicit in code behavior):

- lexer does not produce boolean literal tokens yet (`true` / `false` lex as identifiers; see note in `analyzer/src/parser/expr.rs`)
- completion’s “after-atom operators” list does not include every parsed operator (`analyzer/src/completion.rs`)

## Builtins + sugar (where they live)

Semantic builtins and checks:

- builtin function list: `analyzer::semantic::builtins_functions()` currently returns `if` and `sum` (`analyzer/src/semantic/mod.rs`)
- semantic types include `Boolean`, and `if()` checks its condition is boolean (`analyzer/src/semantic/mod.rs`)
- property lookup: `prop("Name")` is a special-cased call that checks:
  - arity = 1
  - argument is a string literal
  - property exists in `Context.properties` (`analyzer/src/semantic/mod.rs`)
- postfix sugar typing: `condition.if(then, else)` (2 args) is treated like `if(condition, then, else)` for typing/diagnostics only (no evaluator/runtime desugaring yet)

Completion builtins and sugar:

- start-of-expression “builtins” inserted as identifiers: `not`, `true`, `false` (`analyzer/src/completion.rs`)
- postfix completion: `.if()` is offered after an atom when `if` exists in `Context.functions` (`analyzer/src/completion.rs`)

## WASM boundary invariants (`analyzer_wasm`)

- exported functions: `analyze(source: String, context_json?: String)` and `complete(source: String, cursor_utf16: usize, context_json?: String)` (`analyzer_wasm/src/lib.rs`)
- internal spans are byte offsets (`analyzer/src/token.rs`), but JS-facing spans use UTF-16 offsets:
  - `SpanView { start, end, line, col }` uses UTF-16 `start/end` (`analyzer_wasm/src/dto/v1.rs`, `analyzer_wasm/src/lib.rs`)
  - `complete(...)` accepts a UTF-16 cursor and converts to a byte offset before calling Rust completion (`analyzer_wasm/src/lib.rs`)
- span semantics: all ranges are half-open `[start, end)` (end is exclusive; token hi is exclusive; byte offsets in Rust; UTF-16 offsets in JS DTOs)
- text edits are applied in descending `(start,end)` order to avoid offset shifting when multiple edits exist (`analyzer_wasm/src/lib.rs`)

## Tests (what exists and where)

Rust unit tests:

- `analyzer/src/tests/`: unit tests for lexer/parser/formatter/semantic/utf16/completion (`analyzer/src/tests/mod.rs`)
- completion DSL: `analyzer/src/tests/completion_dsl.rs` provides:
  - a mini `Context` builder
  - `$0` cursor markers in input strings
  - assertion helpers used by `analyzer/src/tests/test_completion.rs`

Rust golden tests (fixtures):

- runners:
  - `analyzer/tests/format_golden.rs` → `analyzer/tests/format/*.formula` → `*.snap`
  - `analyzer/tests/diagnostics_golden.rs` → `analyzer/tests/diagnostics/*.formula` → `*.snap`
- harness: `analyzer/tests/common/golden.rs`
- update snapshots: run tests with `BLESS=1` (checked by `analyzer/tests/common/golden.rs`)

Vite demo tests (TS):

- unit: `examples/vite/tests/unit/` (Vitest)
- e2e: `examples/vite/tests/e2e/` (Playwright; regression checks include token highlighting / diagnostics propagation)

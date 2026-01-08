# notion-formula-rs

A Rust-based implementation of the Notion Formula language, designed as a high-performance, embeddable engine with a WASM-friendly analyzer for interactive formula editors.

This repository is structured as a compiler-like toolchain:

- **analyzer**: lexer → parser → AST → diagnostics → formatter (+ basic semantic checks)
- **evaluator**: runtime execution engine (present, but not the current focus)

A Vite + CodeMirror 6 demo is included to showcase live analysis, formatting, token highlighting, and diagnostics via WASM.

---

## Project Goals

- Provide a clean, modular, strongly-typed implementation of a Notion-like formula language in Rust.
- Make the analyzer **fast**, **embeddable**, and **WASM-friendly** for editor integrations.
- Produce **high-quality diagnostics** (span-based, rustc-like) suitable for IDE/editor UX.
- Keep analyzer (frontend) and evaluator (backend) cleanly separated so hosts can:
  - reuse parsing/formatting/diagnostics without evaluation
  - plug in their own data model and function implementations for evaluation

---

## Status

### Implemented

- Analyzer frontend:
  - Lexing with spans
  - Pratt-style expression parsing
  - Span ↔ line/col mapping (WASM uses UTF-16 offsets)
  - Diagnostics collection (multi-error)
  - Formatter / pretty printer (width-aware, stable output)
  - **Basic semantic diagnostics with context**:
    - `prop("Name")` property existence + argument checks
    - `if(cond, a, b)` arity + boolean condition check
    - `sum(...)` arity + number-argument checks
- WASM bindings:
  - `analyze(source, contextJson?)` for syntax + semantic diagnostics (omit/empty to skip context)
- Web demo (Vite + CodeMirror 6):
  - live analysis (debounced)
  - token highlighting using CodeMirror decorations
  - diagnostics list + editor lints
  - formatted output rendering
  - utility code for chip span/mapping experiments (UI replacement is not implemented)

### Not yet implemented (planned)

- A full rust-analyzer-style `AnalysisHost` API
- Richer type inference and more builtin functions/operators
- A production-grade evaluator with pluggable host context
- Notion-like formula editor UX (chips/widgets, advanced cursor mapping)

---

## Repository Layout

- `analyzer/`
  The frontend: lexing, parsing, AST, diagnostics, formatting, semantic pass.

- `analyzer_wasm/`
  WASM bindings for the analyzer (exports `analyze`).

- `evaluator/`
  Runtime backend (present but currently secondary to analyzer work).

- `examples/vite/`
  Vite + CodeMirror 6 web demo that calls the WASM analyzer.

---

## Quick Start

### Prerequisites

- Rust toolchain (stable)
- Node.js (for the Vite demo)

---

## Build & Test (Rust)

From the repository root:

```bash
cargo test
```

This runs:

- analyzer unit tests (lexer/parser/pretty/utf16/etc.)
- golden snapshot tests for diagnostics and formatting

## Updating golden snapshots

The analyzer uses a custom golden test harness (not insta). To update snapshots after intentional changes:

```bash
BLESS=1 cargo test
```

## Design Notes

### Spans and UTF-16 correctness

- Internally, the analyzer uses byte offsets for spans (Rust-native).
- The WASM layer converts spans to UTF-16 offsets for browser/editor interop.

- Diagnostics and tokens returned to JS use UTF-16 offsets plus line/col.

### Parentheses preservation

- The AST preserves explicit grouping using Group { inner } rather than flattening parentheses. This is intentional for formatting and editor tooling.

### Formatting strategy

The formatter aims for stable, readable output:

- prefers single-line output when within max width (currently 80)
- breaks into multi-line formatting only when needed

### Roadmap (high level)

- Expand semantic analysis:
  - richer operator typing and inference
  - more builtin functions and overloads
  - better unknown propagation and error recovery
- Introduce an editor-oriented AnalysisHost:
  - incremental updates
  - cursor-aware queries (token/expr at position)
  - stable IDs for nodes and diagnostics
- Build evaluator:
  - value system
  - function registry
  - host context provider for properties/relations/rollups
  - interpreter (and later optimization opportunities)

### Contributing

Contributions are welcome, especially in these areas:

- semantic analysis rules and tests
- diagnostics UX improvements and recovery
- formatter idempotence and stability
- web demo ergonomics (CodeMirror integration)
- evaluator design and implementation

### Development principles

Prefer small, incremental changes with strong tests.

- Keep analyzer and evaluator separated.
- Avoid breaking the WASM API without a strong reason.

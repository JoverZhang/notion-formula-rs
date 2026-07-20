# Testing inventory

Where regression coverage lives, what each layer validates, and how to refresh snapshots.

## builtin_fn

- Locations: `builtin_fn/tests/`, `builtin_fn/tests/ui/`, and focused inline tests
- Coverage:
  - function-like category DSL compile-pass and compile-fail diagnostics/recovery
  - fixed, repeat, head + repeat, repeat + tail, and synthetic head + repeat + tail shapes
  - complete catalog order, cross-category uniqueness, support status, and resolver placement
  - deterministic README marked-region rendering checked byte-for-byte
  - shared partial-call projection, generic binding, staged lambda inference, argument
    compatibility, and `flat` return refinement

Run:

```bash
cargo test -p builtin_fn
cargo run -p builtin_fn --bin builtin_catalog -- --check
```

## analyzer

### Unit tests

- Location: `analyzer/src/tests/`
- Coverage:
  - lexer/parser/AST behavior
  - parser recovery + diagnostics priority/deconfliction
  - diagnostic actions (quick-fix actions attached to diagnostics)
  - span/token invariants (`Span`, `tokens_in_span`, `TokenQuery`)
  - semantic checks and builtin/type behavior
  - final `ResolvedFunctionSig` records retained in `SemanticMap` for downstream consumers

### Golden tests (diagnostics)

- Runner: `analyzer/tests/diagnostics_golden.rs`
- Fixtures: `analyzer/tests/diagnostics/*.formula` -> `*.snap`
- Update snapshots:

```bash
BLESS=1 cargo test -p analyzer
```

Run:

```bash
cargo test -p analyzer
```

## ide

### Unit tests

- Location: `ide/src/tests/`
- Coverage:
  - formatter behavior + idempotence
  - completion ranking/position behavior
  - signature-help behavior
  - shared incomplete-call projection and resolver output, with IDE-only postfix/presentation
    adaptation
  - edit application/validation behavior

### Golden tests (format)

- Runner: `ide/tests/format_golden.rs`
- Fixtures: `ide/tests/format/*.formula` -> `*.snap`
- Update snapshots:

```bash
BLESS=1 cargo test -p ide format_golden
```

Run:

```bash
cargo test -p ide
```

## analyzer_wasm

- Location: `analyzer_wasm/tests/analyze.rs`
- Coverage:
  - UTF-16 span/offset correctness (including emoji edge cases)
  - diagnostics + diagnostic action conversion
  - line/column projection on diagnostic DTOs
  - `format(source, cursor)` success/failure contract
  - `apply_edits(source, edits, cursor)` validation and cursor rebasing
  - strict `AnalyzerConfig` constructor validation

Run:

```bash
cargo test -p analyzer_wasm
wasm-pack test --node analyzer_wasm
```

Note: `cargo test -p analyzer_wasm` alone does not execute `wasm_bindgen_test` integration tests.

## evaluator

- Locations: `evaluator/src/` unit tests and `evaluator/tests/` integration tests
- Generated-contract coverage: deterministic full-catalog bindings, all five parameter
  layouts, no generated lifetimes/dynamic context or dynamic Value-argument erasure, and compile
  failures for a missing impl or incorrect method signature
- Runtime-structure coverage: required-column manifests, all five `InputContractError`
  classes, independent mask/ok/validity, shared fan-out, unique storage recovery, debug
  contracts, and non-builtin operator execution
- Runtime-behavior coverage: the bounded public matrix (`flat`, `concat`, `splice`, and `ifs`),
  frozen runtime/row IDs, and focused cases for observed regressions

Run:

```bash
cargo test -p evaluator
```

## Vite demo (examples/vite/)

### Unit tests (Vitest)

- Location: `examples/vite/tests/unit/`
- Precondition: run `pnpm -C examples/vite wasm:build` at least once.

### E2E tests (Playwright)

- Location: `examples/vite/tests/e2e/`

Regression coverage (non-exhaustive):

- token highlighting and diagnostics propagation
- chip spans/mapping and chip UI ranges
- undo/redo editor keybindings
- editor auto-height behavior
- completion preferred indices + grouped rows
- completion cursor placement (including UTF-16 content)
- completion list scroll-into-view behavior
- quick-fix action extraction and first-fix-per-click application

Run:

```bash
pnpm -C examples/vite wasm:build
pnpm -C examples/vite test
pnpm -C examples/vite test:e2e
```

## All-crate shortcuts

```bash
just test        # repo test suite
just verify      # deps + static checks + tests
just docker-test # run verify inside the CI Docker image
just check       # format check + Rust/frontend static checks
cargo test       # workspace Rust tests
```

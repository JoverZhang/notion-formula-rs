# Testing inventory

Where regression coverage lives, what each layer validates, and how to refresh snapshots.

## builtin_fn

- Location: `builtin_fn/src/` (inline tests) + `builtin_fn/src/parser/` (parser tests)
- Coverage: signature parsing, param shape validation, type model, union normalization

Run:

```bash
cargo test -p builtin_fn
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

- Location: `evaluator/src/` (inline tests)
- Coverage: literal evaluation, binary arithmetic, type coercion, divide-by-zero, mask propagation

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

# Testing inventory

## Rust unit tests (`analyzer/`)

- Location: `analyzer/src/tests/`
- Coverage includes:
  - lexer/parser/AST
  - diagnostics and parser recovery
  - diagnostic actions (quick-fix actions attached to diagnostics)
  - span invariants
  - formatter
  - completion/signature-help behavior

Run:

```bash
cargo test -p analyzer
```

## Rust golden tests (`analyzer/`)

Runners:
- `analyzer/tests/format_golden.rs`
- `analyzer/tests/diagnostics_golden.rs`

Update snapshots:

```bash
BLESS=1 cargo test -p analyzer
```

## WASM tests (`analyzer_wasm/`)

- `analyzer_wasm/tests/analyze.rs`
- Validates:
  - UTF-16 span correctness
  - diagnostics + diagnostic action conversion
  - `format(source, cursor)` success/failure contract
  - `apply_edits(source, edits, cursor)` validation and cursor rebasing
  - UTF-16 conversion edge cases (emoji)
  - context JSON validation

Run:

```bash
cargo test -p analyzer_wasm
wasm-pack test --node analyzer_wasm
```

Note: `cargo test -p analyzer_wasm` alone does not execute `wasm_bindgen_test`
integration tests under `analyzer_wasm/tests/`.

## Vite demo tests (`examples/vite/`)

- Unit tests: `examples/vite/tests/unit/`
- E2E tests: `examples/vite/tests/e2e/`

Run:

```bash
pnpm -C examples/vite wasm:build
pnpm -C examples/vite test
pnpm -C examples/vite test:e2e
```

# Testing inventory

Where regression coverage lives, what each layer validates, and how to refresh snapshots.

## Rust unit tests (`analyzer/`)

- Location: `analyzer/src/tests/`
- Coverage includes:
  - lexer/parser/AST behavior
  - parser recovery + diagnostics priority/deconfliction
  - diagnostic actions (quick-fix actions attached to diagnostics)
  - span/token invariants (`Span`, `tokens_in_span`, `TokenQuery`)
  - formatter behavior
  - completion ranking + signature-help behavior
  - semantic checks and builtin/type behavior

Run:

```bash
cargo test -p analyzer
```

## Rust golden tests (`analyzer/`)

Runners:

- `analyzer/tests/format_golden.rs`
- `analyzer/tests/diagnostics_golden.rs`

Fixtures:

- `analyzer/tests/format/*.formula` -> `*.snap`
- `analyzer/tests/diagnostics/*.formula` -> `*.snap`

Update snapshots:

```bash
BLESS=1 cargo test -p analyzer
```

## WASM tests (`analyzer_wasm/`)

- `analyzer_wasm/tests/analyze.rs`
- Validates:
  - UTF-16 span/offset correctness (including emoji edge cases)
  - diagnostics + diagnostic action conversion
  - line/column projection on diagnostic DTOs
  - `ide_format(source, cursor)` success/failure contract
  - `ide_apply_edits(source, edits, cursor)` validation and cursor rebasing
  - strict `AnalyzerConfig` constructor validation

Run:

```bash
cargo test -p analyzer_wasm
wasm-pack test --node analyzer_wasm
```

Note: `cargo test -p analyzer_wasm` alone does not execute `wasm_bindgen_test`
integration tests under `analyzer_wasm/tests/`.

## Vite demo tests (`examples/vite/`)

- Unit tests: `examples/vite/tests/unit/` (Vitest)
- E2E tests: `examples/vite/tests/e2e/` (Playwright)

Precondition:

- Unit tests import the generated wasm package under `examples/vite/src/pkg/`.
- Run `pnpm -C examples/vite wasm:build` at least once before `pnpm -C examples/vite test`.

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

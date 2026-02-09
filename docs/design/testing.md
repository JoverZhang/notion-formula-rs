# Testing inventory

Where the current regression coverage lives, and how snapshots are updated.

## Rust unit tests (`analyzer/`)

- Location: `analyzer/src/tests/`
- Coverage includes:
  - lexer
  - parser
  - span invariants
  - `tokens_in_span`
  - `TokenQuery`
  - formatter
  - completion DSL
  - semantic checks

Run:

```bash
cargo test -p analyzer
```

## Rust golden tests (`analyzer/`)

Runners:

- `analyzer/tests/format_golden.rs`
- `analyzer/tests/diagnostics_golden.rs`

Fixtures:

- `analyzer/tests/format/*.formula` → `*.snap`
- `analyzer/tests/diagnostics/*.formula` → `*.snap`

Update snapshots:

```bash
BLESS=1 cargo test -p analyzer
```

## WASM tests (`analyzer_wasm/`)

- `analyzer_wasm/tests/analyze.rs`
- Validates:
  - UTF-16 span correctness
  - token span integrity
  - diagnostics mapping
  - `context_json` validation rules (non-empty JSON; unknown fields rejected)

Run:

```bash
cargo test -p analyzer_wasm
```

## Vite demo tests (`examples/vite/`)

- Unit tests: `examples/vite/tests/unit/` (Vitest)
- E2E tests: `examples/vite/tests/e2e/` (Playwright)

Note:

- Some unit tests import the generated WASM glue from `examples/vite/src/pkg/`.
- Ensure `pnpm -C examples/vite wasm:build` has been run at least once (or provide
  `examples/vite/src/pkg/analyzer_wasm.js`) before running `pnpm -C examples/vite test`.

Regression coverage (non-exhaustive; grep the suites for details):

- token highlighting
- diagnostics propagation
- chip spans / mapping
- editor undo/redo keybindings
- editor auto height growth
- completion cursor placement (including UTF-16 text)
- completion list scroll-into-view behavior


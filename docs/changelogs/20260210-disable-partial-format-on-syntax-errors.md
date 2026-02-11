# 20260210-disable-partial-format-on-syntax-errors

- Type: Fixed
- Component: analyzer_wasm

## Summary

Formatting behavior was tightened:

- syntax-invalid inputs no longer produce partial formatter output
- syntax-valid formatting remains available
- this laid groundwork for strict edit application and cursor rebasing

## Compatibility notes

- formatting on syntax errors is treated as failure instead of returning partial text
- this behavior is now part of the strict `format(..., cursor)` contract

## Tests

- `cargo test -p analyzer_wasm`
- `cargo test -p analyzer`

## Links

- `docs/design/wasm-boundary.md`
- `docs/design/demo-vite.md`

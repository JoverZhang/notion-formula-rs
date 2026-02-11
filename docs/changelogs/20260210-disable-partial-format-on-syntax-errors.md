# 20260210-disable-partial-format-on-syntax-errors

- Type: Fixed
- Component: analyzer_wasm

## Summary

- `analyze()` no longer returns formatter output for syntax-invalid input.
- `AnalyzeResult.formatted` is now `""` whenever lex/parse diagnostics exist.
- `AnalyzeResult` now includes `quick_fixes` with structured UTF-16 edits for syntax recovery
  actions (insert/replace delimiters, comma insertion/removal).
- Quick-fix derivation now lives in core IDE helpers (`analyzer/src/ide/quick_fix.rs`); WASM only
  converts byte-based quick fixes to UTF-16 DTO edits.
- Vite demo adds a `Quick Fix` button next to `Format`, enabled only when quick fixes are available
  and applying one fix per click.

## Compatibility notes

- DTO shape changed:
  - `AnalyzeResult` adds `quick_fixes`.
- Behavioral contract changed for `AnalyzeResult.formatted`:
  - before: could contain parser-recovery rewrites on syntax errors
  - now: formatting is syntax-valid only; syntax recovery is explicit quick-fix data

## Tests

- `cargo test -p analyzer_wasm`
- `cargo test -p analyzer`

## Links

- `docs/design/wasm-boundary.md`
- `docs/design/demo-vite.md`

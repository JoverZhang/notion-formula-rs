# 20260213-wasm-stateful-analyzer-config-and-offset-renames

- Type: Changed
- Component: analyzer_wasm, examples/vite, docs

## Summary

Made the WASM API instance-based (`new Analyzer(config)`) and removed function-style exports that
accepted `context_json` strings. Config is now object input (`AnalyzerConfig`) with top-level
`preferred_limit` support (`null` uses default `5`).

Also centralized and renamed UTF conversion helpers in `analyzer_wasm/src/offsets.rs`:

- `utf16_to_8_offset`
- `utf8_to_16_offset`
- `utf16_to_8_cursor`
- `utf16_to_8_text_edits`

## Compatibility notes

- Breaking (WASM API):
  - Removed: `analyze(source, context_json)`
  - Removed: `ide_help(source, cursor_utf16, context_json)`
  - Now: instantiate `Analyzer` once and call instance methods.
- Breaking (WASM config shape):
  - Removed nested `completion.preferred_limit`.
  - Now use `AnalyzerConfig.preferred_limit`.
- Breaking (demo integration):
  - `examples/vite` now initializes the wrapper with `initWasm(ANALYZER_CONFIG)` and uses a
    stateful `Analyzer` instance.

## Tests

- `cargo test -p analyzer_wasm`
- `cargo run -p analyzer_wasm --bin export_ts`
- `pnpm -C examples/vite -s run wasm:build`
- `pnpm -C examples/vite -s run test -- tests/unit/signature_help_instantiated.test.ts tests/unit/wasm_errors.test.ts`

## Links

- `analyzer_wasm/src/lib.rs`
- `analyzer_wasm/src/offsets.rs`
- `analyzer_wasm/src/dto/v1.rs`
- `analyzer_wasm/tests/analyze.rs`
- `examples/vite/src/analyzer/wasm_client.ts`

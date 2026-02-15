# 20260215-wasm-api-remove-ide-prefix

- Type: Changed
- Component: analyzer_wasm, examples/vite, docs

## Summary

Renamed stateful WASM `Analyzer` methods from `ide_*` to prefix-free names: `format`, `apply_edits`, and `help`. Updated the Vite demo wrapper so proxy function names match the underlying WASM method names directly.

## Compatibility notes

- Breaking (WASM API):
  - Renamed `Analyzer.ide_format(...)` -> `Analyzer.format(...)`
  - Renamed `Analyzer.ide_apply_edits(...)` -> `Analyzer.apply_edits(...)`
  - Renamed `Analyzer.ide_help(...)` -> `Analyzer.help(...)`
- Breaking (demo wrapper API):
  - Renamed `analyzeSource` -> `analyze`
  - Renamed `formatSource` -> `format`
  - Renamed `applyEditsSource` -> `apply_edits`
  - Renamed `helpSource` -> `help`

## Tests

- `pnpm -C examples/vite wasm:build`
- `cargo test -p analyzer_wasm`
- `wasm-pack test --node analyzer_wasm`
- `pnpm -C examples/vite -s run test -- tests/unit/wasm_errors.test.ts tests/unit/signature_help_instantiated.test.ts tests/unit/completion_preferred_indices.test.ts`
- `pnpm -C examples/vite -s run check`

## Links

- `analyzer_wasm/src/lib.rs`
- `analyzer_wasm/tests/analyze.rs`
- `examples/vite/src/analyzer/wasm_client.ts`
- `examples/vite/src/vm/app_vm.ts`
- `examples/vite/src/ui/formula_panel_view.ts`

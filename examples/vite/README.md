# Vite demo (WASM)

- Run dev server: `pnpm dev`
- Rebuild WASM after Rust changes: `pnpm wasm:build`

## Integration notes

- WASM wrapper: `src/analyzer/wasm_client.ts`
  - `analyzeSource`
  - `formatSource`
  - `applyEditsSource`
  - `helpSource`
- wasm-pack output: `src/pkg/`
- DTO types: `src/analyzer/generated/wasm_dto.ts`

## Architecture

Primary files:

- `src/analyzer/wasm_client.ts`
- `src/vm/app_vm.ts`
- `src/ui/formula_panel_view.ts`
- `src/model/diagnostics.ts`
- `src/model/completions.ts`
- `src/model/signature.ts`
- `src/ui/signature_popover.ts`

## UI behavior

- Action row shows `Format`, `Quick Fix`, and `output: <type>`.
- `Format` calls `formatSource(currentSource, currentCursorUtf16)` and applies returned
  `{ source, cursor }`.
- `Quick Fix` is derived from `AnalyzeResult.diagnostics[].actions`.
  - First available action is used.
  - Apply uses `applyEditsSource(currentSource, action.edits, currentCursorUtf16)`.
- Completion/signature help is TypeScript-rendered from structured WASM payloads.

## Debug bridge

Available as `window.__nf_debug` in dev/test mode for panel state and diagnostics inspection.

## Tests

```bash
pnpm test
pnpm test:e2e
```

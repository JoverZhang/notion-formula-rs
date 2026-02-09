# Vite demo (WASM)

- Run the dev server: `pnpm dev`
- Build WASM (if you change Rust/WASM): `pnpm wasm:build`

## Integration notes

- WASM client wrapper: `src/analyzer/wasm_client.ts` (`analyzeSource`, `completeSource`, `posToLineCol`).
- wasm-pack output (JS glue + `.wasm`): `src/pkg/` (produced by `pnpm wasm:build`).
- DTO TS types:
  - Generated: `src/analyzer/generated/wasm_dto.ts`
  - Generator: `cargo run -p analyzer_wasm --bin export_ts` (or `just gen-ts` from repo root)

## Debug bridge

- Enable via any of:
  - `?debug=1` in the URL
  - `import.meta.env.DEV`
  - `import.meta.env.MODE === "test"`
- Accessible as `window.__nf_debug` with helpers to inspect panel state (sources, diagnostics, token decorations, chip spans/mapping, CM diagnostics, selection).
- The bridge is wired from the panel/controller data, not the DOM, so it is stable for tests.

## E2E (Playwright)

- Install deps (from this directory): `pnpm install`
- Run headless suite: `pnpm test:e2e`
- Run with UI viewer: `pnpm test:e2e:ui`
- The suite boots Vite on `127.0.0.1:5173/?debug=1` and asserts:
  - Debug bridge presence and panel registration
  - Token highlighting regression (no “first token only”)
  - Analyzer diagnostics flowing into the UI + CodeMirror lint
  - Chip span detection + mapping (UI chips are intentionally not rendered)
- A screenshot baseline is not enabled yet; add one later if the page can be made fully deterministic.

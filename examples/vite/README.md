# Vite demo (WASM)

- Run the dev server: `pnpm dev`
- Build WASM (if you change Rust/WASM): `pnpm wasm:build`

## Integration notes

- WASM client wrapper: `src/analyzer/wasm_client.ts` (`analyzeSource`, `completeSource`, `posToLineCol`).
- wasm-pack output (JS glue + `.wasm`): `src/pkg/` (produced by `pnpm wasm:build`).
- DTO TS types:
  - Generated: `src/analyzer/generated/wasm_dto.ts`
  - Generator: `cargo run -p analyzer_wasm --bin export_ts` (or `just gen-ts` from repo root)

## Architecture

The demo keeps the JS/WASM boundary in one place and leaves most UI behavior TypeScript-side.

Primary files:

- `src/analyzer/wasm_client.ts`:
  - Only place that imports `src/pkg/analyzer_wasm.js`
  - Exports: `initWasm`, `analyzeSource`, `completeSource`, `posToLineCol`
  - Helpers: `safeBuildCompletionState(...)`, `applyCompletionItem(...)`
- `src/vm/app_vm.ts`:
  - Debounced analyze loop (`DEBOUNCE_MS = 80`) for `FORMULA_IDS = ["f1", "f2"]`
- `src/ui/formula_panel_view.ts`:
  - Panel orchestration, CodeMirror wiring, completion rendering, focused-panel visibility, debug bridge wiring
- `src/model/diagnostics.ts`:
  - Analyzer → CodeMirror diagnostics, chip-range merge, diagnostics rows
- `src/model/completions.ts`:
  - Completion row planning + selection helpers
- `src/model/signature.ts`:
  - Signature token planning + popover layout decisions
- `src/ui/signature_popover.ts`:
  - Signature help popover rendering from model-provided flat tokens

UI behavior that is intentionally TypeScript-owned:

- Completions render under the “Completions” panel.
- Items are grouped by contiguous kind headers, with a “Recommended” section derived from `preferred_indices`.
- Keyboard navigation skips headers; selection is scrolled into view.
- Signature help renders analyzer-provided display segments; the UI does not parse type strings.
- Analyzer diagnostics are mirrored into CodeMirror lint diagnostics.
- Formula editor auto-grows with content via `.editor .cm-editor .cm-scroller`.

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
  - Chip span detection + mapping (and chip UI rendering/interaction)
- A screenshot baseline is not enabled yet; add one later if the page can be made fully deterministic.

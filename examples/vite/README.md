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

- Completions render in the “Completions” panel inside the editor wrap (under the action row).
- Suggestion popover can render both signature help and diagnostics in one surface
  (`src/ui/signature_popover.ts`).
- Items are grouped by contiguous kind headers, with a “Recommended” section derived from `preferred_indices`.
- Keyboard navigation skips headers; selection is scrolled into view.
- The editor action row shows `Format`, `Quick Fix`, and `output: <type>`.
  - `Format` applies `AnalyzeResult.formatted` (available only when syntax is valid).
  - `Quick Fix` applies one analyzer-provided fix per click (the current first item in
    `AnalyzeResult.quick_fixes`), is enabled only when fixes are available, and exposes the active
    fix title via button hover tooltip.
  - `output: <type>` uses `AnalyzeResult.output_type` (non-null with `"unknown"` fallback),
    right-aligned with overflow truncation.
- Signature help renders analyzer-provided display segments; the UI does not parse type strings.
- Analyzer diagnostics are mirrored into CodeMirror lint diagnostics.
- Formula editor auto-grows with content via `.editor .cm-editor .cm-scroller`.

UI implementation details that are intentionally local to the demo:

- Completion panel keeps a fixed-height body (`.completion-body`) with internal scrolling in
  `.completion-items` to avoid vertical jump/flicker when item counts change.
- Signature popover diagnostics are rendered as
  `ul[data-testid="formula-diagnostics"][data-formula-id="<id>"]` inside the popover container
  so e2e checks can verify diagnostics content without depending on layout position.
- Signature popover wrap fallback checks both signature-main overflow and popover-container
  overflow to avoid clipping in grid/nowrap layout edge cases.
- In wrapped mode, signature main uses preserved newlines (`white-space: pre`) with horizontal
  scrolling for long lines to avoid ambiguous auto-wrapping inside type expressions.
- Signature popover keeps pointer interaction enabled for native horizontal scrolling in the
  signature main area, and it is hidden when the active editor loses focus.

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
- The suite boots Vite on `PW_HOST` (default `127.0.0.1`) and a workspace-derived
  stable high port (or use `PW_PORT` to pin one), with `?debug=1`, and asserts:
  - Debug bridge presence and panel registration
  - Token highlighting regression (no “first token only”)
  - Analyzer diagnostics flowing into the UI + CodeMirror lint
  - Chip span detection + mapping (and chip UI rendering/interaction)
- A screenshot baseline is not enabled yet; add one later if the page can be made fully deterministic.

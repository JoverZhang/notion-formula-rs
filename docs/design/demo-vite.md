# Demo app (`examples/vite`)

Notes on the TypeScript demo consuming `analyzer_wasm`.
This document focuses on behavior constraints and UI-owned invariants.

## Integration points

- WASM wrapper: `examples/vite/src/analyzer/wasm_client.ts`
  - Only place that imports wasm-pack glue (`src/pkg/analyzer_wasm.js`).
- Panel orchestration + CodeMirror wiring: `examples/vite/src/ui/formula_panel_view.ts`
- Shared UI models:
  - diagnostics: `examples/vite/src/model/diagnostics.ts`
  - completions: `examples/vite/src/model/completions.ts`
  - signature help: `examples/vite/src/model/signature.ts`
- Signature popover rendering: `examples/vite/src/ui/signature_popover.ts`

For the full file map, see `examples/vite/README.md`.

## Action row behavior

- `Format`:
  - calls `formatSource(source, cursorUtf16)`
  - applies returned `{ source, cursor }`
  - is a no-op on thrown WASM format errors

- `Quick Fix`:
  - derived from `AnalyzeResult.diagnostics[].actions`
  - uses the first actionable diagnostic action (first-fix-per-click)
  - button tooltip reflects the active action title
  - applies edits via `applyEditsSource(source, action.edits, cursorUtf16)`

- `output: <type>` uses `AnalyzeResult.output_type` and is always present.

## Completion model

- Completion and signature data are requested through `safeBuildCompletionState(...)`
  on a debounce (`120ms`) in the focused editor.
- Rows are TypeScript-owned (`completionRows`):
  - grouped by contiguous `kind`
  - optional `Recommended` section from `preferred_indices`
  - header rows are not selectable
- Keyboard behavior:
  - Arrow keys skip header rows
  - Enter/Tab apply selected item
  - Escape clears selection
- Selection is auto-scrolled into view inside the completion list viewport.

## Suggestion popover behavior

- Signature help and diagnostics share one popover surface.
- Signature text renders directly from analyzer-provided segments; UI does not parse type strings.
- Wrap mode switches from unwrapped to wrapped on overflow checks to avoid clipping in narrow
  layouts.

## Cursor placement invariants

- Completion edits are applied in original-document coordinates and sorted before dispatch.
- If `CompletionItem.cursor` exists, it is used as authoritative cursor-after-edit.
- Fallback cursor is deterministic:
  - `primary_edit` end + net shift from `additional_edits` strictly before `primary_edit`
- Demo code path: `examples/vite/src/analyzer/wasm_client.ts` (`applyCompletionItem`).

## Focus and visibility

- Completion/signature UI is shown only for the currently focused formula panel.
- Focus transfer between panels hides inactive panel suggestion UI.
- Popover hides when editor focus is lost.

## Editor history / keybindings

- Undo/redo history uses `history()` + `historyKeymap` from `@codemirror/commands`.
- Completion navigation/accept/cancel keybindings are wired in
  `examples/vite/src/ui/formula_panel_view.ts`.

## Playwright host config

- Playwright boots preview server via `webServer` in `examples/vite/playwright.config.ts`.
- Host/port overrides:
  - `PW_HOST` (default `127.0.0.1`)
  - `PW_PORT` (optional; when unset, port is derived deterministically from worktree path)

## Tests

See `docs/design/testing.md` and `examples/vite/README.md`.

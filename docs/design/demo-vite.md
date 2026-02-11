# Demo app (`examples/vite`)

Notes on the TypeScript demo consuming `analyzer_wasm`.
This design note focuses on behavior and constraints; local implementation details live in
`examples/vite/README.md`.

## Where the integration lives

- WASM wrapper: `examples/vite/src/analyzer/wasm_client.ts`
  - This is the only place that imports the wasm-pack glue (`examples/vite/src/pkg/analyzer_wasm.js`).
- Panel orchestration + CodeMirror wiring: `examples/vite/src/ui/formula_panel_view.ts`
- Shared UI models:
  - completions: `examples/vite/src/model/completions.ts`
  - diagnostics: `examples/vite/src/model/diagnostics.ts`
  - signature help: `examples/vite/src/model/signature.ts`
- Signature popover: `examples/vite/src/ui/signature_popover.ts`

For the current demo file map, see `examples/vite/README.md` (“Architecture”).

## Completion UI model

The demo renders completions from WASM entirely in TypeScript.

Behavior:

- Completions render in the “Completions” panel inside the editor wrap (under the editor action
  row).
- Suggestion UI uses one popover surface that can show signature help and diagnostics together.
- The editor action row exposes `Format`; the right side shows `output: <type>` from
  `AnalyzeResult.output_type` (non-null, unknown/error = `"unknown"`, right-aligned, truncated on
  overflow).
- Completions are grouped by consecutive `kind` changes (UI-owned grouping).
- Function groups are represented directly by function-specific completion kinds
  (`FunctionGeneral`, `FunctionText`, `FunctionNumber`, `FunctionDate`, `FunctionPeople`,
  `FunctionList`, `FunctionSpecial`).
- A “Recommended” section is derived from analyzer-provided `preferred_indices`.
- Navigation operates over a `completionRows` model (headers + items):
  - headers are not selectable
  - arrow keys skip header rows
- Applying a completion maps the selected row back to the underlying `CompletionItem` index.
- Completion and signature UI is shown for the focused formula panel and hidden when focus leaves
  that editor (or switches to another panel).
- Keyboard navigation keeps the selected completion row visible in the completion list viewport.

## Styling and rendering

- Completion list uses a fixed-height scroll region to prevent panel jump/flicker as result counts
  change.
- Signature and diagnostics content share one popover container for clearer suggestion context.
- Signature help is rendered from analyzer-provided segments (UI does not parse signature/type
  strings).
- Long signature labels keep explicit signature line breaks; overflowing lines are horizontally
  scrollable in the signature box.
- Editor auto-grows with content.

## Editor history / keybindings

- History enabled with `history()` and `historyKeymap` from `@codemirror/commands`.
- Wired in `examples/vite/src/ui/formula_panel_view.ts`.

## Cursor placement invariants

- Core analyzer computes completion cursors as byte offsets.
- `CompletionItem.cursor` (when present) is the desired cursor position in the updated document
  after applying the primary edit.
- WASM converts edit ranges and cursor values to UTF-16 and accounts for shifts from
  `additional_edits` that occur before the primary edit.
- The demo uses `item.cursor` when present; otherwise it falls back to:
  - `primary_edit` end + shifts from `additional_edits` before the primary edit
  - Code: `examples/vite/src/analyzer/wasm_client.ts` (`applyCompletionItem`)

## Playwright host config

- The Playwright suite boots Vite via `webServer` in `examples/vite/playwright.config.ts`.
- Host/port overrides:
  - `PW_HOST` (default `127.0.0.1`)
  - `PW_PORT` (optional; when unset, the config derives a stable high port from the worktree path)

## Tests

See `docs/design/testing.md` and `examples/vite/README.md`.

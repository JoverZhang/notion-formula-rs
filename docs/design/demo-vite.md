# Demo app (`examples/vite`)

Notes on the TypeScript demo consuming `analyzer_wasm`.

## Where the integration lives

- WASM wrapper: `examples/vite/src/analyzer/wasm_client.ts`
- Completion UI: `examples/vite/src/ui/formula_panel_view.ts`

## Completion UI model

The demo renders completions from WASM entirely in TypeScript.

Behavior:

- Completions render under the “Suggestions” panel.
- Function completions are grouped by `category` (UI-owned grouping).
- Non-function completions are grouped by consecutive `kind` changes (UI-owned grouping).
- Navigation operates over a `completionRows` model (headers + items):
  - headers are not selectable
  - arrow keys skip header rows
- Applying a completion maps the selected row back to the underlying `CompletionItem` index.

## Styling and rendering

- Group headers: `.completion-group-header` (`examples/vite/src/style.css`)
- Signature help is rendered from analyzer-provided segments (UI does not parse signature/type
  strings).
- Editor auto-grows (no fixed max-height cap); minimum height via:
  - `.editor .cm-editor .cm-scroller`

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
  - Code: `examples/vite/src/ui/formula_panel_view.ts`

## Playwright host config

- The Playwright suite boots Vite via `webServer` in `examples/vite/playwright.config.ts`.
- Host/port overrides:
  - `PW_HOST` (default `127.0.0.1`)
  - `PW_PORT` (default `5173`)

## Tests

See `docs/design/testing.md` and `examples/vite/README.md`.


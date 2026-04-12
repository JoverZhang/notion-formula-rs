# Demo app (Design) -- `examples/vite`

Behavior constraints and UI-owned invariants for the TypeScript demo consuming `analyzer_wasm`.
For implementation details, see `examples/vite/README.md`.

## Purpose

Live demo of the formula editor: format, quick-fix, completion, signature help, diagnostics.

## Integration pipeline

```
  User input (CodeMirror)
       |
       v
  formula_panel_view.ts ──> orchestration
       |                      (examples/vite/src/ui/)
       v
  wasm_client.ts ──> WASM wrapper
       |              (examples/vite/src/analyzer/)
       v
  Analyzer (WASM) ──> analyze / help / format / apply_edits
       |
       v
  UI models ──> diagnostics, completions, signature
                (examples/vite/src/model/)
```

## Integration points

- WASM wrapper: `examples/vite/src/analyzer/wasm_client.ts` (only imports wasm-pack glue)
- Panel orchestration: `examples/vite/src/ui/formula_panel_view.ts`
- Shared UI models:
  - `examples/vite/src/model/diagnostics.ts`
  - `examples/vite/src/model/completions.ts`
  - `examples/vite/src/model/signature.ts`
- Signature popover: `examples/vite/src/ui/signature_popover.ts`
- Full file map: `examples/vite/README.md`

## Action row behavior

- **Format**: calls `format(source, cursorUtf16)`, applies returned `{ source, cursor }`, no-op on thrown WASM errors.
- **Quick Fix**: uses first actionable diagnostic action (first-fix-per-click), applies edits via `apply_edits(source, action.edits, cursorUtf16)`.
- **Output type**: uses `AnalyzeResult.output_type`, always present.

## Completion model

- Requested through `safeBuildCompletionState(...)` on 120ms debounce in focused editor.
- Rows grouped by contiguous `kind`, optional `Recommended` section from `preferred_indices`.
- Header rows are not selectable.
- Keyboard: arrow keys skip headers, Enter/Tab apply, Escape clears.
- Selection auto-scrolled into view.

## Signature popover

- Signature help and diagnostics share one popover surface.
- Renders directly from analyzer-provided segments; UI does not parse type strings.
- Wrap mode switches on overflow checks.

## Cursor placement

- Completion edits applied in original-document coordinates, sorted before dispatch.
- If `CompletionItem.cursor` exists, used as authoritative cursor-after-edit.
- Fallback: `primary_edit` end + net shift from `additional_edits` strictly before `primary_edit`.
- Code path: `examples/vite/src/analyzer/wasm_client.ts` (`applyCompletionItem`)

## Focus and visibility

- Completion/signature UI shown only for focused formula panel.
- Focus transfer hides inactive panel suggestion UI.
- Popover hides on editor focus loss.

## Editor history / keybindings

- Undo/redo: `history()` + `historyKeymap` from `@codemirror/commands`.
- Completion keybindings wired in `formula_panel_view.ts`.

## Playwright host config

- Boots preview server via `webServer` in `examples/vite/playwright.config.ts`.
- `PW_HOST` (default `127.0.0.1`), `PW_PORT` (optional; derived from worktree path when unset).

## Tests

See `docs/design/testing.md` and `examples/vite/README.md`.

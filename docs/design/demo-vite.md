# Demo app (`examples/vite`)

Notes on the TypeScript demo consuming `analyzer_wasm`.

## Integration points

- WASM wrapper: `examples/vite/src/analyzer/wasm_client.ts`
- Panel/UI orchestration: `examples/vite/src/ui/formula_panel_view.ts`
- Shared UI models:
  - diagnostics: `examples/vite/src/model/diagnostics.ts`
  - completions: `examples/vite/src/model/completions.ts`
  - signature help: `examples/vite/src/model/signature.ts`

## Action row behavior

- `Format`:
  - calls `formatSource(source, cursorUtf16)`
  - applies returned `{ source, cursor }`

- `Quick Fix`:
  - derived from `AnalyzeResult.diagnostics[].actions`
  - shows first action title in button tooltip
  - applies first action with `applyEditsSource(source, action.edits, cursorUtf16)`

- `output: <type>` uses `AnalyzeResult.output_type`.

## Completion model

- Completion rows are rendered in TypeScript from structured DTOs.
- Keyboard navigation skips headers and keeps selected row in view.
- Cursor placement for completion relies on analyzer-provided cursor when present.

## Diagnostics model

- Analyzer diagnostics are mirrored into CodeMirror diagnostics.
- Diagnostic text rows are derived from diagnostic payload + chip mapping.
- Diagnostic text rows include DTO-provided `line:col`.
- Quick-fix actions are consumed from diagnostic-level `actions`.

## Focus and visibility

- Completion/signature UI is shown only for active focused panel.
- Popover hides when editor focus is lost.

## Tests

See `docs/design/testing.md` and `examples/vite/README.md`.

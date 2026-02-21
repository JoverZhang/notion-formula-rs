# Docs

Start at: [`docs/design/README.md`](design/README.md).

## What lives where

- `docs/design/README.md`: repo map + contracts/drift tracker index.
- `docs/design/contracts.md`: design contracts (hard rules).
- `docs/design/drift-tracker.md`: open questions and known gaps.
- `README.md` next to code: module/crate docs (e.g. `analyzer/README.md`).
- `docs/changelogs/YYYYMMDD-short-slug.md`: user-visible changes.

## When you change code

- Update the module README next to the code you touched.
  - Edit in place. Remove stale info. Don’t append forever.
- If a change breaks or updates a contract in `docs/design/README.md`:
  - Add tests.
  - Call it out in the PR/commit message.
- If a change is user-visible (behavior/API/DTO/compat):
  - Add a changelog entry (see below).

## Agent edit policy

Allowed without explicit approval:
- Fix links, add indexes, clarify docs with file pointers.
- Add status notes / TODOs in the drift tracker.

Not allowed without explicit approval:
- Changing contracts (spans/offsets, DTO schemas, determinism rules, etc.).

## Commands (repo-verified)

From repo root:

```bash
just test
just check
just fmt
just gen-ts

cargo test
cargo test -p analyzer
cargo test -p analyzer_wasm
BLESS=1 cargo test -p analyzer
```

Vite demo (from `examples/vite/`):

```bash
pnpm -s run wasm:build
pnpm -s run dev
pnpm -s run test
pnpm -s run test:e2e
pnpm -s run check
```

## Changelog entries

- Guidelines: `docs/changelog_entry_guidelines.md`
- Template: `docs/_description_templates/changelog_entry_template.md`

## Templates

- Module README: `docs/_description_templates/module_README_template.md`
- Design contract: `docs/_description_templates/design_contract_template.md`

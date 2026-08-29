---
doc_id: docs.index
title: "Documentation guide"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-29
---

# Docs

[简体中文](README.zh-CN.md)

Start at: [`docs/design/README.md`](design/README.md).

## What lives where

- `docs/design/README.md`: pipeline overview + design docs index.
- `docs/design/contracts.md`: cross-crate hard rules (spans, tokens, determinism, edits).
- `docs/design/builtin-fn.md`: builtin declaration DSL, call-signature resolution, catalog, and evaluator contracts.
- `docs/design/analyzer.md`: lexer, parser, semantic pipeline (analyzer crate).
- `docs/design/ide.md`: completion, signature help, formatting (ide crate).
- `docs/design/wasm-boundary.md`: WASM facade, DTOs, UTF-16 (analyzer_wasm crate).
- `docs/design/evaluator.md`: IR, planner, kernels, row-batch (evaluator crate).
- `docs/design/testing.md`: test inventory across all crates.
- `docs/design/demo-vite.md`: Vite example app UI/UX.
- `docs/design/drift-tracker.md`: open questions and known gaps.
- `docs/glossary.md`: shared Chinese-English terminology and code identifiers.
- `README.md` next to code: module/crate docs (e.g. `analyzer/README.md`).
- `docs/changelogs/YYYYMMDD-short-slug.md`: user-visible changes.

## Bilingual documentation

English documentation keeps its existing path. A complete Simplified Chinese counterpart
uses the adjacent `.zh-CN.md` suffix:

```text
docs/design/evaluator.md <-> docs/design/evaluator.zh-CN.md
docs/README.md           <-> docs/README.zh-CN.md
```

Existing source-only documents do not need an empty or partial counterpart. Once a pair
exists:

- keep each language as a complete, natural document rather than interleaving translations;
- link both directions near the title and reuse language-neutral diagrams, code identifiers,
  schemas, and assets;
- choose one source language for each editing cycle, verify its technical claims first, and
  then update the counterpart;
- preserve scope, lifecycle status, certainty, guarantees, limits, examples, and failure
  behavior across both versions; and
- use the canonical terms in [`glossary.md`](glossary.md), preserving code identifiers exactly.

The glossary is intentionally one shared bilingual mapping rather than two counterparts:
exact term pairing is its interface, and splitting it would create a second source of truth.

Paired architecture and contract documents use this metadata:

```yaml
---
doc_id: architecture.example
title: "Reader-oriented title"
language: en
source_language: en
counterpart: ./example.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: YYYY-MM-DD
---
```

The counterpart uses the same `doc_id`, sets `language: zh-CN`, and links back to the source.
Use `translation_status: needs-update` only to make known drift explicit; a stable Current
architecture or contract change should normally leave both versions `synced`.

Lifecycle labels have fixed meanings:

- **Current**: verified behavior in the current implementation.
- **Planned**: an accepted direction that is not fully implemented.
- **Exploratory**: an option under discussion without a commitment.
- **Deprecated**: still present but being removed or replaced.
- **Historical**: retained to explain an earlier decision or migration.

Before calling a bilingual document complete, review both versions for technical evidence,
failure behavior, lifecycle status, terminology, and semantic parity. The delivery summary
must identify verified claims, remaining uncertainty, and the resulting translation status.

### Automated checks

Run `just docs-check` to validate the repository's Markdown links and the mechanical parts of
the bilingual contract. For every document with YAML metadata, the command requires the fields
shown above, an adjacent and reciprocal counterpart, matching shared metadata, one `en` side and
one `zh-CN` side, and a body link in each direction.

Accepted metadata values are:

- `implementation_status`: `current`, `planned`, `exploratory`, `deprecated`, or `historical`;
- `document_status`: `draft` or `stable`; and
- `translation_status`: `synced` or `needs-update`.

Source-only documents omit the bilingual metadata until a counterpart exists. The automated
check cannot judge semantic parity: setting `translation_status: synced` still requires a manual
review of technical meaning, certainty, guarantees, limits, and failure behavior in both files.

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
just verify
just deps
just check
just docs-check
just fix
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

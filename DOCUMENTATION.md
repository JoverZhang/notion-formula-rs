---
doc_id: documentation.policy
title: "Documentation policy"
language: en
source_language: en
counterpart: ./DOCUMENTATION.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Documentation policy

[简体中文](DOCUMENTATION.zh-CN.md)

This policy answers where project knowledge belongs and when a change must update it. It applies to human and agent contributors.

## Put each fact at one level

- `docs/intent/` states why the system exists: its audience, goals, non-goals, boundaries, trade-offs, and reevaluation conditions. Keep it short.
- `docs/specs/` states the current, user-observable contract without mirroring the crate layout or preserving design history.
- `docs/how/` explains how the current implementation works. Its directories follow source crate names, with one implementation guide per crate by default.
- `docs/contributing/` explains repository-specific development practices such as testing and changelog writing.
- `docs/changelogs/` records user-visible changes. Root `GLOSSARY.md` is the English-only terminology authority.

Own each fact in one document. Other documents link to that owner instead of restating it. Keep Planned, Exploratory, Deprecated, and Historical claims separate from Current behavior.

## Respect human-controlled documents

`DOCUMENTATION.*`, `docs/intent/`, and `docs/specs/` are human-controlled. Edit them only when the user explicitly asks for the corresponding policy, intent, or contract change. Translation permission allows a meaning-preserving counterpart; it does not allow a technical decision to change.

If a code change requires an unauthorized update in a human-controlled area, stop and ask the user. Treat that missing authorization as a code-review blocker.

Agents may update `docs/how/` and `docs/contributing/` when the implementation or development workflow in their assigned task materially changes.

## Update the layer that owns the change

| Change | Documentation owner |
|---|---|
| Goals, non-goals, system boundaries, or accepted trade-offs | `docs/intent/` |
| User-visible behavior, schemas, errors, ordering, or compatibility | `docs/specs/` |
| Crate structure, algorithms, data flow, internal interfaces, or debugging paths | `docs/how/` |
| Testing or contributor workflow | `docs/contributing/` |

When documented facts did not change, do not make a ceremonial documentation edit.

## Maintain bilingual documents by meaning

Except for categories declared in `docs/manifest.toml`, formal Markdown uses adjacent English `.md` and Simplified Chinese `.zh-CN.md` counterparts. Choose either language as the source for an editing cycle, verify its facts, then write the counterpart as natural prose at the same technical altitude.

Use these translation states:

- `synced`: both counterparts exist and carry the same technical meaning;
- `pending`: the declared counterpart has not been written yet;
- `needs-update`: both files exist, but one is known to lag behind.

`pending` and `needs-update` expose intermediate translation debt. A completed documentation package must return to `synced`. Update `last_verified` only after rechecking technical facts, not after a move, translation, or prose-only edit.

Run `just docs-check` to validate classification, metadata, counterparts, and local links. Semantic parity still requires review.

## Write for a reader

- Start from the question the reader needs answered and use direct, concrete sentences.
- Base technical claims on code, tests, schemas, runtime evidence, or an explicit human decision. Do not infer design intent from implementation structure.
- Put a small number of useful implementation or test anchors beside the claim they support. Keep the verification procedure and command log in the PR.
- Preserve scope, certainty, guarantees, limitations, failure behavior, examples, and lifecycle status across languages while writing naturally in each language.
- Remove template sections, summaries, and history that do not help answer the document's core question.

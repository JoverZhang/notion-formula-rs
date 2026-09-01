---
doc_id: contributing.changelogs
title: "How to write a changelog entry"
language: en
source_language: en
counterpart: ./changelogs.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How to write a changelog entry

[简体中文](changelogs.zh-CN.md)

Add a file under [`docs/changelogs/`](../changelogs/) when a change alters behavior that a user or
integrator can observe. Changelog entries preserve what changed at one point in time; the Specs
describe the system's Current contract.

## Decide whether the change needs an entry

Write an entry for:

- user-visible parser, diagnostic, completion, signature-help, formatting, or evaluation changes;
- public Rust or WASM API and DTO changes;
- compatibility changes to schemas, coordinates, ordering, determinism, or controlled errors;
- fixes whose previous behavior could affect a user's formula or integration.

An entry is optional when a refactor cannot change observable output or a documentation change only
repairs wording and links. Do not use the changelog as a development diary.

## Name and write the pair

Use `YYYYMMDD-short-slug.md` and the adjacent `YYYYMMDD-short-slug.zh-CN.md`. The date is the landing
date; the lowercase, hyphenated slug names the behavior. Start from the paired templates in
[`docs/_templates/`](../_templates/).

Keep one entry focused on one change:

- `Type` is `Added`, `Changed`, `Fixed`, `Removed`, or `Security`.
- `Component` names the user-facing area affected by the change.
- `Summary` states the observable difference.
- `Compatibility notes` identify breaking changes, migrations, or explicitly say there are none.
- `Links` point to durable context such as a PR, issue, or the Current Specs owner.

Use public names that readers can search for. Mention implementation details only when they explain
compatibility. Record commands and review evidence in the PR, not in a new changelog entry.

Historical entries describe behavior at their landing date. Fix broken links or factual mistakes,
but do not rewrite an old entry to match the Current implementation. Keep both language versions at
the same historical scope and mark the pair `synced` only after their meaning agrees.

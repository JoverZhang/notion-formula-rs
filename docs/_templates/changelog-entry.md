---
doc_id: template.changelog-entry
title: "Changelog entry template"
language: en
source_language: en
counterpart: ./changelog-entry.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Changelog entry template

[简体中文](changelog-entry.zh-CN.md)

Copy this scaffold to `docs/changelogs/YYYYMMDD-short-slug.md`, replace every placeholder, and create
its adjacent Chinese counterpart.

```markdown
---
doc_id: changelog.YYYYMMDD-short-slug
title: "Short description"
language: en
source_language: en
counterpart: ./YYYYMMDD-short-slug.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: YYYY-MM-DD
---

# Short description

[简体中文](YYYYMMDD-short-slug.zh-CN.md)

- Type: Added | Changed | Fixed | Removed | Security
- Component: user-facing area

## Summary

One or two sentences describing the observable change.

## Compatibility notes

- State breaking changes and migration steps, or say that there are none.

## Links

- PR, issue, or Current Specs owner
```

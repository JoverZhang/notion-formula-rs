# Changelog entry guidelines

Write short, user-facing entries under `docs/changelogs/`.

## Goals

- Describe behavior/contract changes for integrators.
- Keep a high-signal history.

## File naming

- Directory: `docs/changelogs/`
- File name: `YYYYMMDD-short-slug.md`
  - Date should match when the change landed.
  - Slug: lowercase, hyphen-separated, descriptive (`utf16-context-json-validation`, `completion-ranking`, ...).

## Structure

Start from `docs/_description_templates/changelog_entry_template.md`:

- **Type**: `Added | Changed | Fixed | Removed | Security`
- **Component**: `analyzer | analyzer_wasm | evaluator | examples/vite | docs | other`
- **Summary**: what changed (user-visible)
- **Compatibility notes**: breaking changes / DTO impacts
- **Tests**: what you ran
- **Links**: PRs/issues/design docs

## Writing style

- Prefer observable behavior over internal refactors.
- Use the names users see (export/function/type names).
- Keep it focused on one theme.

## When an entry is required

Add a changelog entry when any of the following are true:

- User-visible behavior changes (parser/formatter output, diagnostics, completion, signature help).
- Any public API/DTO/contract changes (Rust public API or WASM exports / DTO shape).
- Compatibility-affecting changes (offset/span semantics, determinism rules, context JSON schema).
- Important bug fixes that users would want to know about.

If a change is purely internal and cannot affect outputs/contracts, a changelog entry is optional.

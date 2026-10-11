# Repository instructions

Read `DOCUMENTATION.md` before creating, changing, translating, or reviewing Markdown documentation.

When creating or updating a pull request, follow the title and description guidance in [.github/pull_request_template.md](.github/pull_request_template.md).

Use current repository files as documentation sources. The `.agent/` directory is legacy and outside the documentation system: do not read, modify, or migrate it.

## Code readability

- During implementation and review, split expressions that combine several rules into named intermediate checks or explicit guard clauses. Name each check for the rule it establishes so readers can follow the decision from the local code.
- Keep validation prerequisites visible: establish the required data shape before indexing or slicing, and express transformations as ordered steps.
- Keep simple checks direct. Extract a helper when it names a coherent operation; use comments for rationale and invariants that names and control flow do not express.

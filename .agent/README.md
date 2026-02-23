# Agent instructions (notion-formula-rs)

## General instruction

- `REPO-ROOT` refers to the root directory of this repository.
- `WORKSPACE-ROOT` refers to the Cargo workspace root (same as `REPO-ROOT`).
- Before editing any source file, read it again and make sure you respect parallel editing.
- If any `*.prompt.md` file is referenced, take immediate action following the instructions in that file.
- Prefer minimal, repo-consistent diffs; avoid unrelated refactors.

## External tools environment and context (just-only)

- Always use `just <recipe>` as the command interface.
  - Prompts and task logs must not tell users to run `cargo ...` or `pnpm ...` directly.
  - If a needed workflow is missing, add a `just` recipe instead of documenting raw commands.
- Common recipes:
  - `just typecheck`
  - `just check`
  - `just verify`
  - `just test`
  - `just wasm`
  - `just bless`
  - `just fmt`
  - `just fix`
  - `just gen-ts`
- If a prompt requires building/testing in sub-agents, follow that policy strictly.

## Coding guidelines and tooling

- This repo is a Rust workspace; follow existing patterns in `analyzer/`, `ide/`, `analyzer_wasm/`, and `examples/vite/`.
- Keep formatting/lints green using the repo’s `just` interface; don’t introduce new tooling without an explicit request.

## Task documents

- Task logs live in `.agent/tasklogs/`.
- Canonical filenames:
  - `agent_scrum.md`: Scrum / task breakdown.
  - `agent_task.md`: Design notes for the selected task.
  - `agent_planning.md`: Concrete execution plan (with code blocks).
  - `agent_execution.md`: Final, step-by-step change list to apply to code.
  - `agent_execution_finding.md`: Notes comparing user edits vs plan (created only when needed).
  - `agent_kb.md`: Knowledge-base drafting scratchpad (only used by KB prompts).
  - `agent_review*.md`: Review board workflow files (only used by review prompts).
- Verification marker:
  - Once `just verify` succeeds for the current task, append `# !!!VERIFIED!!!` to the end of `agent_execution.md`.

## Leveraging the knowledge base

- Knowledge base entry: `docs/design/README.md`.
- Documentation entry point: `docs/README.md`.
- How docs are organized (high level):
  - `docs/design/README.md`: stable architecture + cross-crate contracts + drift tracking index.
  - `docs/design/contracts.md`: hard rules (contracts).
  - `docs/design/drift-tracker.md`: known gaps / open questions.
  - Module READMEs next to code (`analyzer/README.md`, `ide/README.md`, `analyzer_wasm/README.md`, `examples/vite/README.md`): implementation details.
  - `docs/changelogs/*`: user-visible changes.
- When making design/coding decisions, consult docs first and prefer existing conventions.
- When you change behavior, public APIs/DTOs, supported syntax/builtins, or WASM boundary contracts, update the relevant `docs/*` and module README in place (summarize + rewrite/remove outdated parts; don’t only append).
- Do not change hard contracts in `docs/design/contracts.md` unless explicitly requested by the user.

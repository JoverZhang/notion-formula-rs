# IDE Design (Rust Core)

This document describes the high-level architecture of the `ide` crate.
It focuses on system flow, module boundaries, and design intent.
For implementation details, read `ide/README.md` and module source files.

## Purpose

The `ide` crate provides editor-facing behavior on top of `analyzer`:

- completion
- signature help
- formatting
- text edit application

The design goal is to keep editor behavior predictable, deterministic, and easy to evolve without duplicating semantic logic from `analyzer`.

## Scope

In scope:

- orchestration of completion and signature help
- cursor/context detection for editor interactions
- ranking, preferred selections, and edit shaping
- formatter and edit application behavior

Out of scope:

- core parsing and semantic type rules (owned by `analyzer`)
- UTF-8/UTF-16 conversion for web consumers (owned by `analyzer_wasm`)

## Core Principles

- Single semantic source of truth: semantic facts come from `analyzer`.
- Explicit orchestration: the main user flow is visible at the API entry, not hidden across many modules.
- Deterministic output: same input produces the same ordering, spans, and preferred picks.
- Byte-coordinate consistency: all core spans and cursors are UTF-8 byte offsets.
- Best-effort UX: partial input should still produce useful completion and signature results.

## Public Entry Points

Primary API:

- `ide::help(source, cursor, ctx, config) -> HelpResult`

Supporting APIs:

- `ide::format(source, cursor_byte)`
- `ide::apply_edits(source, edits, cursor_byte)`

Compatibility API (inside `completion` module):

- `completion::complete(...)` delegates to `ide::help(...)` and returns completion-shaped output.

## Architecture Overview

### 1) Orchestration Layer

`ide/src/lib.rs` owns the high-level flow via a session-style orchestrator.

Responsibilities:

- parse tokens once per request
- execute help flow in a clear order
- combine completion and signature results into `HelpResult`

### 2) Context Detection Layer

`ide/src/context.rs`

Responsibilities:

- detect call context
- classify cursor position kind
- compute replace span and query text
- provide shared context primitives for completion/signature logic

### 3) Signature Layer

`ide/src/signature.rs`

Responsibilities:

- compute signature help from call context and semantic data
- instantiate function signatures for call-site display
- produce structured signature segments for UI rendering

### 4) Completion Construction Layer

`ide/src/completion/items.rs`

Responsibilities:

- build candidate sets by position kind
- produce raw completion candidates before ranking

### 5) Completion Ranking and Finalization Layer

`ide/src/completion/ranking.rs`

Responsibilities:

- attach primary edits/cursor placement
- apply query ranking and type-aware ranking
- compute preferred indices for UI defaults

## End-to-End Help Flow

At a high level, `ide::help(...)` follows this pipeline:

1. Build request session (`source`, `cursor`, `ctx`, `config`) and tokenize once.
2. Detect cursor context (`call`, `position kind`, `replace`, `query`).
3. Compute signature help when cursor is inside a call.
4. Build completion draft from position kind and semantic expectations.
5. Finalize completion via ranking and preferred-item selection.
6. Return `HelpResult { completion, signature_help }`.

This keeps the user-visible behavior in one explicit, readable flow.

## Analyzer Reuse Strategy

`ide` intentionally reuses `analyzer` for semantic correctness:

- syntax tokens and spans
- expression/type inference
- builtin signatures and type-acceptance rules

`ide` adds editor-specific policy on top:

- cursor/context heuristics
- completion candidate shaping
- ranking strategy and preferred item policy
- edit/cursor behavior for insertion

This split avoids semantic duplication while preserving IDE-specific control.

## Data and Contract Boundaries

- Core coordinate contract: UTF-8 byte ranges, half-open `[start, end)`.
- `ide` does not perform UTF-16 conversions.
- WASM/TS DTO mapping stays outside this layer.

For stable cross-crate contract rules, see `docs/design/contracts.md`.

## Tradeoffs

Benefits:

- clearer top-level behavior and easier onboarding
- lower coupling between detection, candidate generation, and ranking
- easier to test each stage independently

Cost:

- explicit orchestration introduces some boilerplate
- compatibility surfaces may temporarily duplicate some result shapes

This tradeoff is intentional: readability and maintainability are prioritized for IDE behavior evolution.

## References

- `docs/design/README.md`
- `ide/README.md`
- `docs/design/contracts.md`
- `docs/design/wasm-boundary.md`

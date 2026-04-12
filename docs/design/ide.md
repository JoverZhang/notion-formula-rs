# ide (Design)

Design rationale for the `ide` crate.
For implementation details, see `ide/README.md`.

## Purpose

Editor-facing behavior on top of `analyzer`: completion, signature help, formatting, and text edit application.

## Pipeline

```
  Source + Cursor (byte offset)
       |
       v
  analyze_syntax() ──> tokens, AST, diagnostics
       |                (from analyzer)
       v
  Context Detection ──> position kind, call context,
       |                 replace span, query text
       |                 (ide/src/context.rs)
       |
       +──────────────────+
       |                  |
       v                  v
  Signature Help    Completion Items
       |              (ide/src/completion/items.rs)
       |              by position kind:
       |              top-level, arg, member, operator
       |                  |
  (ide/src/               v
   signature/)      Ranking + Preferred Indices
       |              (ide/src/completion/ranking.rs)
       |              query match, type score,
       |              preferred index selection
       |                  |
       +──────+───────────+
              |
              v
  HelpResult { completion, signature_help }
```

Formatting pipeline:

```
  Source + Cursor (byte)
       |
       v
  ide::format(source, cursor_byte)
       |
       v
  Full-document TextEdit (byte-edit pipeline)
       |
       v
  ApplyResult { source, cursor }
```

## Key types

| Type | Location | Role |
| --- | --- | --- |
| `HelpResult` | `ide/src/lib.rs` | Combined completion + signature output |
| `CompletionResult` | `ide/src/lib.rs` | Items, replace span, preferred indices |
| `CompletionItem` | `ide/src/completion/mod.rs` | Single completion candidate |
| `SignatureHelp` | `ide/src/signature/mod.rs` | Active function signature display |
| `DisplaySegment` | `ide/src/display.rs` | Structured UI rendering unit |
| `ApplyResult` | `ide/src/edit.rs` | Source + cursor after edit application |

## Contracts

- Single semantic source of truth: all semantic facts come from `analyzer`.
- Deterministic output: same input produces the same ordering, spans, and preferred picks.
- Byte-coordinate consistency: all core spans and cursors are UTF-8 byte offsets (see `docs/design/contracts.md`).
- Best-effort UX: partial input should still produce useful completion and signature results.
- `ide` does not perform UTF-16 conversions; that stays in `analyzer_wasm`.
- Analyzer reuse: `ide` consumes syntax tokens, spans, expression/type inference, and builtin signatures from `analyzer`. Editor-specific policy (cursor heuristics, ranking, edit shaping) is added on top.

## Why: explicit orchestration

- The main user flow is visible at the API entry (`ide::help`), not hidden across many modules.
- Detection, candidate generation, and ranking are separate stages that can be tested independently.
- Cost: some orchestration boilerplate; compatibility surfaces may temporarily duplicate result shapes.
- This tradeoff is intentional: readability and maintainability are prioritized for IDE behavior evolution.

## Source pointers

- Orchestration: `ide/src/lib.rs` (`help`, `format`, `apply_edits`)
- Context detection: `ide/src/context.rs`
- Signature help: `ide/src/signature/mod.rs`
- Completion items: `ide/src/completion/items.rs`
- Completion ranking: `ide/src/completion/ranking.rs`
- Completion matching: `ide/src/completion/matchers.rs`
- Formatter: `ide/src/format.rs`
- Edit application: `ide/src/edit.rs`
- Display segments: `ide/src/display.rs`

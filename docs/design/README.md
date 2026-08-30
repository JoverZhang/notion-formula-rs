---
doc_id: architecture.index
title: "Architecture map for notion-formula-rs"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# Design (notion-formula-rs)

[简体中文](README.zh-CN.md)

This Current orientation answers where a contributor should begin when reading the
architecture and which document owns each kind of design claim. It is intended for new and
active contributors who need an end-to-end map before reading implementation details.

This index covers stable architecture, cross-crate contracts, and design rationale. For
implementation details, read each module README (for example, `analyzer/README.md`). For the
documentation workflow and bilingual policy, see [`docs/README.md`](../README.md).

## Pipeline

```
  Source (UTF-8 string)
       |
       v
  Lexer ──> Tokens + Trivia + Lex diagnostics
       |         (analyzer/src/lexer/)
       v
  Parser (Pratt) ──> AST + Parse diagnostics
       |              (analyzer/src/parser/)
       v
  Semantic Analysis ──> TypeMap + Semantic diagnostics
       |                 (analyzer/src/analysis/
       |                  + builtin_fn/ type model)
       |
       +──> IDE (format, complete, signature help)
       |         (ide/src/)
       |
       +──> WASM boundary (UTF-8 -> UTF-16, DTOs)
       |         (analyzer_wasm/src/)
       |
       +──> Evaluator (AST -> IR -> row-batch)
                 (evaluator/src/)
```

Read the diagram from the source at the top through the shared analyzer pipeline. It shows
the primary cross-crate data flow and intentionally omits internal calls and error recovery.
Its labels match code identifiers and the shared glossary.

## Goals

- Provide stable, reusable formula parsing and diagnostics.
- Provide IDE-level editing experience: format, completion, and signature help.
- Provide WASM/TS-facing entrypoints plus a lightweight DTO anti-corruption layer, keeping UTF-8/UTF-16 coordinates consistent.

## Module Summary

| Module | Summary | Module README |
| --- | --- | --- |
| `builtin_fn/` | category DSL catalog + signature model + shared call resolution | `builtin_fn/README.md` |
| `builtin_fn_macros/` | procedural implementation of the category DSL | `builtin_fn_macros/README.md` |
| `analyzer/` | lexer + parser + AST + diagnostics + semantic | `analyzer/README.md` |
| `ide/` | format / completion / signature help / edit apply | `ide/README.md` |
| `analyzer_wasm/` | wasm-bindgen boundary + UTF-16 mapping + DTO v1 | `analyzer_wasm/README.md` |
| `evaluator/` | prepared-input synchronous row-batch runtime + generated builtin ABI | `evaluator/README.md` |
| `examples/vite/` | demo integration | `examples/vite/README.md` |
| `docs/` | design docs + changelog guidance | `docs/README.md` |

## Design Docs Index

Documents without a Chinese counterpart continue to link to their English source. They are
source-only, not incomplete translations.

| Doc | Scope |
| --- | --- |
| [`contracts.md`](contracts.md) | Cross-crate coordinates, recovery, determinism, editing, and evaluation boundaries |
| [`builtin-fn.md`](builtin-fn.md) | Builtin declaration DSL, call-signature resolution, catalog, and evaluator contracts |
| [`analyzer.md`](analyzer.md) | Lexer, parser, AST, semantic analysis, postfix sugar |
| [`ide.md`](ide.md) | Completion, signature help, formatting, edit application |
| [`wasm-boundary.md`](wasm-boundary.md) | WASM facade, DTOs, UTF-16 conversion, JS API |
| [`evaluator.md`](evaluator.md) | IR, planner, kernels, row-batch evaluation |
| [`testing.md`](testing.md) | Test inventory across all crates |
| [`demo-vite.md`](demo-vite.md) | Vite example app UI/UX |
| [`drift-tracker.md`](drift-tracker.md) | Open questions and known gaps |

## Design Philosophy

- Keep it simple: ship strong capabilities without dragging in unnecessary structure.
- Contracts first: stable boundaries and contracts come first, and changes must be traceable.
- Best-effort parsing: do not stop parsing on syntax errors; return as much useful output as possible.
- Determinism by default: same input, same output (ordering, dedupe, formatting).
- Clear boundary: Rust core uses UTF-8 bytes, JS/WASM uses UTF-16 code units.

## Glossary

Canonical English terms, Chinese counterparts, code identifiers, and concept boundaries live
in the shared [`Project Glossary`](../glossary.md).

One local notation used throughout parser and IDE tests is `$0`: it marks the cursor, a
source position in `[0, length)`.

## Language Scope

- Syntax follows Notion's official guide: <https://www.notion.com/help/formula-syntax>.

Syntax summary:

- Identifiers: start with a Unicode letter or `_`, followed by Unicode letters, digits, or `_`.
- Numbers: integers, floating-point decimals, and scientific notation.
- Strings: double-quoted strings with escapes: `\n`, `\t`, `\"`, `\\`.
- Lists: trailing commas like `[1, 2,]` are rejected.
- Operators: basic arithmetic operators `+`, `-`, `*`, `/`, `%`, `^`; `%` is modulo, and `^` is right-associative exponentiation.
- Logical operators: `&&`, `||`, `!`.
- Keywords: `not`, `true`, `false`.
- Function calls: regular function calls and member method calls are supported.
  - Regular function call: `name(arg1, ...)`.
  - Member method call: `receiver.name(arg1, ...)`.
  - Built-in function support status: `docs/builtin_functions/README.md`.

# Design (notion-formula-rs)

Stable architecture, cross-crate contracts, and design rationale.
For implementation details, read each module README (e.g. `analyzer/README.md`).
For the documentation entry point, see `docs/README.md`.

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
| `evaluator/` | row-batch runtime evaluation + provider boundary | `evaluator/README.md` |
| `examples/vite/` | demo integration | `examples/vite/README.md` |
| `docs/` | design docs + changelog guidance | `docs/README.md` |

## Design Docs Index

| Doc | Scope |
| --- | --- |
| [`contracts.md`](contracts.md) | Cross-crate hard rules (spans, determinism, tokens, edits) |
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

- `token`: a syntax unit, such as a number, string, operator, keyword, or identifier.
- `trivia`: non-semantic tokens, such as newlines, comments, and doc comments.
- `diagnostic`: an error or warning tied to source code.
- `code action`: a special diagnostic that carries a quick-fix suggestion.
- `span`: a source range represented as a half-open interval `[start, end)`.
- `cursor`: a source position in `[0, length)`. In tests we mark it as `$0` (same naming style as rustc tests).

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

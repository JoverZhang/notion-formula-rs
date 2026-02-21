# Design (notion-formula-rs)

This file only covers stable architecture, cross-crate contracts, and drift tracking.
For implementation details, read each module README, such as `analyzer/README.md`, `ide/README.md`, `analyzer_wasm/README.md`, and `examples/vite/README.md`.
For the docs entry point, see `docs/README.md`.

## Goals

- Provide stable, reusable formula parsing and diagnostics.
- Provide IDE-level editing experience: format, completion, and signature help.
- Provide WASM/TS-facing entrypoints plus a lightweight DTO anti-corruption layer, so coordinate systems stay consistent.

## Module Summary

| Module | Summary | Primary doc |
| --- | --- | --- |
| `analyzer/` | lexer + parser + AST + diagnostics + semantic | `analyzer/README.md` |
| `ide/` | format / completion / signature help / edit apply | `ide/README.md` |
| `analyzer_wasm/` | wasm-bindgen boundary + UTF-16 mapping + DTO v1 | `analyzer_wasm/README.md` |
| `evaluator/` | WIP | - |
| `examples/vite/` | demo integration | `examples/vite/README.md` |
| `docs/` | design docs + changelog guidance | `docs/README.md` |

## Design Philosophy

- Keep it simple: ship strong capabilities without dragging in unnecessary structure.
- Contracts first: stable boundaries and contracts come first, and changes must be traceable.
- Best-effort parsing: do not stop parsing on local errors; return as much useful output as possible.
- Determinism by default: same input, same output (ordering, dedupe, formatting).
- Clear boundary: Rust core uses UTF-8 bytes, JS/WASM uses UTF-16 code units.

## Glossary

- `token`: a syntax unit, such as a number, string, operator, keyword, or identifier.
- `trivia`: non-semantic tokens, such as newlines, comments, and doc comments.
- `diagnostic`: an error or warning tied to source code.
- `code action`: a special diagnostic that carries a quick-fix suggestion.
- `span`: a source range represented as a half-open interval `[start, end)`.
- `cursor`: a source position in `[0, length)`. In tests we mark it as `$0` (same naming style as rustc tests).

## Design (Keep It Simple)

### analyzer

The core goal of `analyzer` is a recovering compiler: parsing does not stop because of local errors, and still produces AST + diagnostics for IDE use and downstream semantic analysis.

Key tradeoffs:

- Keep trivia such as `group`, newlines, and comments in the AST so formatting can reuse the same structure. This avoids maintaining a separate CST in `ide`; for this lightweight grammar, the extra compile-time cost is acceptable.
- During parsing, insert `ErrorExpr` placeholders and emit diagnostics to improve one-pass diagnostic quality.
- Some diagnostics carry code actions (for example missing parentheses or commas) for lightweight quick fixes.

### ide

The core job of `ide` is to provide modern editor behaviors: format, completion, and signature help.
The design direction is to reuse `analyzer` structures and keep output stable and explainable.

See [`docs/design/completion.md`](completion.md).

### analyzer_wasm

`analyzer_wasm` is the JS/TS-facing facade for `analyzer` and `ide`, and also provides a lightweight DTO anti-corruption layer.

Design principles:

- Only expose the APIs and coordinate conversions we actually need; avoid extra logic.
- UTF-8 byte <-> UTF-16 code unit span conversion only happens here.

### evaluator

WIP.

## Language Scope

- Syntax targets follow Notion's official guide: <https://www.notion.com/help/formula-syntax>.

Syntax summary:

- Identifiers: Unicode letters or `_`, followed by Unicode code points.
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

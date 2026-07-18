# builtin_fn_macros

## Purpose

- Own the function-like procedural macro for one builtin category DSL invocation.
- Parse and validate declaration-local invariants, then expand evaluator-independent
  catalog and signature data owned by `builtin_fn`.
- Do not own cross-category validation or evaluator runtime generation.

## Public API

- `builtin_functions!` parses one category, including attributes, generics, types,
  fixed/repeat/tail parameter layouts, documentation, and deterministic Rust field-name
  lowering.

## Contracts / invariants

Malformed declarations recover at the next top-level semicolon so independent diagnostics
can be emitted together, up to the documented diagnostic limit. Cross-category order and
name uniqueness cannot be checked within a single macro invocation; mechanical catalog
tests in `builtin_fn` own those global invariants.

## Layout

| Path | Responsibility |
| --- | --- |
| `src/ast.rs` | DSL parser and declaration AST |
| `src/expand.rs` | validation and catalog/signature expansion |
| `src/lib.rs` | `builtin_functions!` proc-macro entry point |

## Flow

```text
category DSL -> parse/recover -> validate -> BuiltinCategory expression
```

## Tests

Compile-pass and compile-fail fixtures live under `builtin_fn/tests/ui/` because
`builtin_fn` provides the expansion support types used by consumers.

```bash
cargo test -p builtin_fn macro_ui
```

## TODOs

- None.

# builtin_fn_macros

Function-like procedural macro for the builtin category DSL.

The crate parses one complete category invocation, validates declaration-local invariants,
and expands it into evaluator-independent catalog and signature data owned by `builtin_fn`.
It handles attributes, generics, types, fixed/repeat/tail parameter layouts, documentation,
and deterministic Rust field-name lowering.

Malformed declarations recover at the next top-level semicolon so independent diagnostics
can be emitted together, up to the documented diagnostic limit. Cross-category order and
name uniqueness cannot be checked within a single macro invocation; mechanical catalog
tests in `builtin_fn` own those global invariants.

## Module map

| Path | Responsibility |
| --- | --- |
| `src/ast.rs` | DSL parser and declaration AST |
| `src/expand.rs` | validation and catalog/signature expansion |
| `src/lib.rs` | `builtin_functions!` proc-macro entry point |

Compile-pass and compile-fail fixtures live under `builtin_fn/tests/ui/` because
`builtin_fn` provides the expansion support types used by consumers.

```bash
cargo test -p builtin_fn macro_ui
```

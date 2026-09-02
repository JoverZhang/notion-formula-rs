---
doc_id: how.builtin-fn-macros
title: "How the builtin declaration macro reports local errors"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How the builtin declaration macro reports local errors

[简体中文](README.zh-CN.md)

This guide explains how `builtin_fn_macros` turns one category DSL invocation into a
`BuiltinCategory` expression while preserving useful compile-time diagnostics. It is for
maintainers changing the DSL parser, validation, or expansion.

The crate owns declaration-local parsing and validation. It does not own the production
catalog, cross-category invariants, signature resolution, or evaluator code generation.
The generated semantic model is described in the
[`builtin_fn` guide](../builtin_fn/README.md).

## One invocation follows one parse-to-expression pipeline

The `builtin_functions!` entry point in
[`builtin_fn_macros/src/lib.rs`](../../../builtin_fn_macros/src/lib.rs) parses a
`CategoryDecl`, passes it to expansion, and returns an expression of type
`BuiltinCategory`:

```text
token stream
    |
    v
CategoryDecl parse -- recover malformed functions
    |
    v
local validation -- collect independent errors
    |
    +-- errors --> compile_error! block, no category
    |
    v
BuiltinCategory expression
```

The macro generates neither a surrounding function nor item visibility. Ordinary Rust
code owns the category function and decides where its returned value enters the complete
catalog.

## Parsing recovers only at a stable boundary

[`builtin_fn_macros/src/ast.rs`](../../../builtin_fn_macros/src/ast.rs) parses the category
header first and then attempts one function at a time. Each function attempt runs on a
forked `syn` parse stream. A successful fork advances the original stream; a failed fork
leaves the original cursor at the declaration start.

After a function syntax error, recovery consumes tokens through the next top-level `;`.
Nested delimiter groups arrive as single token trees, so their internal punctuation cannot
be mistaken for that boundary. Recovery deliberately does not continue inside the broken
function. This gives later declarations a chance to report independent errors without
guessing how the malformed declaration was intended to nest.

Parse errors are retained beside successfully parsed functions in `CategoryDecl`.
Validation therefore sees both the syntax failures and every declaration that recovery
could safely preserve.

If the category header itself cannot be parsed, `syn::parse_macro_input!` returns that
error immediately because there is no category boundary from which expansion can
continue.

## Validation and lowering are separate phases

[`builtin_fn_macros/src/expand.rs`](../../../builtin_fn_macros/src/expand.rs) first validates
the complete local AST and only then lowers it. Local validation covers:

- known categories, generic kinds, and types for supported declarations;
- duplicate function, generic, and parameter names within the invocation;
- optional-parameter ordering and repeat shape;
- attribute syntax and incompatible `#[resolver]` / `#[unsupported]` combinations;
- the required explanation on `#[unsupported]` declarations; and
- deterministic Rust field names after snake-case and keyword normalization.

If validation succeeds, expansion constructs `BuiltinCatalogEntry`, `FunctionSig`,
`ParamShape`, generic IDs, type nodes, and canonical presentation strings using absolute
`::builtin_fn` paths. Unsupported declarations receive catalog metadata but no
`FunctionSig`.

The macro can parse a resolver path, but the Rust compiler verifies that the path exists
and has the `SigResolver` function type. The proc macro should not duplicate name
resolution or Rust type checking.

## Diagnostics stay local and point to author tokens

Independent `syn::Error` values are combined into multiple `compile_error!` expansions.
The invocation emits at most 32 diagnostics. When more errors are found, the last retained
diagnostic reports how many additional errors were suppressed.

Primary spans follow the construct the author can fix:

| Failure | Primary span |
| --- | --- |
| unknown generic kind or type | unknown identifier |
| duplicate function name | later declaration, plus a supplementary error at the first declaration |
| duplicate generic or parameter name | later declaration |
| invalid repeat minimum | integer literal |
| invalid or duplicate repeat layout | `repeat` keyword or offending member name |
| unsupported declaration without docs | `#[unsupported]` |
| incompatible or malformed attribute | offending attribute |
| function syntax error | token reported by `syn` before semicolon recovery |

If any parse or validation error exists, expansion returns no partial
`BuiltinCategory`. The proc-macro entry point emits the combined diagnostics inside a
block because the macro occupies an expression position.

Compile-fail snapshots in
[`builtin_fn/tests/ui/fail/`](../../../builtin_fn/tests/ui/fail/) pin recovery, validation,
resolver type checking, diagnostic spans, and the error limit. They live in `builtin_fn`
because expanded code refers to that crate's support types.

## Global invariants remain outside the macro

One invocation can compare declarations only within its category. It cannot see another
macro expansion, the order in which category functions are composed, or the final
supported registry. Cross-category name uniqueness, category order, support status, and
whole-catalog inclusion are therefore checked over `builtin_categories()` in
[`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs).

This split keeps diagnostics close to declaration tokens without pretending that a proc
macro has repository-wide visibility. The macro does not read repository files, render a
Markdown catalog, traverse formula ASTs, or generate evaluator runtime behavior.

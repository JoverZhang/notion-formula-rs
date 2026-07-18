# builtin_fn

Compile-time builtin catalog, shared signature model, and call-resolution engine.

Design rationale: [`docs/design/builtin-fn.md`](../docs/design/builtin-fn.md).

## Responsibility

Each builtin category contains exactly one `builtin_functions!` invocation. The DSL lowers
directly to `BuiltinCategory`, catalog presentation metadata, `FunctionSig`, and
`ParamShape`; production code does not parse signature strings at runtime.

The declaration model supports fixed parameters and explicit repeat groups in all five
layouts: fixed only, repeat only, head + repeat, repeat + tail, and head + repeat + tail.
Repeat members keep logical names in the declaration. Numbered names (`lists1`, `lists2`,
`...`) are produced only by presentation rendering.

`#[resolver(path)]` may refine only the resolved return type after normal shape and generic
binding. `flat` is the representative resolver. `#[unsupported]` entries remain in the
catalog, require a doc comment, and are excluded from executable signatures and evaluator
implementation obligations.

## Entry points

- `builtin_categories()` returns the complete ordered catalog, including unsupported
  declarations.
- `builtins_functions()` returns supported semantic/runtime signatures only.
- `resolve_call_signature()` owns incomplete-call projection, shape validation, generic
  binding, staged lambda observations, argument compatibility, and return resolvers.
- `render_builtin_readme()` and `render_builtin_catalog()` produce deterministic catalog
  documentation.
- `BuiltinSigParser` remains available for standalone signature parsing, but is not the
  builtin registry source.

`analyzer` consumes the shared resolver and retains its final result in `SemanticMap`; `ide`
uses the same projection for Signature Help; `evaluator/build.rs` consumes the catalog to
generate typed implementation contracts.

## Catalog maintenance

```bash
cargo run -p builtin_fn --bin builtin_catalog -- --check
cargo run -p builtin_fn --bin builtin_catalog -- --write
cargo test -p builtin_fn
```

The renderer owns only the marked region in
[`docs/builtin_functions/README.md`](../docs/builtin_functions/README.md). Whole-catalog
tests enforce declaration order, cross-category uniqueness, support status, resolver
placement, and deterministic output.

# builtin_fn (Design)

Design rationale for the `builtin_fn` crate.
For implementation details, see `builtin_fn/README.md`.

## Purpose

Owns the formula type model, builtin function signatures, and the string-driven signature parser.
`analyzer::semantic` re-exports these types for downstream API stability.

## Pipeline

```
  Signature strings (builtin_fn/src/builtins/*.rs)
       |
       v
  Parser ──> FunctionSig + ParamShape
       |      (builtin_fn/src/parser/)
       v
  Registry: builtins_functions() -> Vec<FunctionSig>
       |      (builtin_fn/src/builtins/mod.rs)
       v
  Context (passed to analyzer for type inference)
       |      (analyzer/src/analysis/mod.rs)
       v
  Inference: infer_expr_with_map(expr, ctx, map) -> Ty
             (analyzer/src/analysis/infer.rs)
```

## Key types

| Type | Location | Role |
| --- | --- | --- |
| `Ty` | `builtin_fn/src/types.rs` | Semantic type (Number, String, Boolean, Date, Unknown, Generic, List, Union, Fn, Ident) |
| `FunctionSig` | `builtin_fn/src/signature.rs` | name, params, ret, category, generics, detail, optional resolver |
| `ParamShape` | `builtin_fn/src/param_shape.rs` | head/repeat/tail param layout for arity rules |
| `ParamSig` | `builtin_fn/src/signature.rs` | Single param slot: name, ty, optional flag |
| `GenericParam` | `builtin_fn/src/signature.rs` | Plain vs Variant generic binding |
| `FunctionCategory` | `builtin_fn/src/types.rs` | Function grouping for completion kinds |
| `SigResolver` | `builtin_fn/src/signature.rs` | Escape hatch for complex signatures (e.g. `flat`) |
| `BuiltinSigParser` | `builtin_fn/src/parser/mod.rs` | String -> FunctionSig parser |

## Contracts

### ParamShape invariants

`ParamShape { head, repeat, tail }`:

- `head`: fixed prefix params (appear once)
- `repeat`: repeating group params (1+ times when non-empty; `repeat_min_groups = 1`)
- `tail`: fixed suffix params (appear once after repeat group)

`ParamShape::new(...)` rejects:

- optional params in `repeat`
- `repeat` + optional `tail` (tail must be required when repeat exists)
- required tail params after an optional tail param (optional tail is suffix-only)

Spec: `docs/signature-help.md`

### Type model (Ty)

- Includes `Unknown`, `Generic(GenericId)`, `List(T)`, `Union([..])`.
- UI rendering: generics render as `T0`, `T1`, ...; `List(Union(A | B))` renders as `(A | B)[]`.

### ty_accepts (validation acceptance)

- `actual = Unknown` is accepted (avoid mismatch noise when inference is unsure).
- `expected = Generic(_)` is a wildcard (only on the expected side).
- `Union` uses containment semantics:
  - `expected = Union(E...)` accepts `actual = Union(A...)` iff every `Ai` is accepted by `expected`
  - `expected = T` accepts `actual = Union(A...)` iff `T` accepts each `Ai`
- `List` is covariant: `List(E)` accepts `List(A)` iff `E` accepts `A`.
- Where: `analyzer/src/analysis/mod.rs` (`ty_accepts`)

### Generic binding rules (Plain vs Variant)

- `Plain`: `Unknown` does not bind; conflicts accumulate permissively into a deterministic union.
- `Variant`: if any participating actual contains `Unknown`, the instantiated generic becomes `Unknown`; otherwise concrete bindings accumulate into a deterministic union.
- Where: `analyzer/src/analysis/infer.rs`

### Inference + validation

- `infer_expr_with_map(expr, ctx, &mut TypeMap)` records `ExprId -> Ty`.
- Ternary type joins: if either branch is `Unknown`, result is `Unknown`; otherwise `normalize_union(then, else)` (deterministic).
- Validation is validation-first: call arity/shape errors first; on shape error, emit one diagnostic and skip per-arg mismatches.
- Where: `analyzer/src/analysis/infer.rs`, `analyzer/src/analysis/mod.rs`

## Why: string-based signatures

The original macro DSL was replaced with a string-driven parser (`builtin_fn/src/parser/`) because:

- Signature strings are more readable and closer to how builtins are documented.
- Parse errors report byte offsets, making authoring failures traceable.
- The parser is intentionally narrow: only supports the signature language needed by the registry.

## Source pointers

- Registry entry: `builtin_fn/src/builtins/mod.rs` (`builtins_functions()`)
- Function categories: `builtin_fn/src/builtins/` (general, text, math, date, people, list, special)
- Signature parser: `builtin_fn/src/parser/`
- Type model: `builtin_fn/src/types.rs`
- Param shapes: `builtin_fn/src/param_shape.rs`
- Union normalization: `builtin_fn/src/type_hints.rs`
- Inference (consumer): `analyzer/src/analysis/infer.rs`
- Validation (consumer): `analyzer/src/analysis/mod.rs`

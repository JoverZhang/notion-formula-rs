# Builtins, Context, and the type model

Semantic analysis is best-effort inference + validation against builtin signatures.

## Context

- `Context { properties, functions }`
  - `properties`: supplied externally (WASM `AnalyzerConfig`)
  - `functions`: sourced from Rust builtins (JS cannot supply them)
- Code: `analyzer/src/analysis/mod.rs`

## Builtin signatures (FunctionSig)

- Entry point: `builtins_functions() -> Vec<FunctionSig>`
  - Code: `analyzer/src/analysis/builtins/mod.rs`
- Declarations:
  - macro DSL: `analyzer/src/analysis/builtins/macros.rs`
  - function lists: `analyzer/src/analysis/builtins/*.rs`
- `FunctionSig` fields:
  - `name`, `params: ParamShape`, `ret: Ty`
  - `category: FunctionCategory` (mapped to function-specific completion kinds)
  - `detail: String` (completion/signature help)
  - `generics: Vec<GenericParam>`
- Code: `analyzer/src/analysis/signature.rs`

## ParamShape invariants (hard rules)

`ParamShape { head, repeat, tail }` is used for deterministic arity/shape rules.

Shape meaning:

- `head`: fixed prefix params (appear once)
- `repeat`: repeating group params (appear 1+ times when non-empty; see `repeat_min_groups = 1`)
- `tail`: fixed suffix params (appear once after the repeat group)

Param slots:

- `ParamSig { name, ty, optional }`
  - Code: `analyzer/src/analysis/signature.rs`

`ParamShape::new(...)` rejects:

- optional params in `repeat`
- `repeat + optional tail` (tail must be required when `repeat` exists)
- required tail params after an optional tail param (optional tail must be suffix-only)

Code:
- `analyzer/src/analysis/signature.rs` (`ParamShape::new`)

Spec:
- `docs/signature-help.md`

## Type model (Ty)

- `Ty` is the semantic type used for inference/validation/editor features.
- Includes `Unknown`, `Generic(GenericId)`, `List(T)`, `Union([..])`.
- UI rendering:
  - generics render as `T0`, `T1`, ...
  - `List(Union(A | B))` renders as `(A | B)[]` (parens from precedence)
- Code: `analyzer/src/analysis/mod.rs`

### `ty_accepts` (validation acceptance)

Rules (code-backed):

- `actual = Unknown` is accepted (avoid mismatch noise when inference is unsure).
- `expected = Generic(_)` is a wildcard (only on the expected side).
- `Union` uses containment semantics:
  - `expected = Union(E...)` accepts `actual = Union(A...)` iff every `Ai` is accepted by `expected`
  - `expected = T` accepts `actual = Union(A...)` iff `T` accepts each `Ai`
- `List` is covariant: `List(E)` accepts `List(A)` iff `E` accepts `A`

Code: `analyzer/src/analysis/mod.rs` (`ty_accepts`)

## Inference + validation shape policy

- Inference:
  - `infer_expr_with_map(expr, ctx, &mut TypeMap)` records `ExprId -> Ty`
  - `Ternary` type joins:
    - if either branch is `Unknown`, result is `Unknown`
    - otherwise `normalize_union(then, else)` (deterministic)
  - Code: `analyzer/src/analysis/infer.rs`
- Validation:
  - `analyze_expr` is validation-first:
    - call arity/shape errors first
    - on a shape error, emit one diagnostic for that call and skip per-arg mismatches
  - Code: `analyzer/src/analysis/mod.rs` (`analyze_expr`, `validate_call`)

## `prop("Name")` (special-cased)

`prop` is not modeled as a `FunctionSig`.

Rules (semantic validation):

- expects exactly 1 argument
- argument must be a string literal
- name must exist in `Context.properties` (else emit a diagnostic)

Code: `analyzer/src/analysis/mod.rs` (`validate_prop_call`)

## Generics (Plain vs Variant)

Generic binding rules are implemented in `analyzer/src/analysis/infer.rs`:

- `Plain` generics:
  - `Unknown` does not bind
  - conflicts accumulate permissively into a deterministic union
- `Variant` generics:
  - if any participating actual contains `Unknown`, the instantiated generic becomes `Unknown`
  - otherwise, concrete bindings accumulate into a deterministic union

Examples (from builtins):

- `if<T: Variant>(condition: boolean, then: T, else: T) -> T`
- `ifs<T: Variant>([condition: boolean, value: T]..., default: T) -> T`
  - Code: `analyzer/src/analysis/builtins/general.rs`

## Postfix sugar (member-call)

The parser only accepts member *calls*: `receiver.method(...)`.

Semantic treatment (best-effort):

- Inference may treat `receiver.fn(arg1, ...)` as `fn(receiver, arg1, ...)` when:
  - `fn` is in `postfix_capable_builtin_names()`, and
  - `is_postfix_capable(sig)` is true
  - Code: `analyzer/src/analysis/infer.rs`, `analyzer/src/analysis/mod.rs`
- Validation currently only applies postfix-call validation when:
  - the builtin has `flat_params()` and `flat.len() > 1`
  - Code: `analyzer/src/analysis/mod.rs` (`validate_expr` for `ExprKind::MemberCall`)
- IDE completion (`receiver.$0` / `receiver.pre$0`) uses the same postfix-first-arg idea:
  - start from postfix-capable builtins
  - keep only functions where the first postfix parameter accepts receiver type (`ty_accepts`)
  - if receiver infers to `Unknown`, current completion keeps the full postfix-capable set
    (TODO: narrow once an explicit `any` type is modeled)
  - Code: `ide/src/completion/pipeline.rs`, `ide/src/completion/items.rs`

The postfix allowlist is derived from builtins:

- `postfix_capable_builtin_names()` is built by filtering `builtins_functions()` with
  `is_postfix_capable(sig)`.
- `is_postfix_capable` requires a deterministic “first parameter slot” and at least one additional
  displayed parameter slot.
- Code: `analyzer/src/analysis/mod.rs`

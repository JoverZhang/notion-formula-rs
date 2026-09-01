---
doc_id: how.builtin-fn
title: "How builtin declarations become resolved signatures"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How builtin declarations become resolved signatures

[简体中文](README.zh-CN.md)

This guide answers how one Rust declaration becomes the catalog metadata and resolved
signature shared by semantic analysis and editor services. It is for maintainers who add
a builtin or change the signature model.

The `builtin_fn` crate owns the declaration catalog, semantic signature types, parameter
projection, generic binding, and return-type refinement. It does not own user-facing
calling guarantees, editor rendering, or row-batch execution. Those facts belong to the
builtin specification, the IDE guide and editor-services specification, and the evaluator
guide respectively.

## The catalog starts in Rust

All production declarations live in
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs). Each category function
contains one `builtin_functions!` invocation, and `builtin_categories()` composes those
functions in catalog order.

```text
builtins.rs declarations
        |
        v
builtin_fn_macros: parse, validate, expand
        |
        v
BuiltinCategory -> BuiltinCatalogEntry
        |                    |
        | supported          | #[unsupported]
        v                    v
FunctionSig             metadata only
        |
        v
resolve_call_signature -> ResolvedFunctionSig
```

The diagram stops before AST inference, IDE presentation, and evaluator dispatch. Those
consumers use the generated model but own their own behavior.

`builtin_categories()` returns every declaration, including entries marked
`#[unsupported]`. `builtins_functions()` preserves the same order while retaining only
entries that have a `FunctionSig`. The complete inventory therefore has one authoring
source: `builtin_fn/src/builtins.rs`. This guide intentionally does not copy or render the
name and signature list.

`BuiltinSigParser` parses standalone signature strings for callers that need that format.
It is not the production registry source.

## The declaration DSL carries structure, not formatted text

A category declaration describes the function name, generics, parameter layout, return
type, optional support metadata, and documentation for unsupported entries. For example:

```rust,ignore
builtin_functions! {
    category: General;

    ifs<T: Variant>(
        repeat(min = 1) {
            condition: boolean,
            value: () -> T,
        },
        else: () -> T,
    ) -> T;
}
```

The DSL accepts:

- primitive types `number`, `string`, `boolean`, `date`, `null`, and `any`;
- declared generic names, unions with `|`, postfix list types with `[]`, lambda types,
  grouped types, and `Ident<T>` binder types;
- optional fixed parameters written as `name?: Type`;
- one explicit `repeat(min = N) { ... }` group;
- `#[resolver(path)]` for return types that ordinary generic substitution cannot express;
  and
- `#[unsupported]` plus one or more doc-comment lines for catalog-only declarations.

The macro lowers a supported declaration into both a `BuiltinCatalogEntry` and a
`FunctionSig`. It derives the canonical signature and detail strings from the parsed
shape; declarations cannot provide a separate detail override. An unsupported declaration
still produces ordered catalog metadata, but its `implementation` field is `None`.

Parsing, local validation, and expansion belong to
[`builtin_fn_macros`](../builtin_fn_macros/README.md). `builtin_fn` owns the structures
produced by that expansion.

## `ParamShape` makes position explicit

Every `FunctionSig` contains
`ParamShape { head, repeat, tail, repeat_min_groups }`. Parameter position in the DSL
determines the three regions:

- ordinary parameters before `repeat` become `head`;
- parameters inside the block form one repeating group;
- ordinary parameters after the block become `tail`.

The model supports five layouts without inferring repetition from names:

| Layout | Representative declaration |
| --- | --- |
| fixed only | `flat` or `substring` |
| repeat only | `concat` |
| head + repeat | `splice` |
| repeat + tail | `ifs` |
| head + repeat + tail | the synthetic `caseOf` contract fixture |

For a repeating signature, an exact call shape satisfies:

```text
total = head.len + repeat.len * groups + tail.len
groups >= repeat_min_groups
```

`min` counts complete groups rather than individual parameters. It must be an unsuffixed,
non-negative integer literal. A repeat group cannot be empty or optional, and a declaration
can contain only one such group. Fixed `head` and `tail` parameters are also required when
repeat is present; otherwise the boundary between a repeated group and the tail would be
ambiguous.

Without repeat, optional fixed parameters are allowed only as one contiguous suffix. The
macro rejects a required parameter after an optional one. Repeat members use logical base
names such as `condition`; numeric names and the legacy `N` suffix are rejected. Consumers
derive group numbers from `ResolvedParamSlot::repeat_group` instead of encoding them in the
declaration.

The lowering and layout matrix are exercised in
[`builtin_fn/tests/macro_dsl.rs`](../../../builtin_fn/tests/macro_dsl.rs) and
[`builtin_fn/tests/equivalence.rs`](../../../builtin_fn/tests/equivalence.rs). The shared
shape projection lives in
[`builtin_fn/src/param_shape.rs`](../../../builtin_fn/src/param_shape.rs) and
[`builtin_fn/src/resolution.rs`](../../../builtin_fn/src/resolution.rs).

## Types preserve relationships between parameters

Generics receive deterministic `GenericId` values in declaration order. Two binding kinds
control how observations accumulate:

| Kind | Binding behavior |
| --- | --- |
| `Plain` | An unknown observation does not bind the generic. Distinct concrete observations form a deterministic union. |
| `Variant` | Concrete observations form a deterministic union, but any unknown observation makes the binding unknown. |

Omitting the kind is equivalent to `Plain`. Union normalization recursively flattens
nested unions, removes duplicates, and applies a stable type order. The DSL's `any` type
lowers to a hidden `Plain` generic so different occurrences can participate in the same
ordinary binding machinery.

Lambda types preserve both parameter type and binding origin. `current` lowers to
`LambdaParam::Current`. Any other lambda parameter name lowers to
`LambdaParam::ParamRef`, referring to another declared parameter. The `let` declaration
uses this relationship:

```rust,ignore
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;
```

`Ident<T>` marks the identifier-bearing argument, while the `body` parameter refers back
to it. The Analyzer later supplies the actual identifier spelling and performs staged
lambda inference; `builtin_fn` only preserves the type and reference relationship.

The model is defined in
[`builtin_fn/src/types.rs`](../../../builtin_fn/src/types.rs),
[`builtin_fn/src/signature.rs`](../../../builtin_fn/src/signature.rs), and
[`builtin_fn/src/type_hints.rs`](../../../builtin_fn/src/type_hints.rs).

## Resolution returns one immutable snapshot

`resolve_call_signature()` accepts a `FunctionSig` and arguments in semantic order. A
postfix receiver, when present, has already been inserted at index zero. Each argument is
observed as either:

- `Empty`, when a syntactic slot exists but has no expression; or
- `Typed(Ty)`, including `Ty::Unknown` when an expression exists but inference is
  inconclusive.

The end of the input slice means that no argument slot exists yet. Keeping these cases
separate lets incomplete editor input use the same resolver as complete semantic input.

Resolution performs these steps:

1. project the observed count onto the fixed or repeating shape;
2. map projected positions to `ParamRef::Head`, `Repeat`, or `Tail`;
3. bind generics from typed observations;
4. instantiate declared parameter and return types;
5. invoke a custom return resolver, if present;
6. compare observations with instantiated parameter types; and
7. return validity, projection, per-argument status, and return type together.

A count accepted by the shape produces `ShapeValidity::Valid`. An incomplete or invalid
count produces a specific `CallShapeError`, but resolution still returns the smallest
completable projection. Excess arguments that have no projected parameter are marked
`Unmapped`. Empty and unknown arguments are `Indeterminate`, so a partial call does not
become a false type mismatch.

The projection remains semantic data: each `ResolvedParamSlot` carries a logical
`ParamRef`, an optional one-based repeat-group number, an optional source argument index,
and an instantiated expected type. It does not contain a rendered label or active editor
parameter. Re-running resolution uses no hidden state.

For a repeating shape, `resolve_repeat_tail_used()` first finds a split whose middle is a
whole number of groups and meets `repeat_min_groups`. It retains the required tail prefix
and prefers the largest tail count if an externally constructed shape admits more than one
split. Production declarations cannot have an optional tail with repeat, so their exact
split is unique. When an observed count has no exact split, projection chooses the
smallest greater or equal count that does, using the count only to build completion slots;
it never invents observations for absent source arguments.

[`builtin_fn/tests/resolution.rs`](../../../builtin_fn/tests/resolution.rs) covers exact,
incomplete, and invalid shapes, generic binding, staged observations, and all five layout
forms.

### Custom resolvers can refine only the return type

`#[resolver(path)]` attaches a `SigResolver` with this input:

```rust,ignore
pub struct ResolverInput<'a> {
    pub arguments: &'a [ArgumentObservation],
    pub default_return_ty: &'a Ty,
}
```

Normal shape projection and generic substitution run first. The resolver then returns the
final `return_ty`. It cannot change parameter mapping, expected argument types, validity,
catalog metadata, or generic declarations. It also runs for partial snapshots, so it must
fall back to `default_return_ty` when observations are empty or unknown.

`flat` is the production example. Its resolver recursively collects non-list leaves from
the first list argument and normalizes them into the result element union. The function
and its focused test live in
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs) and
[`builtin_fn/tests/resolution.rs`](../../../builtin_fn/tests/resolution.rs).

## Postfix eligibility is derived from the signature shape

The current postfix gate is implemented by
`analyzer::semantic::is_postfix_capable` in
[`analyzer/src/analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs). It derives
eligibility from the supported `FunctionSig`; the catalog does not maintain a second
postfix flag.

A signature is eligible only when it has a deterministic first parameter and at least one
additional physical argument position:

- a non-empty `head` supplies the receiver slot, and the full displayed shape must contain
  at least two logical parameters;
- otherwise a non-empty repeat group supplies the receiver slot, and either the displayed
  shape has at least two logical parameters or its minimum repetition requires at least
  two physical slots;
- a tail-only signature, or a single repeat slot whose minimum is only one group, is not
  eligible.

This keeps a repeat-only declaration such as `concat(min = 2)` eligible without making a
one-group reducer eligible. The Analyzer desugars an eligible member call to an ordinary
call with the receiver prepended. From that point on, `resolve_call_signature()` uses the
same shape and type path as prefix syntax. Postfix rendering and cursor mapping remain IDE
responsibilities.

## Validation is split across the layer that has enough context

No single validation site can see every invariant:

- `builtin_fn_macros` checks syntax, attributes, generics, supported types, parameter
  names, optional placement, and repeat layout within one category invocation. Any local
  error prevents a partial `BuiltinCategory` from being generated.
- `ParamShape::new` and `FunctionSig::new_builtin` defend the semantic structures when
  callers construct them outside the production DSL. Violating those programmer
  invariants panics rather than producing a recoverable call result.
- ordinary Rust type checking verifies that a resolver path exists and has the
  `SigResolver` function type.
- tests over `builtin_categories()` check invariants that one macro invocation cannot see,
  including cross-category name uniqueness, category order, support status, and registry
  inclusion.
- Analyzer, IDE, and evaluator tests check only the behavior added by those consumers;
  they do not redefine the declaration model.

The principal whole-catalog checks are in
[`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs). Compile-pass and
compile-fail coverage for the DSL is under
[`builtin_fn/tests/ui/`](../../../builtin_fn/tests/ui/), while the procedural-macro
implementation explains its own recovery and diagnostic boundary in the
[`builtin_fn_macros` guide](../builtin_fn_macros/README.md).

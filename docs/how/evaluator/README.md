---
doc_id: how.evaluator
title: "How the evaluator prepares and executes a formula"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How the evaluator prepares and executes a formula

[简体中文](README.zh-CN.md)

The `evaluator` crate turns a parsed expression and a schema into an owned execution plan, then
runs that plan over caller-prepared columns. This guide explains how that boundary keeps execution
synchronous, how masks preserve lazy control flow, and how builtin declarations become typed
kernel obligations. It is for maintainers who need to extend or debug the evaluator.

The guide describes the Current Rust implementation. Formula-language behavior and integration
contracts belong in `docs/specs/`; external data loading, parsing, and the complete builtin
inventory are outside its scope.

## Preparation separates meaning from row data

Evaluation has two phases. Preparation resolves everything that depends on syntax and schema;
execution later supplies only row data and a frozen runtime snapshot.

```text
parsed Expr + EvalContext
          |
          v
 prepare_formula -- semantic analysis --> SemanticMap
          |
          v
       Planner -------------------------> PreparedFormula
          |                                  |
          |                                  +--> ExecPlan
          |                                  +--> RequiredColumn[]
          |                                           |
          |                                caller prepares columns
          |                                           |
          +-------------------------------------------v
                           EvalInputsBuilder::finish
                                      |
                                      v
              RowBatch + Mask + EvalInputs --> Runtime --> EvalBlock
```

Read the diagram from the parsed expression at the top to the row result at the bottom. The left
side finishes semantic work and freezes the plan; the right side is the caller-owned input seam.
The diagram intentionally omits parsing, data retrieval, and individual operator algorithms.

[`prepare_formula`](../../../evaluator/src/planner/prepared.rs) runs semantic analysis with the
properties in `EvalContext` and the supported Rust builtin registry. Semantic diagnostics stop
preparation before a `PreparedFormula` exists. On success, the final `SemanticMap` contains the
expression types and resolved builtin call signatures that the planner needs. The planner does not
infer builtin generics again from batch values or rerun custom signature resolvers.

[`Planner`](../../../evaluator/src/planner/planner.rs) walks the expression and lowers it into an
owned `ExecPlan`. Each `PlanId` indexes an immutable node for a literal, input, operator, cast,
variable, list, branch, or builtin call. Lambda bodies and thunks are plans too; the runtime does
not retain analyzer AST references. ABI casts are inserted while lowering so runtime dispatch can
work with typed columns.

### Required columns are part of the prepared layout

When the planner encounters `prop("Name")`, it resolves the property through `EvalContext`, assigns
an `InputSlot`, and records a `RequiredColumn` containing the name and semantic type. Repeated
references to the same name reuse one slot. The manifest is therefore complete, deduplicated, and
ordered by first appearance, including references inside branches that may not execute for a
particular row.

An `InputSlot` carries both its index and an opaque layout identity. It is meaningful only for the
`PreparedFormula` that created it. The caller may load required columns concurrently or by any
other policy, but that work must finish outside the evaluator before row execution begins. The
crate contains no provider or asynchronous execution boundary.

[`EvalInputsBuilder`](../../../evaluator/src/core/inputs.rs) collects the prepared columns and the
`BuiltinRuntimeContext`. `finish` checks that every slot is present exactly once, belongs to the
same prepared layout, has the expected physical ABI kind, and has the requested batch and validity
length. It then freezes the columns into `EvalInputs`. A later `evaluate` call also checks that the
inputs belong to the prepared layout and that the inputs, `RowBatch`, and optional execution mask
have matching lengths.

## The runtime walks the plan under a mask

`PreparedFormula::evaluate` creates an all-row mask; `evaluate_with_mask` accepts one supplied by
the caller. [`Runtime`](../../../evaluator/src/runtime/evaluator.rs) recursively evaluates the root
node under that mask. Ordinary operators materialize their inputs and call typed operator helpers.
`&&`, `||`, ternary expressions, and Controlled builtins instead derive narrower masks before
visiting later operands or branches.

This distinction matters even though every property column is already loaded. Preparation is
eager about dependencies, but execution remains lazy about computation. An unselected branch may
contribute a `RequiredColumn`, yet its nodes do not run and cannot produce row errors.

Lambda execution uses a stack of name-to-column scopes. A `LambdaPlan` contains only a plan owner,
node ID, parameter names, and debug contract. `Runtime::apply_lambda` validates the handle, pushes
the supplied bindings, evaluates the body under the element mask, and then removes the scope.
Handles from a different plan are rejected instead of being followed.

The runtime snapshot is also explicit. `EvalInputs` owns one `BuiltinRuntimeContext` with the
evaluation timestamp and timezone offset, while `RowBatch` owns the row IDs. Value kernels read
these through `BuiltinKernelContext`, so `now()` and `today()` share one frozen clock and `id()`
reads the matching row identity.

## Columns carry three independent row states

The evaluator deliberately separates whether a row runs, whether it succeeded, and whether it is
null:

| Representation | Question it answers | Meaning of `false` |
| --- | --- | --- |
| execution `Mask` | Should this node run for the row? | The physical slot is inactive |
| `EvalBlock.ok` | Did evaluation succeed for the row? | The slot is a placeholder and must not be consumed |
| `Validity` | Does a successful row contain a value? | The row is null |

Null is not an error, and an inactive row is not null. Inactive physical slots are normalized to
successful, non-null placeholders so their storage cannot leak control-flow state. Only the mask
decides whether those placeholders are observable.

[`Column`](../../../evaluator/src/core/columns.rs) has physical variants for number, boolean, text,
date, list, and dynamic `Value` data. Each variant contains a typed `KernelColumn<K>` with
`SharedStorage` and a separate `Validity` (`AllValid`, `AllNull`, or a shared bitmap). Cloning a
kernel column clones its `Arc`-backed storage handle rather than its rows. A kernel with sole
ownership can recover the buffer through `try_into_unique`; `abs()` uses this path for in-place
work and allocates a result when storage is shared.

`EvalBlock` combines a physical `Column`, the `ok` mask, and indexed `EvalError` values. The
physical value in a null, failed, or inactive slot is an implementation placeholder. Kernel code
must consult the corresponding state before reading it.

[`abi_kind_for_ty`](../../../evaluator/src/core/inputs.rs) maps semantic types onto these physical
variants. After removing `Null`, a union with at least one remaining member retains an ABI kind
when every member maps to that kind; a heterogeneous union falls back to `Any`. `Unknown`,
generics, `Fn`, and `Ident` also use `Any`. Null itself has no column variant and remains in
`Validity`. The builtin generator applies the same mapping in
[`build_support.rs`](../../../evaluator/build_support.rs), keeping prepared inputs and generated
argument and return types on the same ABI boundary.

## Builtin declarations generate an evaluator ABI

The complete builtin inventory remains in
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs). During compilation,
[`evaluator/build.rs`](../../../evaluator/build.rs) reads the supported declarations and writes a
deterministic Rust contract into `OUT_DIR`. Unsupported declarations are filtered out and create no
evaluator obligation.

For each supported declaration, the generator in
[`build_support.rs`](../../../evaluator/build_support.rs) emits:

- a `BuiltinKey` entry and its evaluation mode and return ABI;
- a marker type and per-function kernel trait;
- named typed `Args` or `Plans`, including repeat-group structures; and
- a dispatch arm that decodes arguments and calls the marker through the trait.

The generated code contains no implementation body. Handwritten trait implementations in
[`builtins/implementations.rs`](../../../evaluator/src/builtins/implementations.rs) delegate to
reusable kernels. Because generated dispatch references every marker and trait, a missing
implementation or an incompatible method signature fails compilation.

The planner reuses each call's final resolved signature from `SemanticMap` to assign logical
`ParamRef` positions and repeat-group numbers. A shared `ArgumentPool` in
[`builtins/support/arguments.rs`](../../../evaluator/src/builtins/support/arguments.rs) later
performs destructive lookup by that pair and reports missing or duplicate arguments. Dispatch does
not repeat parameter-shape or generic resolution.

### Value mode materializes arguments before dispatch

A declaration whose top-level parameters contain neither `Fn` nor `Ident` generates a Value
contract. The runtime evaluates every argument plan under the current mask, preserves upstream row
errors, and intersects their `ok` masks to produce the rows eligible for the kernel. It does not
intersect `Validity`: a null-aware function such as `empty()` must still be able to inspect null.

The generated decoder moves typed `KernelColumn<K>` handles into the function's named `Args`; it
does not copy row buffers. The handwritten kernel chooses the compute path that matches its
semantics: a pure total operation may compute all physical slots, a fallible operation computes
only eligible non-null rows, and a null-aware operation receives null rows explicitly. Function
signatures cannot determine that choice, so it remains handwritten in
[`kernels/`](../../../evaluator/src/kernels/).

### Controlled mode keeps every argument as a plan

If any top-level parameter is a function or identifier, the generator selects Controlled mode and
keeps the entire argument set unevaluated. Delaying only lambda arguments would be insufficient:
for `ifs()`, a later ordinary boolean condition must not run for rows already matched by an earlier
condition.

Generated fields become typed `ValuePlan`, `ThunkPlan`, `LambdaPlan`, or `BinderHandle` values.
The kernel receives those handles plus the restricted `BuiltinEvalContext`, which can evaluate a
plan under a chosen mask, apply a lambda with bindings, or split a condition mask. It cannot access
the analyzer AST. [`kernels/controlled.rs`](../../../evaluator/src/kernels/controlled.rs) uses this
interface for branch selection, binders, and per-element list evaluation; short-circuiting shrinks
the active mask as work completes.

Plan handles carry an owner token, so a Controlled kernel cannot accidentally execute a node from
another `ExecPlan`. Generated plan structures contain no borrowed AST lifetime, and dispatch is
static through a generic context rather than `dyn` trait objects.

### Golden fixtures prove catalog wiring, not every failure path

The builtin golden suite requires one baseline fixture for every supported declaration. Each
fixture enters through production syntax and semantic analysis, then crosses `prepare_formula`,
required-column construction, `EvalInputsBuilder`, and masked evaluation before comparing row
outcomes. The invariant in [`builtin_golden.rs`](../../../evaluator/tests/builtin_golden.rs) and its
[`support module`](../../../evaluator/tests/support/builtin_golden.rs) proves nominal catalog
coverage and dispatch wiring. It does not claim that one baseline exhausts each function's null,
error, mask, or boundary behavior; focused fixtures and lower-level tests cover selected cases.

## Failures stop at the boundary that owns them

| Failure | Representation | Effect |
| --- | --- | --- |
| Semantic or lowering failure | `PrepareError` | No `PreparedFormula` is returned |
| Missing, duplicate, wrong-kind, wrong-length, or foreign-layout input | `InputContractError` | The whole operation is rejected before kernels run |
| Formula or data failure for one row | `EvalError` with `ok[row] = false` | Other eligible rows continue |
| Null result | `Validity` | The row succeeds with no value |
| Generated ABI mismatch | Rust compilation error | The evaluator implementation cannot build |
| Runtime implementation-contract violation | debug assertion | Signals an evaluator bug; no recovery contract is promised |

Whole-operation failures do not produce a partial `EvalBlock`, so there is nothing to merge or roll
back. Row failures retain their indexed errors and use placeholders only for storage; downstream
work excludes them through `ok`. Controlled execution never merges errors from a branch or later
predicate that its mask did not evaluate.

Generated traits and Rust types protect the static ABI. Debug builds add resolved argument and
return-type checks for active, successful, non-null rows at materialization and dispatch
boundaries. These assertions diagnose implementation mistakes; release-mode data errors still
have to become ordinary `EvalError` values rather than relying on assertions.

## Where to continue reading

- Preparation and input-layout behavior: [`planner/prepared.rs`](../../../evaluator/src/planner/prepared.rs),
  [`planner/planner.rs`](../../../evaluator/src/planner/planner.rs), and
  [`core/inputs.rs`](../../../evaluator/src/core/inputs.rs).
- Masked IR execution: [`runtime/evaluator.rs`](../../../evaluator/src/runtime/evaluator.rs) and
  [`runtime/operators.rs`](../../../evaluator/src/runtime/operators.rs).
- Typed storage and row-state representations:
  [`core/columns.rs`](../../../evaluator/src/core/columns.rs) and
  [`core/types.rs`](../../../evaluator/src/core/types.rs).
- Generated and handwritten builtin seam: [`build_support.rs`](../../../evaluator/build_support.rs),
  [`builtins/support/arguments.rs`](../../../evaluator/src/builtins/support/arguments.rs), and
  [`builtins/implementations.rs`](../../../evaluator/src/builtins/implementations.rs).
- Representative invariant tests:
  [`runtime_structure.rs`](../../../evaluator/tests/runtime_structure.rs) and
  [`generated_contract.rs`](../../../evaluator/tests/generated_contract.rs).

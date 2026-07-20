# evaluator

## Purpose

- Prepare semantically resolved formulas as owned execution plans with complete property
  dependencies.
- Validate caller-prepared typed columns before synchronous row-batch evaluation.
- Own the generated builtin ABI and the shared storage, mask, validity, planner, and runtime
  boundaries used by builtin kernels.
- Do not load external data or perform asynchronous work during evaluation.

Design rationale: [`docs/design/evaluator.md`](../docs/design/evaluator.md) and
[`docs/design/builtin-fn.md`](../docs/design/builtin-fn.md). Cross-crate rules:
[`docs/design/contracts.md`](../docs/design/contracts.md).

## Public API

- `prepare_formula(expression, context) -> Result<PreparedFormula, PrepareError>` performs
  semantic analysis once and lowers the final `SemanticMap` into an owned plan.
- `PreparedFormula::required_columns()` returns the complete, deduplicated input manifest in
  first-appearance order.
- `EvalInputsBuilder::finish(prepared, batch_len)` validates slots, ABI kinds, column and
  validity lengths, completeness, duplicates, and input-layout identity.
- `PreparedFormula::evaluate(batch, inputs)` and `evaluate_with_mask(...)` execute
  synchronously after inputs are finalized.
- `KernelColumn<K>` exposes typed read-only rows, cheap shared clones, and
  `try_into_unique()` for in-place work under unique ownership.

## Contracts / invariants

- External loading finishes before evaluation. The evaluator contains no `Provider`, Future,
  `block_on`, or runtime property-name lookup.
- `Mask`, row `ok`, and null `Validity` are independent; inactive rows remain successful and
  non-null placeholders.
- `RowBatch` owns row IDs. `BuiltinRuntimeContext` freezes evaluation time and timezone for
  every row in one evaluation.
- The Planner consumes Analyzer's final `ResolvedFunctionSig`; it does not bind generics or
  invoke signature resolvers from runtime values.
- `evaluator/build.rs` deterministically generates one trait, marker, typed Args/Plans shape,
  metadata entry, and dispatch binding for every supported catalog declaration.
- Missing implementations and mismatched method signatures fail compilation. Generated code
  contains no implementation body, `todo!()`, unreachable stub, explicit Args/Plans lifetime,
  or `dyn BuiltinEvalContext`.
- Debug builds reuse resolved parameter and return types to validate active, successful,
  non-null rows at dispatch boundaries.

All generated-trait implementations live in `src/builtins/implementations.rs`. Value kernels
materialize typed arguments once and reuse shared family implementations; Controlled kernels
retain typed plans so they can evaluate only the rows and branches selected by their masks.
Unsupported catalog declarations do not generate evaluator obligations.

## Builtin behavior

- Value families cover general, text/regex, numeric, date, eager-list, and special functions.
  Fixed ABI fields are read directly as typed scalars; top-level dynamic type switches are
  confined to `AnyKind` fields (list elements remain `Value` by storage design). Null rows
  propagate through `Validity` unless a function such as `empty()` explicitly interprets null.
- `if()` and `ifs()` split masks before evaluating thunks. `let()` and list functions apply
  generated lambda plans with typed binding contracts; unselected branches and later predicates
  do not contribute errors.
- `now()` and `today()` read the frozen `BuiltinRuntimeContext`; `id()` reads the matching
  `RowId` from the current `RowBatch`.
- `abs()` demonstrates the unique-storage in-place path while shared input columns keep their
  row buffers and allocate a separate result.

## Layout

| Path | Responsibility |
| --- | --- |
| `build.rs` / `build_support.rs` | deterministic catalog-to-Rust contract generation |
| `src/core/` | typed columns, shared storage, masks, runtime snapshots, inputs, and errors |
| `src/planner/` | SemanticMap handoff, required-column manifest, ABI casts, and `PreparedFormula` |
| `src/ir/` | owned arena plan and typed controlled-plan metadata |
| `src/runtime/` | synchronous masked IR execution and non-builtin operators |
| `src/builtins/` | generated contract inclusion, typed dispatch support, and impl obligations |
| `src/kernels/` | Value-family kernels, Controlled mask/lambda kernels, and reusable helpers |
| `tests/generated_contract.rs` | deterministic shape and compile-fail ABI contracts |
| `tests/runtime_structure.rs` | public input, ownership, state, and preparation contracts |
| `tests/builtin_behavior.rs` | bounded public behavior matrix across builtin families |

## Flow

```text
formula + EvalContext
        |
        v
prepare_formula + SemanticMap --> PreparedFormula --> required_columns()
                                                        |
                                            caller prepares columns
                                                        |
                                                        v
RowBatch + EvalInputsBuilder::finish --> EvalInputs --> synchronous runtime
```

Generated Value dispatch receives materialized typed columns. Generated Controlled dispatch
receives owned typed plan handles and a generic synchronous context; the concrete context may
borrow runtime state without leaking lifetimes into generated types.

## Tests

```bash
cargo test -p evaluator --test generated_contract
cargo test -p evaluator --test runtime_structure
cargo test -p evaluator --test builtin_behavior
cargo test -p evaluator
```

Tests cover all five parameter layouts, missing/wrong implementations, every
`InputContractError`, required-column ordering, independent runtime states, shared fan-out,
unique and in-place storage paths, debug contracts, Value family representatives, Controlled
branch behavior, generated lambda-binding contracts, and preservation of non-builtin operators.

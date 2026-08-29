---
doc_id: architecture.evaluator
title: "How formula evaluation crosses the prepared-input boundary"
language: en
source_language: en
counterpart: ./evaluator.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-29
---

# Evaluator Design

[简体中文](evaluator.zh-CN.md)

This Current document answers how a parsed formula is semantically analyzed, prepared, and
turned into a synchronous row-batch result, and where data-loading, input-contract, null, and
row-error responsibilities separate. It is for evaluator maintainers and Rust integrators who
need a working understanding of preparation, execution, and failure behavior.

The scope begins with a parsed expression and caller-owned schema and ends with an `EvalBlock`.
It does not define external data retrieval or the semantics of individual builtin functions. See
[`evaluator/README.md`](../../evaluator/README.md) for the current implementation and
[`builtin-fn.md`](builtin-fn.md) for the traits, typed ABI, and input manifest generated
from builtin declarations.

## Goals

Preparation runs semantic analysis and lowers its final `SemanticMap` into an owned plan. The
runtime then consumes caller-prepared typed columns and an execution mask, executes the IR
synchronously, and returns per-row values, null state, and errors.

## Pipeline

```text
parsed expression + schema
          |
          v
 Semantic Analysis --> SemanticMap
          |
          v
      Planner
          |
          +--> PreparedFormula
          |      +-- ExecPlan
          |      +-- RequiredColumn[]
          |
          +-----------------------------+
                                        |
caller loads complete columns from      |
RequiredColumn                          |
          |                             |
          v                             |
 EvalInputsBuilder --validate----------+
          |
          v
      EvalInputs
          |
          v
PreparedFormula::evaluate (synchronous)
          |
          +--> IR walker
          +--> generated builtin dispatch
          +--> handwritten kernels
          |
          v
       EvalBlock
          +-- values
          +-- validity
          +-- ok
          +-- row errors
```

Read the diagram from the parsed expression at the top through caller preparation into synchronous
execution. It shows the ownership seam and result states; it intentionally omits individual IR
nodes, builtin algorithms, and external I/O scheduling.

## Core Types

| Type | Purpose |
| --- | --- |
| `PreparedFormula` | Resolved execution plan and complete input dependencies |
| `RequiredColumn` / `InputSlot` | Property columns callers must prepare and their plan-local slots |
| `EvalInputsBuilder` | Collects and validates inputs against the prepared layout |
| `EvalInputs` | Finalized complete input columns ready for synchronous evaluation |
| `InputContractError` | Missing columns, duplicate slots, or incorrect kinds, lengths, or layouts |
| `KernelColumn<K>` | Typed runtime column with shared ownership |
| `Validity` | `AllValid`, `AllNull`, or a shared bitmap |
| `Mask` | Rows the current control-flow step must execute |
| `EvalBlock` | Column, validity, `ok`, and per-row errors |
| `BuiltinEvalContext` | Synchronous interface for Controlled builtins to execute typed plans under masks |

## Contracts

- The Planner consumes `ResolvedFunctionSig` values stored by the Analyzer and does not
  bind generics again or execute signature resolvers from batch values.
- `PreparedFormula::required_columns()` returns the complete, deduplicated manifest of
  properties statically referenced by the formula, including references in branches that
  may be unselected.
- Callers prepare every column before evaluation. External data sources may load
  asynchronously, but the evaluator contains no Provider, Future, or `block_on`.
- Before any kernel starts, `EvalInputsBuilder` validates slots, ABI kinds, batch length,
  and layout. Failure returns `InputContractError` and produces no partial result.
- Execution masks, `ok`, and `Validity` are independent states. Null is a successful value,
  not a row error; an inactive row is not null either.
- `ok[i] = false` means the row's physical value is a placeholder that downstream kernels
  must not consume.
- `if`, `ifs`, `&&`, `||`, and lambda builtins remain mask-lazy. Preloading complete columns
  does not permit eager evaluation of unselected expressions.
- Each supported builtin uses generated traits, markers, and dispatch bindings. Missing
  implementations or ABI mismatches fail at compile time.

## Why Use an IR

- The IR fixes builtins, types, and parameter shapes already resolved by semantic analysis
  into executable nodes.
- The Planner can select typed column specializations instead of making kernels match on a
  dynamic `Value`.
- Execution masks propagate naturally through control-flow branches.
- `PlanId` and named Args/Plans give Controlled builtins a restricted evaluation interface.
- Property input nodes reference `InputSlot` directly, avoiding runtime string lookup.

## Why Callers Prepare Columns

External data retrieval and formula computation have different scheduling and error
boundaries. Callers can load database or API data concurrently from the `RequiredColumn`
manifest and then construct `EvalInputs` once; the evaluator performs only synchronous,
deterministic column computation.

This boundary means a column referenced by an unselected branch may still be loaded, but
the branch expression itself remains unevaluated. In return, kernel ABIs stay synchronous,
async traits do not propagate through the evaluator, and the input contract remains
explicit.

## Error Boundaries

| Category | Representation | Scope |
| --- | --- | --- |
| Preparation error | `PrepareError` | Returned before an executable plan exists |
| Input structure error | `InputContractError` | Returned for the whole batch before evaluation |
| Row evaluation error | `EvalError` + `ok` | One row |
| Null | `Validity` | Valid row value |
| Kernel contract error | Debug assertion | Implementation error during development |

A `PrepareError` requires correcting the expression, schema, or unsupported construct before
building inputs. An `InputContractError` is recoverable only by correcting or rebuilding the
caller-owned inputs; no kernel has started, so there is no partial evaluator result to merge or
roll back. A row-level `EvalError` does not abort the batch: unaffected rows continue and the
failed row is marked through `ok` plus its error entry. Null remains a successful value. A kernel
debug assertion indicates an implementation contract violation and has no evaluator-level
recovery guarantee.

## Implementation Entry Points

- Current Planner: `evaluator/src/planner/`
- Current IR: `evaluator/src/ir/`
- Current runtime: `evaluator/src/runtime/`
- Current kernels: `evaluator/src/kernels/`
- Current implementation status and known differences: `evaluator/README.md`

## Runtime Behavior Verification

Supported builtin behavior is verified through catalog-complete golden fixtures under
`evaluator/tests/builtins/`. Each fixture crosses the public evaluator seam: production
syntax and semantic analysis, `prepare_formula`, required-column construction,
`EvalInputsBuilder`, and masked row-batch evaluation. The fixture metadata only supplies
caller-owned property columns, row IDs, masks, and the frozen runtime context.

Every supported catalog declaration requires one baseline fixture in its catalog category.
Additional fixtures are reserved for readable boundary and regression cases. Snapshots show
the exact source and only observable row outcomes—value, null, error, or inactive—rather
than physical placeholders or internal column storage.

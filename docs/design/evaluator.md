# Evaluator Design

This document defines the row-batch formula evaluation boundary. See
[`evaluator/README.md`](../../evaluator/README.md) for the current implementation and
[`builtin-fn.md`](builtin-fn.md) for the traits, typed ABI, and input manifest generated
from builtin declarations.

## Goals

The evaluator consumes a semantically analyzed formula, caller-prepared typed columns, and
a row mask; executes the IR synchronously; and returns per-row values, null state, and
errors.

## Pipeline

```text
formula + schema + SemanticMap
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
- Each supported builtin uses generated traits, markers, and dispatch bindings to require
  an evaluator implementation. Missing implementations or ABI mismatches fail at compile
  time.

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
| Input structure error | `InputContractError` | Returned for the whole batch before evaluation |
| Formula semantic error | `EvalError` + `ok` | One row |
| Null | `Validity` | Valid row value |
| Kernel contract error | Debug assertion | Implementation error during development |

## Implementation Entry Points

- Current Planner: `evaluator/src/planner/`
- Current IR: `evaluator/src/ir/`
- Current runtime: `evaluator/src/runtime/`
- Current kernels: `evaluator/src/kernels/`
- Current implementation status and known differences: `evaluator/README.md`

# evaluator (Design)

Design rationale for the `evaluator` crate.
For implementation details, see `evaluator/README.md`.

## Purpose

Row-batch formula evaluation. Lowers AST to an IR, dispatches operations through a kernel registry, and returns per-row results with externalized errors.

## Pipeline

```
  Expr (AST) + EvalContext
       |
       v
  Planner ──> ExecPlan (IR tree)
       |        (evaluator/src/planner/)
       |
       |    Runs analyzer's infer_expr_with_map
       |    to produce TypeMap, then lowers AST
       |    to ExecNode variants.
       v
  Evaluator ──> walks IR, dispatches kernels
       |         (evaluator/src/runtime/)
       |
       |    Binary ops dispatched via static
       |    BinaryKernelRegistry.
       v
  EvalBlock
       |
       +── values: ColumnBlock (Column::F64 or Column::Any)
       +── ok: Mask (row ok/fail flags)
       +── errors: Vec<(usize, EvalError)>
```

## Key types

| Type | Location | Role |
| --- | --- | --- |
| `EvalContext` | `evaluator/src/core/context.rs` | Batch context for evaluation |
| `Provider` | `evaluator/src/core/provider.rs` | Async external value source for `prop(...)` |
| `EvalBlock` | `evaluator/src/core/types.rs` | Row-level result (values, ok, errors) |
| `ColumnBlock` | `evaluator/src/core/types.rs` | Typed column storage |
| `Column` | `evaluator/src/core/types.rs` | `F64(Vec<f64>)` or `Any(Vec<Value>)` |
| `Mask` | `evaluator/src/core/types.rs` | Row ok/fail bitmap |
| `Value` | `evaluator/src/core/types.rs` | Data-only value (no errors) |
| `RowBatch` | `evaluator/src/core/types.rs` | Row count for a batch |
| `ExecPlan` / `ExecNode` | `evaluator/src/ir/nodes.rs` | IR tree (LiteralF64, LiteralAny, CastToF64, Binary) |
| `BinaryExecKey` | `evaluator/src/kernels/registry.rs` | Kernel lookup key |
| `Evaluator` | `evaluator/src/runtime/evaluator.rs` | IR walker + kernel dispatch |

## Contracts

- `Value` is data-only; row errors are externalized via `EvalBlock { ok, errors }`.
- `ok[i] = false` means `values[i]` is placeholder-only and must not be consumed.
- `Provider::get_prop` receives full `Property` metadata plus optional row `mask`.
- `Provider::get_prop` must return a `ColumnBlock` whose length equals batch row count.
- `if(cond, then, else)`, `&&`, and `||` are mask-driven; branch sides evaluated only for required rows.
- See `docs/design/contracts.md` for full cross-crate contract listing.

## Why: IR instead of direct AST walking

- Kernel dispatch: binary operations are dispatched through a static registry, allowing f64 fast paths and polymorphic fallbacks.
- Mask propagation: the IR tree structure makes it natural to thread row masks through conditional branches.
- Type specialization: the planner selects specialized `ExecNode` variants (e.g. `LiteralF64` vs `LiteralAny`) based on inferred types.

## Why: async Provider

- External data sources (databases, APIs) are inherently async.
- `Provider` receives `mask` so only required rows trigger external work.
- The evaluator runtime is currently synchronous; bridging async Provider integration is a known issue (see `evaluator/README.md`).

## Source pointers

- Planner: `evaluator/src/planner/planner.rs`
- IR nodes: `evaluator/src/ir/nodes.rs`
- Evaluator runtime: `evaluator/src/runtime/evaluator.rs`
- Kernel registry: `evaluator/src/kernels/registry.rs`
- Core types: `evaluator/src/core/types.rs`
- Provider trait: `evaluator/src/core/provider.rs`
- Error model: `evaluator/src/core/types.rs` (EvalError, ProviderError, SimpleEvalError)

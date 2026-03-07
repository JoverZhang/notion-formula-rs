# evaluator

Row-batch formula evaluation runtime.

## Responsibility

`evaluator` resolves expression values for a batch of rows with provider-backed property access.

- `Evaluator`: executes expressions against `RowBatch` + `EvalContext`
- `Provider`: async external value source for `prop("...")`
- `EvalBlock`: row-level result (`values`, `ok`, `errors`)

## Architecture

```
Expr (AST)  -->  Planner  -->  ExecPlan (IR)  -->  Evaluator  -->  EvalBlock
                   |                                    |
            infer_expr_with_map              dispatch via kernel registry
            (from analyzer)
```

1. **Planner** receives a parsed `Expr` and an `EvalContext`. It runs the analyser's
   type inference (`infer_expr_with_map`) to produce a `TypeMap`, then lowers the
   AST into an `ExecPlan` -- a tree of `ExecNode` variants.
2. **Evaluator** walks the `ExecPlan` recursively, dispatching binary operations
   through the **kernel registry** (a static lookup table keyed by `BinaryExecKey`).
3. Results are collected into an `EvalBlock` per expression: a `ColumnBlock` of
   values, a `Mask` of ok/fail flags, and a `Vec<(usize, EvalError)>` of row-level
   errors.

## Core contracts

- `Value` does not carry errors.
- Row failures are externalised via `EvalBlock.ok` and `EvalBlock.errors`.
- `ok[i] = false` means `values[i]` is placeholder-only (`Null`) and must not be consumed.
- Provider receives `Property` directly (`get_prop(&Property, ...)`).
- Branching/short-circuit paths pass `mask` so provider work is limited to required rows.

## Current runtime scope

### Implemented

- Literal evaluation: `number`, `string`, `boolean`, constant `list`
- Binary arithmetic: `+`, `-`, `*`, `/` (f64-specialised fast path)
- Polymorphic `+`: string concatenation, mixed-type coercion (number/text/list)
- Parenthesised grouping
- Row mask propagation and null propagation
- Cast `Any -> F64` with row-level error tracking
- Divide-by-zero detection (row-level `EvalError::DivideByZero`)

### Planned (not yet implemented)

- `prop("Name")` -- `Provider` trait is defined but not wired into evaluation
- `if(cond, then, else)` -- conditional with mask-driven branch evaluation
- Logical `&&` / `||` -- short-circuit with mask propagation
- Comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Unary operators (`-x`, `not x`)
- `%` (modulo), `^` (exponentiation)
- Builtin function calls
- Date operations

Unsupported expressions currently return row-level errors instead of panicking.

## IR: `ExecNode` variants

| Variant | Description |
|---|---|
| `LiteralF64(f64)` | Broadcast a number across all active rows |
| `LiteralAny(Value)` | Broadcast any value across all active rows |
| `CastToF64 { input }` | Convert `Column::Any` to `Column::F64` with error tracking |
| `Binary { key, left, right }` | Dispatch to a registered kernel |

## Kernel registry

Static lookup table (`LazyLock<BinaryKernelRegistry>`) indexed by `BinaryExecKey`:

| Key | Operation | Path |
|---|---|---|
| `AddF64` | f64 addition | f64 fast path |
| `AddAny` | Polymorphic addition (string concat, mixed coercion) | Any path |
| `SubF64` | f64 subtraction | f64 fast path |
| `MulF64` | f64 multiplication | f64 fast path |
| `DivF64` | f64 division (with divide-by-zero) | f64 fast path |

To add a new binary operation: add a `BinaryExecKey` variant, implement
`prepare_*` / `exec_*` functions, and register them in `BINARY_KERNEL_REGISTRY`.

## Error model

- `ProviderError`: batch-level external failures (`NotFound`, `BackendError`, `Timeout`)
- `EvalError`: row-level semantic failures (`TypeMismatch`, `DivideByZero`, `PropertyDisabled`, etc.)
- `SimpleEvalError`: convenience wrapper for `eval_simple_fail_batch`

## Known issues

- The planner constructs a `SemaContext` with an empty functions list; function
  call type inference will produce `Unknown` until `builtins_functions()` is wired in.
- The `Provider` trait is async but the evaluator runtime is synchronous; integrating
  `prop(...)` will require bridging this gap.
- The planner re-runs full type inference on every `build()` call; accepting a
  pre-computed `TypeMap` would avoid duplicate work when the caller has already
  run analysis.

## Testing

From workspace root:

```bash
just typecheck
just check
just verify
```

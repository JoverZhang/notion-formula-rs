# evaluator

Row-batch formula evaluation runtime.

## Responsibility

`evaluator` resolves expression values for a batch of rows with provider-backed property access.

- `Evaluator`: executes expressions against `RowBatch` + `EvalContext`
- `Provider`: external value source for `prop("...")`
- `EvalBlock`: row-level result (`values`, `ok`, `errors`)

## Core contracts

- `Value` does not carry errors.
- Row failures are externalized via `EvalBlock.ok` and `EvalBlock.errors`.
- `ok[i] = false` means `values[i]` is placeholder-only (`Null`) and must not be consumed.
- Provider receives `Property` directly (`get_prop(&Property, ...)`).
- Branching/short-circuit paths pass `mask` so provider work is limited to required rows.

## Current runtime scope

- `prop("Name")`
- `if(cond, then, else)`
- Logical `&&` / `||`
- Literal support for `boolean` / `number` / `string`

Unsupported expressions currently return row-level errors instead of panicking.

## Error model

- `ProviderError`: batch-level external failures (`NotFound`, `BackendError`, `Timeout`)
- `EvalError`: row-level semantic failures (`TypeMismatch`, `PropertyDisabled`, etc.)
- `SimpleEvalError`: convenience wrapper for `eval_simple_fail_batch`

## Testing

From workspace root:

```bash
just typecheck
just check
just verify
```

# evaluator

Synchronous row-batch formula evaluator over caller-prepared inputs.

Design rationale: [`docs/design/evaluator.md`](../docs/design/evaluator.md) and
[`docs/design/builtin-fn.md`](../docs/design/builtin-fn.md). Cross-crate rules:
[`docs/design/contracts.md`](../docs/design/contracts.md).

## Evaluation boundary

```text
formula + EvalContext
        |
        v
prepare_formula -- semantic analysis --> PreparedFormula
        |                                  |
        |                                  +--> required_columns()
        |                                             |
        |                              caller loads complete columns
        |                                             |
        +-------------------- EvalInputsBuilder::finish
                                                      |
RowBatch + EvalInputs + optional Mask ----------------+
        |
        v
PreparedFormula::evaluate / evaluate_with_mask --> EvalBlock
```

`PreparedFormula::required_columns()` is complete, deduplicated, and ordered by first
appearance, including property references in lazy branches. Callers may load those columns
asynchronously, but all loading finishes before evaluation. The evaluator contains no
Provider, Future, `block_on`, or runtime string lookup for properties.

`EvalInputsBuilder::finish` validates the prepared layout, plan-local slot, physical ABI,
batch length, and validity length. It returns one of the five `InputContractError` classes
before any kernel runs: missing, duplicate, wrong kind, wrong length, or wrong layout.

## Runtime data

- `RowBatch` owns the row IDs used by `id()`.
- `BuiltinRuntimeContext` is an immutable snapshot of evaluation time and timezone. Every
  row and kernel in one evaluation observes the same `now()`/`today()` inputs.
- `EvalBlock` keeps execution selection external in `Mask`, row success in `ok`, and nulls
  in `Validity`. An inactive row remains successful and non-null; only a row error sets
  `ok = false`.

## Generated and handwritten boundary

`evaluator/build.rs` deterministically consumes the supported builtin catalog and generates
one marker, implementation trait, named typed `Args` or `Plans`, repeat-group type, metadata
entry, and exhaustive dispatch binding per builtin. Missing implementations and mismatched
method signatures therefore fail compilation. Generated argument and plan types own their
handles and have no explicit lifetime parameters. Value dispatch is generic over the
read-only `BuiltinValueContext`, so a borrowed runtime façade does not leak its lifetime
into generated traits; controlled dispatch is generic over the concrete
`BuiltinEvalContext` rather than `dyn`.

Value builtins receive already materialized typed columns. Controlled builtins receive typed
plan handles and choose evaluation order and row masks through the synchronous context;
`if`, `ifs`, `let`, and lambda list functions stay lazy. The Planner consumes Analyzer's
final `ResolvedFunctionSig`, lowers its projected head/repeat/tail structure, and inserts
explicit ABI cast nodes where a generic `AnyKind` producer feeds a concrete consumer. The
generated wrapper only moves matching shared column handles and never copies rows.

Debug builds use the resolved call contract to check active successful input and output
types, physical shape, error rows, mask membership, `ok`, and validity invariants. The same
helpers are available to handwritten kernels.

## Column ownership

`KernelColumn<K>` hides a reference-counted `SharedStorage` buffer and independent shared
validity bitmap. Cloning a column for plan fan-out clones only handles. A kernel can call
`try_into_unique()` to recover the owned buffer and compute in place when no other handle
exists; otherwise it computes into new storage. Generated dispatch moves existing handles
without copying row values.

## Module map

| Path | Responsibility |
| --- | --- |
| `src/core/` | columns, masks, runtime snapshots, rows, inputs, and errors |
| `src/planner/` | semantic handoff, input manifest, ABI adapters, and `PreparedFormula` |
| `src/ir/` | owned arena plan and typed controlled-plan metadata |
| `src/runtime/` | synchronous masked IR execution and operators |
| `src/builtins/` | generated contract inclusion, typed dispatch support, and all impl obligations |
| `src/kernels/` | handwritten Value/Controlled behavior and compute helpers |
| `build_support.rs` | deterministic catalog-to-Rust contract generator shared by build and tests |

## Testing

Permanent behavior coverage is bounded to `flat`, `concat`, `splice`, `ifs`, and synthetic
`caseOf`; mechanical tests cover the full catalog and implementation bindings. Additional
tests cover input contract failures, lazy masks and error isolation, state independence,
runtime snapshots and row IDs, shared fan-out, unique in-place storage, deterministic
generation, and compile-fail implementation contracts.

```bash
cargo check -p evaluator
cargo test -p evaluator
```

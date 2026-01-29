# Signature Help (ParamShape) Spec

This document defines the deterministic Signature Help algorithm for functions whose parameters are modeled as:

`ParamShape { head, repeat, tail }`

It is intended as a concise, code-backed spec for editor integrations (labels + `activeParam`).

## Invariants (for determinism)

- `repeat` parameters are never optional.
- If `repeat.len() > 0` and `tail.len() > 0`, then all `tail` parameters are required (non-optional).  
  (Optional tail under repeat is ambiguous, so it is rejected at builtin construction time.)
- The rendered `...` token is never the active parameter highlight.

## Shape parsing (`repeat` + optional `tail`)

For a shape with `repeat.len() > 0`, define:

- `head_len = head.len()`
- `repeat_len = repeat.len()`
- `repeat_min_groups = 1` (this repo’s repeat-group signatures require at least one group)
- `tail_used` = how many tail params are “in use” at the end of the call

### `resolve_repeat_tail_used(params, total_args) -> Option<tail_used>`

Select a deterministic `tail_used` such that:

- `total_args >= head_len + tail_used`
- `middle = total_args - head_len - tail_used`
- `middle >= repeat_len * repeat_min_groups`
- `middle % repeat_len == 0`

If multiple `tail_used` satisfy the constraints (only possible when `tail` contains optional suffix params), choose the **largest** `tail_used` (prefer including optional tail when possible). If no solution exists, return `None`.

### Completable-shape rule (for Signature Help)

Signature Help must be stable even for calls whose `total_args` are not parseable.

If `resolve_repeat_tail_used(params, total_args)` returns `None`, compute the smallest `total' >= total_args` such that `resolve_repeat_tail_used(params, total')` succeeds. Use `total'` only for:

- mapping `arg_index -> activeParam`
- deciding whether the call has entered the tail
- deciding how many repeat groups have been entered

Do **not** invent argument types for arguments that do not exist in source.

## Display + mapping rules

### Type display (per parameter slot)

For each displayed parameter slot, pick the type to render using:

1) the best-effort inferred *actual* type of the corresponding call-site argument **if the argument expression is non-empty**
2) else the instantiated expected type (after generic unification/substitution)
3) else `unknown`

Return type: the instantiated return type.

### Repeat rendering

For repeat-group signatures, the UI display order is:

- `[head...]`, then
- repeat group #1 (numbered names, e.g. `condition1`, `value1`)
- repeat group #2 (numbered names, e.g. `condition2`, `value2`) **only if entered**
- `...`
- `[tail...]`

“Entered” is based on the parseable/completed total: `repeat_groups = (total' - head_len - tail_used) / repeat_len`.

### Mapping `call_ctx.arg_index -> activeParam`

Let `total_args` for mapping be:

- `total_args = max(actual_arg_slots, call_ctx.arg_index + 1)`  
  (the current cursor argument counts as present even if empty)

Compute the completed parseable total `total'` as described above.

Then:

- If the argument index maps into `head`, highlight the corresponding head slot.
- If it maps into `tail` (per `total'`), highlight the corresponding tail slot.
- Otherwise it maps into `repeat`:
  - compute `(cycle, pos)` within the repeat group
  - if `cycle >= 2`, clamp to cycle #2 but preserve `pos` within the group
- Never highlight `...`.

## Canonical examples

### SUM (variadic `number` only)

NOTE: Array arguments (`number[]`) are deferred until the language has list literals (or an equivalent array expression).  
TODO: restore `number[]` support once arrays exist.

1) `sum($0)`
   - label: `sum(values1: number, ...) -> number`
   - `activeParam`: `0`

2) `sum(42$0)`
   - label: `sum(values1: number, ...) -> number`
   - `activeParam`: `0`

3) `sum(42, $0)`
   - label: `sum(values1: number, values2: number, ...) -> number`
   - `activeParam`: `1`

4) `sum(42, 42$0)`
   - label: `sum(values1: number, values2: number, ...) -> number`
   - `activeParam`: `1`

### IF

- `if(true, "123", 123$0)`
  - label: `if(condition: boolean, then: string, else: number) -> number | string`
  - `activeParam`: `2`

- `if(true, x, 1$0)`
  - label: `if(condition: boolean, then: unknown, else: number) -> unknown`
  - `activeParam`: `2`

### IFS (head=0, repeat=(condition,value), tail=(default), repeat_min_groups=1)

- `ifs(true, "42", $0)`
  - `activeParam`: `3` (default)

- `ifs(true, "42", false, $0)` (invalid total=4)
  - guides completion toward `value2`
  - `activeParam`: `3`

- `ifs(true, "42", false, 7, $0)` (total=5)
  - `activeParam`: `5` (default)

## Postfix form (presentation-only)

Member-call syntax is treated as a normal call internally:

- `receiver.fn(arg1, ...)` is analyzed as `fn(receiver, arg1, ...)` for:
  - semantic inference (types + diagnostics)
  - signature help instantiation
  - active parameter mapping

Signature help output is then transformed only for rendering:

- `signature_help.receiver` is the first formatted parameter slot
- `signature_help.params` is the remaining slots
- `signature_help.label` prints only `params` (return type unchanged)
- `signature_help.active_param` is the “full call” active index shifted by `-1` (clamped), and never highlights `...`

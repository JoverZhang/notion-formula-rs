---
doc_id: architecture.contracts
title: "Which cross-crate invariants may callers rely on?"
language: en
source_language: en
counterpart: ./contracts.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# Cross-crate Contracts

[简体中文](contracts.zh-CN.md)

This Current document answers which invariants Rust and JavaScript callers may rely on when
formula data crosses `analyzer`, `ide`, `analyzer_wasm`, and `evaluator`. It is for maintainers and
integrators who need to change or debug those boundaries without accidentally changing observable
behavior.

The scope begins with source coordinates, tokens, and recovered syntax, then follows diagnostics,
edits, signature help, the WASM facade, and prepared evaluator inputs. Individual builtin semantics
and module-internal APIs are outside this document. A rule here is a compatibility surface: changing
one requires matching tests, documentation in both languages, and a changelog entry. Moving an
internal helper without changing the rule does not.

## Boundary map

| Boundary | Caller supplies | Caller receives | Failure scope |
| --- | --- | --- | --- |
| `analyzer` | UTF-8 formula source | Tokens, a recoverable AST, diagnostics, and byte spans | Problems become diagnostics; recoverable syntax still produces an AST |
| `ide` edit operations | Source plus byte-coordinate cursor or edits | Updated source and rebased byte cursor | Invalid input or formatting failure rejects the whole operation |
| `analyzer_wasm` | JS config, source, and UTF-16 editor coordinates | DTOs whose spans, edits, and cursors use UTF-16 | Constructor and edit-operation errors cross the WASM boundary as errors |
| `evaluator` | A parsed expression, schema, row batch, and complete typed inputs | `EvalBlock` with values, validity, row success, and row errors | Preparation and input errors reject the whole operation; evaluation errors stay row-local |

The table is an ownership map, not another pipeline description. The following sections define the
coordinate conversions, recovery guarantees, and failure boundaries that make those handoffs safe.

## Coordinates change only at the WASM boundary

Inside the Rust analyzer and IDE layers, every source [`Span`](../../analyzer/src/span.rs),
[`TextEdit.range`](../../analyzer/src/text_edit.rs), and cursor is measured in UTF-8 bytes. Ranges are
half-open `[start, end)`, and both endpoints must be valid character boundaries in the same source
string. A valid `Span` can therefore slice its source directly as `&source[start..end]`.

[`SourceMap::line_col`](../../analyzer/src/source_map.rs) is a display location, not a second
source-offset system. It returns a 1-based line and a 1-based column counted in Unicode scalar values
(Rust `char`s), not bytes or UTF-16 code units. `Diagnostic.line` and `Diagnostic.col` are derived
this way while converting diagnostics for WASM.

At the JavaScript boundary, DTO spans, edit ranges, and cursors use half-open UTF-16 code-unit
coordinates. `analyzer_wasm` owns all conversion between those coordinates and Rust byte offsets;
neither `analyzer` nor `ide` performs UTF-16 conversion. Callers must not use DTO spans to slice a
UTF-8 string or pass UTF-16 positions directly to a Rust API.

Conversion has deterministic boundary behavior:

- A position inside a multi-unit Unicode scalar is floored to that scalar's start.
- General conversion helpers, including the conversion used by `help`, clamp positions past the end
  of the source.
- `format` and `apply_edits` use checked cursor conversion and reject a cursor past the UTF-16
  document length. Edit conversion also rejects reversed ranges and ranges whose end is past the
  document; an endpoint inside a surrogate pair is still floored to the scalar start.
- Core IDE edit operations reject byte cursors or edit endpoints that are out of bounds or are not
  UTF-8 character boundaries.

These rules keep conversion panic-free, but they do not make the coordinate systems interchangeable.
Flooring and clamping are covered by unit tests beside the
[offset converters](../../analyzer_wasm/src/offsets.rs); operation failures are covered by the
[WASM boundary tests](../../analyzer_wasm/tests/analyze.rs).

## Tokens and syntax remain usable after local errors

The lexer emits tokens in source order, retains `DocComment` and `Newline` as trivia, and appends one
explicit `Eof` token with an empty span at the end of the source. Ordinary spaces are not tokens.
Token ranges are also half-open, but their unit is a token index rather than a byte.

[`tokens_in_span`](../../analyzer/src/lexer/token.rs) maps a non-empty byte span to every token whose
span intersects it. Because `Eof` has an empty span, a non-empty source span does not include it. An
empty or reversed span maps to the stable insertion point before the first token whose start is at or
after `span.start`; that insertion point may be the `Eof` index.
[`TokenQuery`](../../analyzer/src/parser/tokenstream.rs) is the canonical API for this mapping and for
trivia-aware neighbor scans, so parser and formatter code should not duplicate token-index arithmetic.

The parser recovers from local syntax errors by inserting
[`ExprKind::Error`](../../analyzer/src/parser/ast.rs) nodes and continuing. AST consumers must
therefore handle `Error` nodes instead of assuming that diagnostics imply no tree. They must also
account for `ExprKind::ImplicitLambda`: the parser never creates it, but semantic analysis may insert
it for function-typed arguments. Member syntax has one supported form, `receiver.method(...)`; bare
member access such as `receiver.member` is diagnosed and recovered as an error expression.
Representative span-mapping and recovery cases live in
[`test_tokens_in_span.rs`](../../analyzer/src/tests/lexer/test_tokens_in_span.rs) and
[`test_parser_spans.rs`](../../analyzer/src/tests/parser/test_parser_spans.rs).

## Diagnostics and edits are deterministic

[`Diagnostics`](../../analyzer/src/diagnostics.rs) retains at most one diagnostic for an exact span.
An incoming higher-priority diagnostic replaces the existing one. At equal priority, an identical
message merges and deduplicates labels, notes, and code actions; a different message leaves the
existing diagnostic unchanged.

`format_diagnostics` orders diagnostics by start, end, descending priority, and message. It also
sorts labels by start, end, and label message; notes retain their deduplicated emission order. Code
actions remain attached to their diagnostic as `Diagnostic.actions: Vec<CodeAction>`, and every action
contains core byte-coordinate `TextEdit` values. The WASM converter preserves the action structure
while converting its edit ranges to UTF-16.

Public edit operations are all-or-nothing:

- [`ide::apply_edits`](../../ide/src/edit.rs) sorts edits in original-source coordinates, validates
  every cursor and range, rejects overlaps, then applies the complete set and rebases the cursor.
- `ide::format` rejects source with lexer or parser errors. Otherwise it creates one full-document
  replacement and uses the same validated byte-edit pipeline.
- WASM `format` and `apply_edits` convert inputs to bytes, call the IDE operation, and convert the
  returned cursor to UTF-16. A conversion or IDE failure is returned as an operation error, not as a
  partial `ApplyResult`.

Overlap rejection, cursor rebasing, and format failures are exercised in
[`test_edit_ops.rs`](../../ide/src/tests/ide/test_edit_ops.rs).

## Editor help shares the semantic signature model

[`ParamShape`](../../builtin_fn/src/signature.rs) in `builtin_fn` is the canonical parameter model.
Its shape consists of `head`, a repeat group, `tail`, and `repeat_min_groups`; consumers must use the
shared call-signature projection rather than re-derive variadic or tail positions.

Signature help resolves that shared semantic model and returns structured `DisplaySegment` values
plus `active_parameter`. The WASM DTO mirrors those segments instead of flattening them to one display
string, leaving final rendering to the caller. The detailed projection, active-parameter, and postfix
presentation rules live in the [Signature Help specification](../signature-help.md); the broader
declaration and resolution model lives in [Builtin Function Design](builtin-fn.md).

## The WASM facade owns its configuration boundary

[`Analyzer::new`](../../analyzer_wasm/src/lib.rs) accepts an object and rejects unknown top-level
keys. The only accepted keys are `properties` and `preferred_limit`; JavaScript cannot supply
`functions`, because the constructor always installs the Rust builtin catalog.

Runtime deserialization accepts an omitted `properties` as an empty list and an omitted or `null`
`preferred_limit` as the default `5`. The generated TypeScript DTO currently declares both fields
explicitly as `{ properties: Property[], preferred_limit: number | null }`, so typed integrations
should pass both fields even though the runtime accepts omission. Invalid object shape or field values
fail construction with `Invalid analyzer config`. Constructor rejection and `null` defaulting are
exercised by the [WASM tests](../../analyzer_wasm/tests/analyze.rs); the stricter generated shape is
visible in [`wasm_dto.ts`](../../examples/vite/src/analyzer/generated/wasm_dto.ts).

## Evaluation starts after the caller completes the input contract

[`prepare_formula`](../../evaluator/src/planner/prepared.rs) runs semantic analysis and lowers the
expression to an owned execution plan. A preparation failure returns `PrepareError` before a
`PreparedFormula` exists. On success, `PreparedFormula::required_columns()` returns the complete,
deduplicated required-column manifest in first-seen order. Each `RequiredColumn` carries the property
name, expected type, and an `InputSlot` that is valid only for that prepared input layout; the method
does not return bare `InputSlot` values.

The caller must load every required column, including columns referenced only by an unselected
branch, before synchronous evaluation begins. `EvalInputsBuilder::finish` validates missing or
duplicate columns, slot layout, ABI kind, batch length, and validity length. A structural mismatch
returns `InputContractError`; evaluation also rejects inputs, masks, or row batches from incompatible
layouts or lengths. No kernel result is produced for these whole-operation failures.

During evaluation, three row states remain independent:

- the execution mask says whether a control-flow step should run for a row;
- `EvalBlock.ok` records whether that row evaluated successfully; and
- column `Validity` records whether a successful row contains a non-null value.

When `ok[i]` is false, the physical value at that row is a placeholder and downstream kernels must
not consume it. Its `EvalError` remains row-local, so other rows can complete. Mask-driven control
flow such as `if`, `ifs`, `&&`, `||`, and lambda builtins evaluates branch or argument plans only for
the rows that require them, even though all referenced input columns were prepared up front.

The full ownership rationale, IR design, null semantics, and evaluator failure table live in
[Evaluator Design](evaluator.md). The required-column manifest and input error classes are exercised
through the public evaluator seam in
[`runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs).

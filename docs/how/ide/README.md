---
doc_id: how.ide
title: "How editor help and edits flow through the IDE crate"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How editor help and edits flow through the IDE crate

[简体中文](README.zh-CN.md)

The `ide` crate turns a formula source string, a byte cursor, and Analyzer semantic context into
completion and signature-help data. It also formats formulas and applies text edits while rebasing
the cursor. This guide explains the Current Rust implementation for maintainers who need to extend
or debug those mechanisms.

The crate owns editor orchestration, presentation policy, formatting, and byte-edit application. It
does not own formula semantics, builtin parameter projection, UTF-16 conversion, or the observable
editor-service contract. Those belong to `analyzer`, `builtin_fn`, `analyzer_wasm`, and
`docs/specs/` respectively. The exact client-facing results remain owned by the editor-services
specification; the passes below describe how the current implementation produces them, not a second
compatibility contract.

## One help session coordinates two independent results

[`ide::help`](../../../ide/src/lib.rs) creates a `HelpSession` and lexes and parses the source through
`analyzer::analyze_syntax`. The session retains the returned tokens, then runs a fixed sequence:

```text
source + UTF-8 byte cursor + semantic::Context
                       |
                       v
              detect_cursor_context
                 /             \
                v               v
       signature help        candidates
                |               |
                |          type ranking
                |               |
                |        attach primary edits
                |               |
                |   query ranking + preferred picks
                |               |
                +-------+-------+
                        v
                    HelpResult
```

Read the diagram from the inputs at the top to the combined `HelpResult`. Both branches use the same
token snapshot and call context, but either branch can be empty. The diagram omits the additional
best-effort parsing used to infer receiver and argument types.

`HelpResult` deliberately keeps `CompletionResult` and `Option<SignatureHelp>` separate. Completion
can be suppressed inside a string while signature help remains available for the containing call.
Conversely, an unknown callee removes signature help without preventing position-based completion.
The orchestration and this independence are covered by
[`test_completion_position.rs`](../../../ide/src/tests/ide/test_completion_position.rs) and
[`test_edit_ops.rs`](../../../ide/src/tests/ide/test_edit_ops.rs).

Internally, `ide` consumes Analyzer `Span` and `TextEdit` values in UTF-8 byte coordinates. `help`
does not have an error return: a cursor that does not fit in `u32` saturates to `u32::MAX`, while
helpers that need to slice source text decline invalid boundaries. Checked edit operations use the
stricter validation path described below. `analyzer_wasm`, not this crate, converts editor-facing
UTF-16 positions.

## Cursor context turns partial syntax into editor intent

[`detect_cursor_context`](../../../ide/src/context.rs) derives four values from tokens around the
cursor: the innermost call, a coarse `PositionKind`, a replacement span, and an optional normalized
query.

Call detection keeps a stack of open parentheses before the cursor. The innermost unmatched `(` is
a call only when its preceding non-trivia token is an identifier; the detector does not fall back to
an outer call while the cursor is inside a still-open grouping parenthesis. Once a call has been
selected, commas increment `arg_index` only at the top level of its argument list. Nested parentheses
and brackets do not increment that selected call's index. This token-based path continues to work
for many incomplete calls without requiring a complete AST.

Position detection classifies the nearby token sequence as:

- `NeedExpr` at an expression start or while extending an identifier;
- `AfterAtom` after an identifier, literal, or closing parenthesis;
- `AfterDot` after an identifier, literal, or closing parenthesis followed by `.` and an optional
  method prefix;
- `None` when no supported completion position is recognized.

A cursor strictly inside a string literal forces `None`, but does not discard the previously
detected call context. For `NeedExpr` and `AfterDot`, the replacement span covers the identifier
being edited when possible; other positions insert at an empty span. A query is produced only when
that span contains ASCII letters or digits, underscores, or whitespace. Query normalization removes
underscores and whitespace and folds ASCII case. Non-ASCII or punctuation-bearing replacement text
therefore skips query ranking rather than risking an invalid slice or misleading match.

## Completion is candidate generation followed by policy

The `PositionKind` selects one candidate source in
[`completion/items.rs`](../../../ide/src/completion/items.rs):

- `NeedExpr` offers configured properties, `not`/`true`/`false`, and every function in
  `semantic::Context`. A property inserts `prop("Name")`; a function inserts its call parentheses.
- `AfterAtom` offers the fixed operator subset `==`, `!=`, `>=`, `>`, `<=`, `<`, `+`, `-`, `*`,
  and `/`, plus postfix-capable functions with the leading dot. Parser-supported `%`, `^`, `&&`,
  and `||` are not completion candidates here.
- `AfterDot` offers only postfix-capable functions and omits the dot from inserted text because it is
  already present in the source. The current token gate does not recognize a closing bracket as a
  completion receiver, so a list literal does not enter this branch.
- `None` produces no candidates.

Postfix completion is gated by Analyzer's shared postfix-capable set and the signatures present in
the supplied `Context`; `ide` does not keep a second allowlist. After a dot,
`HelpSession::infer_postfix_receiver_ty` reparses the source prefix before the dot and asks Analyzer
to infer the receiver type. A known type filters functions whose first parameter cannot accept it.
If parsing or inference leaves the receiver `Unknown`, the current implementation keeps the full
postfix-capable set as a best-effort fallback.

Properties with `disabled_reason` remain visible so a UI can explain why they are unavailable, but
[`attach_primary_edits`](../../../ide/src/completion/ranking.rs) gives them neither a primary edit nor
a requested cursor. Enabled function and postfix items request a cursor just after the inserted
opening parenthesis; property items request the end of the inserted `prop(...)` expression.

When `NeedExpr` is also an active call argument, `expected_call_arg_ty` obtains a best-effort
declared parameter type from the function signature. A concrete expected type reorders completion
categories and candidates by type compatibility; a top-level `Unknown` or `Generic(_)` skips that
pass. Incompatible candidates are ranked lower rather than removed, which keeps completion usable
while the formula is incomplete.

If a query exists, [`rank_by_query`](../../../ide/src/completion/ranking.rs) then orders function and
property labels by exact match, substring match, and fuzzy subsequence quality. Ordinary expression
completion retains unmatched items. Member completion after a dot filters out unmatched methods. The
original candidate position remains the final tie-break, so a stable `Context` produces stable
output. `preferred_indices` selects up to `CompletionConfig::preferred_limit` enabled matching
functions and properties from the final list; it does not create a second candidate list.

## Signature help adapts the shared resolved signature

[`compute_signature_help_if_in_call`](../../../ide/src/signature/mod.rs) starts only when call context
exists, the cursor is positioned after the opening parenthesis, and the callee resolves in
`semantic::Context`. It returns `None` otherwise. The Current implementation produces at most one
signature candidate and sets `active_signature` to zero.

For a recognized call, the module performs four presentation-oriented steps:

1. It detects whether token connectivity forms `receiver.name(...)` and confirms that the resolved
   function is postfix-capable. Method form reserves the receiver as semantic argument zero.
2. It splits source after the opening parenthesis into argument fragments, ignoring commas inside
   nested parentheses and brackets. Each non-empty fragment is parsed and inferred independently;
   an empty fragment becomes `ArgumentObservation::Empty`, while an uncertain non-empty fragment can
   remain typed as `Unknown`. The observation list is extended through the cursor argument so an
   empty active slot is still represented. Receiver inference uses the smallest matching member-call
   expression recovered from the whole source.
3. It calls `analyzer::semantic::resolve_call_signature` with those observations. That shared
   resolver owns parameter-shape projection, generic binding, and return-type refinement. The IDE
   adapter treats its `ResolvedFunctionSig` as input and does not reimplement those rules.
4. It converts the resolved projection into `DisplaySegment` values and maps the cursor argument to
   an active rendered parameter.

The presentation adapter in
[`signature/render.rs`](../../../ide/src/signature/render.rs) shows the observed type for a declared
generic slot when one exists, including `Unknown`; without an observation it uses the resolved
expected type. For non-generic slots, an `Unknown` observation does not replace the expected type,
while a compatible union observation can narrow it. The adapter also unwraps function parameters to
their return type for display, marks an optional slot with a `?` suffix on its displayed type,
numbers projected repeat slots, and inserts one ellipsis.
These are rendering choices applied after shared resolution, not another parameter model.

For postfix calls, the first projected slot becomes a receiver prefix such as
`(condition: boolean).`; its `DisplaySegment::Param` has no `param_index` and cannot be active. The
remaining parameter segments receive contiguous display indices. Active-parameter mapping finds the
projected slot whose `argument_index` matches the cursor, subtracts the receiver slot in method form,
and falls back to the last rendered parameter when incomplete input has no direct mapping. Ellipsis
segments are never counted as active parameters. The focused cases in
[`test_completion_signature_help.rs`](../../../ide/src/tests/ide/test_completion_signature_help.rs)
cover nested commas, empty arguments, generic and union display, repeated slots, postfix receivers,
and fallback mapping.

[`build_signature_segments`](../../../ide/src/display.rs) keeps names, punctuation, separators,
parameters, ellipsis, arrow, and return type as structured segments. The crate does not flatten them
into a UI string or choose colors and typography; downstream adapters own final rendering.

## Formatting combines the AST with original trivia

[`ide::format`](../../../ide/src/lib.rs) delegates to `ide_format` in
[`edit.rs`](../../../ide/src/edit.rs). It first calls `analyzer::analyze_syntax`. Any lexer or parser
diagnostic returns `IdeError::FormatError`; semantic diagnostics are not part of this operation. On
success, the formatter emits one replacement covering the full source and sends it through the same
validated edit path as caller-supplied edits.

[`Formatter`](../../../ide/src/format.rs) renders the recovered AST while consulting the original
token stream through `TokenQuery`. AST structure supplies precedence and expression nesting;
original trivia supplies line and block comments. `used_comments` prevents a comment from attaching
twice. Inline-layout attempts snapshot that set and roll it back when the expression must become
multiline, so a failed compact layout cannot consume comments.

The formatter centralizes indentation and width policy in the `INDENT` and `MAX_WIDTH` constants.
Existing multiline expressions normally remain on a multiline path; binary expressions with a
trailing line comment may retry inline layout. Compound calls, lists, groups, operators, ternaries,
and member calls remain inline only when their recursive inline attempt fits the width. Atomic
identifiers and literals bypass that width check and can therefore form a line longer than
`MAX_WIDTH`. Golden snapshots under
[`ide/tests/format`](../../../ide/tests/format) exercise comment placement and multiline layouts;
unit tests in
[`test_format_idempotence.rs`](../../../ide/src/tests/ide/test_format_idempotence.rs) require a
second formatting pass to produce identical text.

## Edit application separates validation from mutation

Analyzer parser recovery creates diagnostic `CodeAction` values and their byte-range `TextEdit`
lists. The IDE neither chooses nor generates those actions. Once a caller selects an action, its
edits enter the same [`apply_edits`](../../../ide/src/edit.rs) path as any other caller-supplied edit.

`apply_edits` stably sorts edits by original-source `(start, end)` and passes the whole vector through
`validate_cursor` and `validate_sorted_non_overlapping_edits` before constructing updated text. The
validators check source bounds, UTF-8 character boundaries, and range direction. An edit overlaps
only when its start is before the previous sorted end, so adjacent ranges and zero-width insertions
at the same position remain valid. For tied zero-width insertions, stable sorting and reverse
application preserve caller order. Failures map to `IdeError` variants before mutation begins.

After validation, [`apply_text_edits_bytes_with_cursor`](../../../ide/src/text_edit.rs) traverses
edits from the end of the source toward the beginning so original coordinates remain valid. During
that traversal it shifts a cursor after an edit by the byte-length delta and anchors a cursor inside
a replacement at that replacement's start. The helper builds one final source string and cursor;
there is no intermediate result returned to the caller.

## Tests follow the seams in the implementation

The in-crate IDE tests use the `$0` marker and builders in
[`completion_dsl.rs`](../../../ide/src/tests/ide/completion_dsl.rs) to keep source, cursor, semantic
context, application, and expected items in one scenario. The suites are separated by the seams
maintainers normally change:

- `test_completion_position.rs` checks call and position detection, replacement spans, incomplete
  input, and string suppression;
- `test_completion_ranking.rs` checks query filtering, fuzzy and type ordering, receiver filtering,
  and preferred indices;
- `test_completion_signature_help.rs` checks the adapter from observed arguments to structured
  signatures and active parameters;
- `test_edit_ops.rs` checks overlap rejection, cursor movement through a valid edit, syntax-error
  rejection during formatting, and the combined help result;
- `test_format_idempotence.rs` checks formatter stability.

The current focused edit suite does not directly cover every `InvalidCursor` or
`InvalidEditRange` branch. A change to those validators should add boundary cases rather than rely
only on the overlap test.

The integration test [`format_golden.rs`](../../../ide/tests/format_golden.rs) runs every
`*.formula` fixture against its adjacent `*.snap` result in sorted order. A formatter change should
update those snapshots only after reviewing the source/output pairs. Run the complete crate suite
with:

```bash
cargo test -p ide
```

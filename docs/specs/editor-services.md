---
doc_id: specs.editor-services
title: "What editor-service behavior can integrations rely on?"
language: en
source_language: en
counterpart: ./editor-services.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Editor services

[简体中文](editor-services.zh-CN.md)

This Current specification defines the observable diagnostics, completion, signature-help,
formatting, and edit behavior that formula authors and editor integrations may rely on. These
services remain useful while a formula is incomplete, but their best-effort results do not make
malformed source valid for evaluation.

This document defines service behavior, not transport. The WASM API specification owns exported
method names, DTO fields, UTF-16 conversion, boundary validation, serialization, and JavaScript
error delivery. Formula syntax and evaluation belong to the formula-language and builtin-function
specifications. Completion popovers, grouping, focus, and quick-fix selection in the Vite example
are example policy rather than editor-service guarantees.

## Consume diagnostics in the order they are produced

An analysis request can report syntax and semantic problems together. The returned sequence is
formed in three phases:

1. parser diagnostics;
2. lexer diagnostics;
3. semantic diagnostics.

The service preserves that phase concatenation. It does not globally sort diagnostics by source
position or globally deconflict diagnostics emitted by different phases. A phase may coalesce its
own same-span reports, so consumers must preserve the returned order without assuming that it is a
source-order sort. [`analyzer::analyze`](../../analyzer/src/lib.rs) is the implementation anchor for
this ordering.

Some parser-recovery diagnostics include actions that insert, replace, or remove source text. An
action is optional: not every parser diagnostic has one, and lexer and semantic diagnostics
currently have no actions. Action edits use original-document positions and follow the common edit
rules below. Clients choose whether and when to offer or apply them; the service never applies an
action as part of analysis. Representative recovery actions are covered by
[`test_errors.rs`](../../analyzer/src/tests/parser/test_errors.rs).

Diagnostic messages are descriptive, human-facing prose. Their exact English wording is not a
machine-readable compatibility key, and integrations must not branch on a complete message string.
Transport-level diagnostic fields are defined by the WASM API specification.

## Request completion according to cursor context

Completion candidates depend on the source immediately around the cursor and the configured
properties and supported functions:

| Cursor context | Current candidates |
| --- | --- |
| At an expression start, including an empty argument | configured properties, `not`, `true`, `false`, and supported functions |
| After an identifier, literal, or `)` | `==`, `!=`, `>=`, `>`, `<=`, `<`, `+`, `-`, `*`, `/`, and postfix-capable functions |
| After a receiver and `.` | postfix-capable functions accepted by the known receiver type |
| Strictly inside a string literal | no completion candidates |

For a receiver whose type is unknown, member completion currently keeps the full postfix-capable
set. For a known receiver type, it removes functions whose receiver parameter cannot accept that
type. A query after `.` further removes nonmatching functions. These boundaries are exercised in
[`test_completion_position.rs`](../../ide/src/tests/ide/test_completion_position.rs) and
[`test_completion_ranking.rs`](../../ide/src/tests/ide/test_completion_ranking.rs).

The parser also accepts `%`, `^`, `&&`, and `||`, but the Current after-atom completion set does not
offer them. Integrations must not infer the completion catalog from the grammar.

Enabled candidates carry an edit for the detected insertion or replacement range. Function and
postfix-function edits insert parentheses and place the requested cursor inside them. `not`,
`true`, and `false` insert a trailing space. A property candidate inserts `prop("Name")` and places
the cursor after the call.

Property names are inserted verbatim between the quotes. The Current completion service does not
escape a quotation mark, backslash, or other source-sensitive character in the configured name.
Such a candidate can therefore insert invalid formula source. Integrations must not treat property
completion as an escaping guarantee. The candidate construction is anchored in
[`completion/items.rs`](../../ide/src/completion/items.rs).

Completion and signature help are independent results. In particular, completion is empty while
the cursor is strictly inside a string, but signature help can remain available for the containing
known call.

## Treat ordering and preferred indices as deterministic hints

When an expression-start cursor is inside a known call argument, candidates are first reordered by
the expected argument type. Compatible result types rank ahead of unknown or incompatible types;
this step reorders candidates but does not remove them.

When the replacement text yields a query, matching ignores ASCII case and underscores. For
function and property labels, an exact match ranks before a containing match, which ranks before an
ordered-subsequence fuzzy match; deterministic compactness, candidate-kind, and original-order tie
breakers settle the remaining cases. Ordinary expression completion retains nonmatching candidates
after the matches. Member completion after `.` removes nonmatching candidates instead. Function
`()` and a postfix label's leading `.` are not part of the text used for matching. The Current
ordering rules are implemented in [`completion/ranking.rs`](../../ide/src/completion/ranking.rs) and
[`completion/matchers.rs`](../../ide/src/completion/matchers.rs).

`preferred_indices` are selection hints into the final, already ordered item list. They contain at
most the configured preferred limit, preserve final item order, and refer only to enabled function
or property candidates that match the query. No query, a zero limit, or no matching enabled
candidate produces an empty list. The indices do not form a second candidate list and do not
authorize a client to reorder the returned items.

## Show one best-effort signature for the active call

Signature help is available after the opening parenthesis of the innermost known function call
that contains the cursor. It is absent before the parenthesis, after the cursor has left the call,
or when the callee is unknown. A missing closing parenthesis does not suppress help, so partial
calls such as `if(` can still receive it.

The Current service returns one structured signature and selects it as the active signature. The
displayed parameter and return types incorporate best-effort types from arguments already present;
unknown or incomplete arguments can remain generic or `unknown`. A normal call has no receiver
prefix. A supported postfix call displays its receiver separately and excludes that receiver from
the visible parameter indices. The declaration and call-shape mechanisms that produce these slots
are outside this specification.

The active parameter follows the current top-level argument position. Commas inside nested calls
or lists do not advance it. Empty arguments in an incomplete call still select the slot being
edited. For repeated or otherwise projected call shapes, the service selects the displayed slot
corresponding to the current argument when one exists. Without a direct mapping it returns the
final displayed parameter index when at least one exists; a zero-parameter signature returns `0`
even though it has no displayed parameter. The mapped cases are covered by
[`test_completion_signature_help.rs`](../../ide/src/tests/ide/test_completion_signature_help.rs).
The no-mapping fallback has no dedicated regression test.

## Format only syntactically valid source

Formatting is a full-document, all-or-nothing operation. If lexing or parsing produced any
diagnostic, formatting fails and returns no partial formatted source. Semantic problems alone do
not block formatting because formatting does not perform semantic analysis.

For accepted source, formatting is deterministic and idempotent over the covered syntax. It:

- uses two spaces for each indentation level;
- inserts conventional spaces around binary and ternary operators and after commas;
- emits a single trailing newline;
- preserves comments through their syntax attachment; and
- selects an inline layout for compound constructs only when the construct permits it and
  indentation plus rendered byte length is at most 80 bytes, otherwise using its multiline layout.

Atomic identifiers and literals bypass the width check and can form a longer line. The 80-byte
threshold is otherwise a Current fixed layout rule, not configurable editor width. The formatter
returns the whole formatted document and rebases the supplied cursor through that replacement using
the edit rules below. Most cursors strictly inside a changed full-document range therefore move to
the start; a cursor at the document end remains at the end after length adjustment.
[`format.rs`](../../ide/src/format.rs), the
[`format` goldens](../../ide/tests/format/), and
[`test_format_idempotence.rs`](../../ide/src/tests/ide/test_format_idempotence.rs) anchor these
guarantees.

## Apply every edit against the original document

An edit batch interprets every range against the same original source. Before applying the batch,
the service stably sorts edits by range start and then range end. It applies them from the end of
the document toward the beginning so earlier edits do not shift later original positions.

Nonempty ranges must not overlap. Adjacent ranges are accepted. Multiple zero-width insertions at
the same position are also accepted; stable sorting preserves their caller-supplied order in the
result. Invalid ranges or an overlap reject the entire batch rather than applying a prefix. The
coordinate unit, character-boundary checks, and representation of that failure belong to the WASM
API specification.

The returned cursor is rebased deterministically:

- an edit ending at or before the cursor shifts it by the edit's inserted-length minus
  replaced-length delta;
- a cursor strictly inside a replaced range moves to that range's start;
- a cursor at a range start stays before the replacement; and
- an edit strictly after the cursor does not move it.

A zero-width insertion at the cursor therefore moves the cursor after the inserted text, while a
replacement beginning at the cursor leaves the cursor at the replacement start. Formatting uses
these same rules for its single full-document edit. The sort, overlap, and cursor behavior is
anchored in [`edit.rs`](../../ide/src/edit.rs),
[`text_edit.rs`](../../ide/src/text_edit.rs), and
[`test_edit_ops.rs`](../../ide/src/tests/ide/test_edit_ops.rs).

## Keep transport and presentation outside this contract

This specification does not promise Rust `pub` API stability, a particular completion widget, a
diagnostic grouping policy, signature popover layout, or automatic action selection. It also does
not define serialized enum spellings, optional-field representation, position units, scalar
boundary conversion, clamping, or JavaScript exception messages. Those transport details belong
to the WASM API specification; application presentation belongs to the consuming editor.

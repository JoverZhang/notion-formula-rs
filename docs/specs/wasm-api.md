---
doc_id: specs.wasm-api
title: "What does the WASM and JavaScript boundary guarantee?"
language: en
source_language: en
counterpart: ./wasm-api.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# WASM API

[简体中文](wasm-api.zh-CN.md)

This Current specification defines the synchronous WASM boundary for JavaScript integrations: how
to configure an `Analyzer`, which methods it exposes, the exact serialized request and result
shapes, how positions cross the boundary, and which controlled failures callers can distinguish.

Formula meaning belongs to the formula-language, formula-reference, and builtin-function
specifications. Completion, signature, formatting, and edit behavior belongs to the editor-services
specification. This document defines how those results cross WASM without repeating their
algorithms. It does not expose the Rust evaluator API or specify any demo UI policy.

## Reuse one configured Analyzer

The module exports one stateful `Analyzer` class with this synchronous surface:

```ts
new Analyzer(config: AnalyzerConfig)

analyzer.analyze(source: string): AnalyzeResult
analyzer.format(source: string, cursor_utf16: number): ApplyResult
analyzer.apply_edits(
  source: string,
  edits: Array<TextEdit>,
  cursor_utf16: number,
): ApplyResult
analyzer.help(source: string, cursor_utf16: number): HelpResult
```

An instance retains its property schema and completion preference limit. It has no method to change
that configuration. It does not retain source text, analysis output, edit history, or a cursor;
every operation receives a complete source string. Applications may therefore reuse an instance
for multiple documents that share one configuration.

Functions are not configurable through this boundary. Every instance receives the canonical
supported builtin set from the Rust declarations. A top-level `functions` field is rejected rather
than extending or replacing that set. The exports are anchored in
[`analyzer_wasm/src/lib.rs`](../../analyzer_wasm/src/lib.rs).

## Distinguish runtime config from generated TypeScript

The generated declaration is:

```ts
type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };

type Property = {
  name: string;
  type: Ty;
};

type AnalyzerConfig = {
  properties: Array<Property>;
  preferred_limit: number | null;
};
```

Both `AnalyzerConfig` fields are required by the generated TypeScript type. Runtime deserialization
is deliberately more permissive:

| Field | Runtime acceptance |
| --- | --- |
| `properties` | may be omitted, which is equivalent to `[]` |
| `preferred_limit` | may be omitted, `undefined`, or `null`, which selects the default `5` |
| `preferred_limit: 0` | accepted and disables `preferred_indices` |

When present, `properties` must be an array and each entry must contain a string `name` and a valid
`type`; an explicit `properties: undefined` is therefore invalid rather than equivalent to
omission. A supplied preference limit must deserialize as a nonnegative integer in the WASM
`usize` range. Missing required property fields, invalid type variants, or a wrong value shape
reject the whole configuration.

Unknown fields at the `AnalyzerConfig` top level are rejected. Unknown extra fields inside an
individual `Property` object are currently ignored. Passing schema validation does not make
duplicate property names a supported input; the formula-reference specification requires unique
names for specified lookup behavior.

The runtime schema is defined in
[`dto/v1.rs`](../../analyzer_wasm/src/dto/v1.rs). The stricter generated declaration is checked in as
[`wasm_dto.ts`](../../examples/vite/src/analyzer/generated/wasm_dto.ts). Integrations written in
TypeScript should satisfy the generated type; JavaScript integrations may rely on the runtime
omission and `null` behavior described above.

## Use UTF-16 positions at every JavaScript surface

The shared transport types are:

```ts
type Span = {
  start: number;
  end: number;
};

type TextEdit = {
  range: Span;
  new_text: string;
};

type ApplyResult = {
  source: string;
  cursor: number;
};
```

Every span, edit endpoint, input cursor, and returned cursor is measured in UTF-16 code units.
Spans and edit ranges are half-open: `start` is included and `end` is excluded. An `ApplyResult`
cursor refers to its returned, updated `source`, while input ranges refer to the supplied original
source.

An offset inside a Unicode scalar that occupies two UTF-16 code units is floored to the start of
that scalar. This rule applies to `help`, `format`, and `apply_edits` cursors and to **both** edit
range endpoints. A scalar-interior endpoint is not universally rejected; flooring can also collapse
a nonempty UTF-16 range to an empty internal range. The conversion contract is anchored in
[`offsets.rs`](../../analyzer_wasm/src/offsets.rs).

Past-end positions are method-specific:

| Input | Past-end behavior |
| --- | --- |
| `help` cursor | clamps to the end of the source |
| `format` cursor | throws `Invalid cursor` |
| `apply_edits` cursor | throws `Invalid cursor` |
| `apply_edits` range endpoint | throws `Invalid edit range` |

Diagnostic `line` and `col` are separate human-readable coordinates. Both are 1-based, and `col`
counts Unicode scalar values rather than UTF-16 code units. An emoji therefore advances a
diagnostic column by one while occupying two units in a span.

## Analyze returns formula problems as data

`analyze` returns these exact fields:

```ts
type DiagnosticKind = "error";

type CodeAction = {
  title: string;
  edits: Array<TextEdit>;
};

type Diagnostic = {
  kind: DiagnosticKind;
  message: string;
  span: Span;
  line: number;
  col: number;
  actions: Array<CodeAction>;
};

type Token = {
  kind: string;
  text: string;
  span: Span;
};

type AnalyzeResult = {
  diagnostics: Array<Diagnostic>;
  tokens: Array<Token>;
  output_type: string;
};
```

Lexer, parser, and semantic problems are returned in `diagnostics`; they do not make `analyze`
throw. The diagnostic ordering and optional action semantics are owned by the editor-services
specification. The transport includes only `kind`, `message`, `span`, `line`, `col`, and `actions`;
internal diagnostic codes, labels, and notes are not exposed.

`tokens` excludes comment and newline trivia but includes the `Eof` token. `Token.kind` is an open
string in the generated API rather than a closed TypeScript union. `output_type` is always a
non-null string; failed or indeterminate inference is represented as `"unknown"`.

For supported string input, the only controlled failure from `analyze` is result serialization,
reported as `Serialize error`. Exact formula diagnostic prose is not a compatibility key even
though controlled boundary-error messages below are. The boundary projection is anchored in
[`converter/analyze.rs`](../../analyzer_wasm/src/converter/analyze.rs).

## Help returns completion and optional signature data

`help` returns the following exact serialized surface:

```ts
type CompletionItemKind =
  | "FunctionGeneral"
  | "FunctionText"
  | "FunctionNumber"
  | "FunctionDate"
  | "FunctionPeople"
  | "FunctionList"
  | "FunctionSpecial"
  | "Builtin"
  | "Property"
  | "Operator";

type CompletionItem = {
  label: string;
  kind: CompletionItemKind;
  insert_text: string;
  primary_edit: TextEdit | null;
  cursor: number | null;
  additional_edits: Array<TextEdit>;
  detail: string | null;
  is_disabled: boolean;
  disabled_reason: string | null;
};

type CompletionResult = {
  items: Array<CompletionItem>;
  replace: Span;
  preferred_indices: Array<number>;
};

type DisplaySegment =
  | { kind: "Name"; text: string }
  | { kind: "Punct"; text: string }
  | { kind: "Separator"; text: string }
  | { kind: "Ellipsis" }
  | { kind: "Arrow"; text: string }
  | { kind: "Param"; name: string; ty: string; param_index: number | null }
  | { kind: "ReturnType"; text: string };

type SignatureItem = {
  segments: Array<DisplaySegment>;
};

type SignatureHelp = {
  signatures: Array<SignatureItem>;
  active_signature: number;
  active_parameter: number;
};

type HelpResult = {
  completion: CompletionResult;
  signature_help: SignatureHelp | null;
};
```

`completion` is always present, even when `items` is empty. Arrays are always present. When
signature help is unavailable, `signature_help` has no value. The generated declaration represents
absent optional values as `null`, but the current runtime serializer keeps the object properties
and puts `undefined` in them instead: this applies to `signature_help`, the nullable
`CompletionItem` fields, and `Param.param_index`. JavaScript and TypeScript consumers must account
for this generated-type/runtime difference. Completion candidates, ordering, edit meaning,
signature availability, and active-parameter behavior are owned by the editor-services
specification. Result conversion is anchored in
[`converter/completion.rs`](../../analyzer_wasm/src/converter/completion.rs).

`help` accepts incomplete formula source and uses the permissive cursor behavior described above.
Its only controlled failure is `Serialize error`.

## Format and apply edits return one updated document

Both mutating operations return `ApplyResult`; they do not mutate document state retained by the
Analyzer.

`format(source, cursor_utf16)` checks the cursor before it checks source syntax. A past-end cursor
therefore produces `Invalid cursor` even when the formula also cannot be formatted. On success it
returns the complete formatted source and a cursor in that new source. Syntactically invalid source
produces `Format error`; formatting layout and cursor rebasing are owned by the editor-services
specification.

`apply_edits(source, edits, cursor_utf16)` expects `edits` to deserialize as `Array<TextEdit>`. It
converts every range against the original source, checks the cursor, then validates and applies the
batch. On success it returns the complete updated source and rebased cursor. Original-coordinate
sorting, overlap behavior, and cursor rebasing are owned by the editor-services specification.

## Rely on controlled messages and validation order

The constructor rejects with the primitive string `Invalid analyzer config`. Operation failures are
JavaScript `Error` objects whose `message` is one of:

| Message | Meaning |
| --- | --- |
| `Invalid edits` | the `edits` value cannot deserialize as `Array<TextEdit>` |
| `Invalid cursor` | a checked cursor is past the original source end |
| `Invalid edit range` | an edit is reversed or its end is past the source end |
| `Overlapping edits` | converted original-document ranges overlap |
| `Format error` | the source has a lexer or parser problem |
| `Serialize error` | a result cannot be serialized to JavaScript |

Capitalization and wording in this table are part of the Current boundary contract.

Validation precedence is also observable:

| Operation | Validation and execution order |
| --- | --- |
| constructor | require an object and reject unknown top-level keys -> deserialize config -> construct Analyzer |
| `analyze` | analyze and convert result -> serialize |
| `format` | validate and convert cursor -> format -> serialize |
| `apply_edits` | deserialize edits -> validate and convert edit ranges in supplied order -> validate and convert cursor -> sort/check overlap/apply -> serialize |
| `help` | clamp/floor cursor -> compute help -> serialize |

For example, malformed edit payload wins over all later `apply_edits` failures; an invalid range
wins over an invalid cursor; but after valid range conversion, an invalid cursor is reported before
overlap detection. Scalar-interior flooring happens during conversion before overlap is checked, so
the ranges used for overlap detection can differ from the raw UTF-16 endpoints.

The exact messages and method order are defined in
[`analyzer_wasm/src/lib.rs`](../../analyzer_wasm/src/lib.rs) and exercised at the exported boundary
by [`analyzer_wasm/tests/analyze.rs`](../../analyzer_wasm/tests/analyze.rs).

## Keep evaluator and UI behavior outside this API

The WASM module exposes analysis and editor services only. It does not export formula evaluation,
prepared plans, row inputs, evaluator results, or Rust crate APIs. It also makes no promise about
completion widgets, quick-fix selection, popover layout, focus, or formula-panel identity. Those are
application concerns rather than fields or behavior of this boundary.

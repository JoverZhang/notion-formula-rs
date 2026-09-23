---
doc_id: specs.wasm-api
title: "WASM API and Worker"
language: en
source_language: zh-CN
counterpart: ./wasm-api.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: synced
translation_model: gpt-6-luna
translation_review_model: gpt-6-astra
last_verified: 2026-09-23
---

# WASM API and Worker

[简体中文](wasm-api.zh-CN.md) · [Specification index](README.md)

> The Planned Worker client and Current Analyzer are defined in separate sections; the current WASM has no Engine evaluation entry point.

## Planned: Thin Client

```text
Main thread → FormulaEngineClient / FormulaDraftClient → Worker RPC → WASM → Rust
The wrapper handles only RPC/session routing, lossless DTO conversion, UTF-8 ↔ UTF-16, Result ↔ Promise, and lifecycle.
Dependency analysis, cycle detection, compilation, evaluation, and text editing stay in Rust; Worker/thread-pool count is not part of the interface.
```

```ts
interface FormulaEngineClient {
  getProperty(id: PropertyId): Promise<PropertyState | null>;
  getProperties(): Promise<PropertyState[]>;
  getState(): Promise<FormulaEngineState>;
  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult>;
  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null>;
  // EvaluateInputError rejects the Promise; formula and row errors are returned in EvaluateResult.
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  close(): Promise<void>;
}
interface FormulaDraftClient {
  getState(): Promise<FormulaDraftState>;
  help(cursor: number, config: CompletionConfig): Promise<CursorHelp>;
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult>;
  intoDefinition(): Promise<FormulaDefinition>;
  close(): Promise<void>;
}
```

```text
queue
  One Engine and all its Draft clients share one FIFO queue; all calls execute serially in enqueue order.
engine.close()
  Idempotent; reject new calls → wait for queued calls to finish → release Engine and associated Drafts → terminate Worker.
draft.close()
  Releases that draft, that is, discards it.
draft.intoDefinition()
  Consumes the draft; the returned definition must still be wrapped in the Formula branch of a PropertyDefinition and explicitly upserted to modify Engine.
coordinates
  JS cursor/span use UTF-16 code units; Rust uses UTF-8 bytes.
DTO
  The names above correspond to the Rust contract; they do not declare that Rust memory layouts can be transferred directly.
  Concrete JS encoding, initialization entry point, and error payloads for the new interface will be refined with implementation; the Current DTO in the next section cannot be applied to it.
```

Business semantics are defined only by [Engine](formula-engine.md) and [Draft](ide.md).

## Current: Synchronous Analyzer

```ts
// The initialized generated package exports Analyzer; the exact signatures of default async initializer / initSync are not a stable contract.
declare class Analyzer {
  constructor(config: AnalyzerConfig);
  analyze(source: string): AnalyzeResult;
  format(source: string, cursor_utf16: number): ApplyResult;
  apply_edits(source: string, edits: TextEdit[], cursor_utf16: number): ApplyResult;
  help(source: string, cursor_utf16: number): HelpResult;
  free(): void;
}
// Retains only fixed properties/preferred_limit; does not store source, results, edit history, or cursor.
// Can be reused for multiple documents with the same configuration; has no configuration update method.
// When the host has Symbol.dispose, the generated glue also exposes the corresponding destructor method.
// Do not call after free/dispose; the resulting failure is outside the controlled contract.

type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };
type Property = { name: string; type: Ty };
type AnalyzerConfig = { properties: Property[]; preferred_limit: number | null };
// These are generated TS declarations; both fields are required. The accepted JS runtime range is described below.
```

```text
config
  Omitted properties → []; explicit undefined → invalid.
  Supports Array<Property>; serde's incidental acceptance of other iterables is not a stable guarantee.
  Omitted/undefined/null preferred_limit → 5; 0 → no preferred_indices.
  A specified value must be a non-negative integer within the WASM usize range; missing property fields, invalid types, or invalid shape → the entire config fails.
  Unknown top-level fields are rejected (including functions); extra fields inside Property are ignored; property names must be unique.
  The builtin set is fixed; configuration cannot extend or replace it. TS callers should satisfy the generated type.
```

### Current DTO

```ts
type Span = { start: number; end: number };
type TextEdit = { range: Span; new_text: string };
type ApplyResult = { source: string; cursor: number };
type DiagnosticKind = "error";
type CodeAction = { title: string; edits: TextEdit[] };
type Diagnostic = {
  kind: DiagnosticKind; message: string; span: Span;
  line: number; col: number; actions: CodeAction[];
};
type Token = { kind: string; text: string; span: Span };
type AnalyzeResult = {
  diagnostics: Diagnostic[]; tokens: Token[]; output_type: string;
};
type CompletionItemKind =
  | "FunctionGeneral" | "FunctionText" | "FunctionNumber" | "FunctionDate"
  | "FunctionPeople" | "FunctionList" | "FunctionSpecial" | "Builtin" | "Property" | "Operator";
type CompletionItem = {
  label: string; kind: CompletionItemKind; insert_text: string;
  primary_edit: TextEdit | null; cursor: number | null; additional_edits: TextEdit[];
  detail: string | null; is_disabled: boolean; disabled_reason: string | null;
};
type CompletionResult = {
  items: CompletionItem[]; replace: Span; preferred_indices: number[];
};
type DisplaySegment =
  | { kind: "Name"; text: string } | { kind: "Punct"; text: string }
  | { kind: "Separator"; text: string } | { kind: "Ellipsis" }
  | { kind: "Arrow"; text: string }
  | { kind: "Param"; name: string; ty: string; param_index: number | null }
  | { kind: "ReturnType"; text: string };
type SignatureItem = { segments: DisplaySegment[] };
type SignatureHelp = {
  signatures: SignatureItem[]; active_signature: number; active_parameter: number;
};
type HelpResult = { completion: CompletionResult; signature_help: SignatureHelp | null };
```

```text
analyze
  Formula problems are returned as diagnostics; invalid formulas do not throw.
  Internal diagnostic codes/labels/notes are not exposed; kind currently has only "error".
  Comments/newlines are removed from tokens, but Eof is retained; Token.kind is an open string.
  output_type is always a string; failed/unknown inference is "unknown", not null.
help
  Tolerates incomplete source; arrays such as items/additional_edits/preferred_indices are always present.
  Note the current discrepancy: generated TS maps Option to null, but the actual serializer retains the field and outputs undefined.
  This affects signature_help, primary_edit, cursor, detail, disabled_reason, and param_index.
format / apply_edits
  Return the updated complete source and cursor; do not store source or modify Analyzer configuration.
```

Candidate, ordering, diagnostic-order, formatting, and editing semantics belong to the [Current section of IDE](ide.md).

### Current Coordinates and Exceptions

```text
strings
  source/property name/new_text, etc. must be valid Unicode, with no unpaired surrogate.
  The generated boundary replaces an unpaired surrogate with U+FFFD; inputs relying on this normalization are unsupported.
positions
  All JS cursor/span/edit endpoints are UTF-16 code units, using half-open ranges [start,end).
  Input ranges are based on the original source; returned cursor is based on the returned new source.
  Supported numeric values are finite integers 0..=4_294_967_295.
  Invalid numeric values for a direct cursor parameter may first be coerced by the ABI; such inputs are unsupported.
  Invalid numeric values in an edit DTO → Invalid edits.
  A position inside a surrogate pair → floored to the start of that scalar; this applies to cursor and both endpoints.
  Thus a non-empty UTF-16 range may collapse to empty; it is not always rejected.
past end
  help cursor → clamp to end of document
  format/apply_edits cursor → Invalid cursor
  apply_edits endpoint → Invalid edit range
diagnostic location
  line/col start at 1; col counts Unicode scalars, not UTF-16; an emoji counts as 1 in col and 2 in span.

controlled failures
  constructor → throws primitive string "Invalid analyzer config"
  methods → Error("Invalid edits" | "Invalid cursor" | "Invalid edit range" |
                "Overlapping edits" | "Format error" | "Serialize error")
  Reversed/out-of-bounds range → Invalid edit range; lexer/parser diagnostic → Format error.
  The only controlled exceptions from analyze/help are Serialize error; formula/incomplete-source problems are returned as data.

validation order
  constructor → object/top-level field validation → deserialize → construct
  analyze     → analyze/convert → serialize
  format      → cursor validation/conversion → format → serialize
  apply_edits → deserialize edits → validate/convert ranges in input order → validate/convert cursor
              → sort/check overlap/apply → serialize
  help        → clamp/floor cursor → help → serialize
  Endpoint flooring happens before overlap checks; when multiple errors coexist, the order above determines which one is reported first.
```

The current boundary does not expose the evaluator, plan, or business rows, and does not define demo UI policy.
Implementation anchors: [exports](../../analyzer_wasm/src/lib.rs), [DTO](../../analyzer_wasm/src/dto/v1.rs),
[coordinates](../../analyzer_wasm/src/offsets.rs),
[generated TS](../../examples/vite/src/analyzer/generated/wasm_dto.ts).

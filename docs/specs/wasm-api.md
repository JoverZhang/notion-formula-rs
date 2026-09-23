---
doc_id: specs.wasm-api
title: "WASM API and Worker"
language: en
source_language: zh-CN
counterpart: ./wasm-api.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: needs-update
last_verified: 2026-09-23
---

# WASM API and Worker

[简体中文](wasm-api.zh-CN.md) · [Specification index](README.md)

> The Planned Worker client and Current Analyzer are specified in separate sections; Current WASM has no Engine evaluation entry point.

## Planned: thin clients

```text
Main thread → FormulaEngineClient / FormulaDraftClient → Worker RPC → WASM → Rust
Wrapper owns only RPC/session routing, lossless DTO conversion, UTF-8 ↔ UTF-16, Result ↔ Promise, and lifetime.
Dependency analysis, cycle checks, compilation, evaluation, and text editing stay in Rust; Worker/pool counts are unspecified.
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
  One Engine and all its Draft clients share one FIFO queue; every call executes serially in enqueue order.
engine.close()
  Idempotent: reject new calls → drain queued calls → release Engine and associated Drafts → terminate Worker.
draft.close()
  Release this draft: discard.
draft.intoDefinition()
  Consume this draft; wrap it in the Formula variant of PropertyDefinition and explicitly upsert to mutate Engine.
coordinates
  JS cursors/spans use UTF-16 code units; Rust uses UTF-8 bytes.
DTO
  Names above refer to Rust contracts, not directly transferable Rust memory layouts.
  New JS encodings, initialization entry point, and error payloads need specification alongside implementation;
  the Current DTOs below are not substitutes.
```

Only [Engine](formula-engine.md) and [Draft](ide.md) define domain semantics.

## Current: synchronous Analyzer

```ts
// Analyzer is exported after package initialization; exact default async initializer / initSync signatures are not stable contract.
declare class Analyzer {
  constructor(config: AnalyzerConfig);
  analyze(source: string): AnalyzeResult;
  format(source: string, cursor_utf16: number): ApplyResult;
  apply_edits(source: string, edits: TextEdit[], cursor_utf16: number): ApplyResult;
  help(source: string, cursor_utf16: number): HelpResult;
  free(): void;
}
// Retains fixed properties/preferred_limit, never source, results, edit history, or cursor.
// Reusable across documents with the same config; no configuration mutation.
// Generated glue also installs disposal when the host defines Symbol.dispose.
// No domain calls after free/dispose; resulting failures are uncontrolled.

type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };
type Property = { name: string; type: Ty };
type AnalyzerConfig = { properties: Property[]; preferred_limit: number | null };
// Generated TS requires both fields; JS runtime acceptance is defined below.
```

```text
config
  Omitted properties → []; explicit undefined → invalid.
  Array<Property> is supported; incidental serde acceptance of other iterables is not a stable guarantee.
  Omitted/undefined/null preferred_limit → 5; 0 → no preferred_indices.
  Supplied limit must be a nonnegative WASM usize integer; missing property fields, invalid types/shapes → whole config fails.
  Unknown top-level fields rejected, including functions; extra Property fields ignored; property names must be unique.
  Builtins are fixed, not configurable or replaceable. TS callers should satisfy the generated declaration.
```

### Current DTOs

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
  Formula problems return diagnostics, not exceptions merely because a formula is invalid.
  Internal diagnostic codes/labels/notes are not exposed; kind currently only "error".
  Tokens exclude comments/newlines but include Eof; Token.kind is an open string.
  output_type is always a string; unknown/failed inference is "unknown", not null.
help
  Tolerates incomplete source; arrays such as items/additional_edits/preferred_indices are always present.
  Current discrepancy: generated TS declares Option as null, but the serializer retains the fields and emits undefined.
  Affects signature_help, primary_edit, cursor, detail, disabled_reason, and param_index.
format / apply_edits
  Return full updated source and cursor; neither retain source nor modify Analyzer configuration.
```

[Current IDE behavior](ide.md) owns candidates, ranking, diagnostic order, formatting, and edits.

### Current coordinates and exceptions

```text
strings
  Source, property names, new_text, etc. must be well-formed Unicode without isolated surrogates.
  Generated bindings replace isolated surrogates with U+FFFD; inputs depending on this normalization are unsupported.
positions
  Every JS cursor/span/edit endpoint uses UTF-16 code units and half-open [start,end) ranges.
  Input ranges refer to original source; returned cursors refer to returned updated source.
  Supported numbers are finite integers 0..=4_294_967_295.
  Invalid direct cursor numbers may be coerced by the ABI before validation; such inputs are unsupported.
  Invalid numbers inside edit DTOs → Invalid edits.
  Inside a surrogate pair → floor to the scalar's start, for cursors and both endpoints.
  A nonempty UTF-16 range may therefore collapse to empty; such positions are not universally rejected.
past end
  help cursor → clamp to source end
  format/apply_edits cursor → Invalid cursor
  apply_edits endpoint → Invalid edit range
diagnostic location
  line/col are 1-based; col counts Unicode scalars, not UTF-16. An emoji takes one column but two span units.

controlled failures
  constructor → throws primitive string "Invalid analyzer config"
  methods → Error("Invalid edits" | "Invalid cursor" | "Invalid edit range" |
                  "Overlapping edits" | "Format error" | "Serialize error")
  Reversed/past-end ranges → Invalid edit range; lexer/parser diagnostics → Format error.
  analyze/help have only Serialize error as a controlled exception; formula/incomplete-source problems return data.

validation order
  constructor → object/top-level field checks → deserialize → construct
  analyze     → analyze/convert → serialize
  format      → validate/convert cursor → format → serialize
  apply_edits → deserialize edits → validate/convert ranges in supplied order → validate/convert cursor
              → sort/check overlap/apply → serialize
  help        → clamp/floor cursor → help → serialize
  Endpoint flooring precedes overlap checks; the order above determines which coexisting failure is reported first.
```

Current exports expose no evaluator, plans, or business rows, and specify no demo UI policy.
Implementation anchors: [exports](../../analyzer_wasm/src/lib.rs), [DTOs](../../analyzer_wasm/src/dto/v1.rs),
[coordinates](../../analyzer_wasm/src/offsets.rs),
[generated TS](../../examples/vite/src/analyzer/generated/wasm_dto.ts).

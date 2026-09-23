---
doc_id: specs.ide
title: "FormulaDraft: Editor Capabilities"
language: en
source_language: zh-CN
counterpart: ./ide.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: synced
translation_model: gpt-6-luna
last_verified: 2026-09-23
---

# FormulaDraft: Editor Capabilities

[简体中文](ide.zh-CN.md) · [Specification index](README.md)

> Planned: FormulaDraft has not been implemented. Completion, signature help, and text editing follow the Current IDE behavior in the final section.

## FormulaDraft

```rust spec=formula_draft.h.rs
pub use analyzer::{Span, TextEdit, Token};
pub use ide::{
    CompletionConfig, CompletionItem, CompletionKind, CompletionResult,
    DisplaySegment, SignatureHelp, SignatureItem,
};

/// Analyzes through a shared borrow of Engine; dependencies with the same ID are
/// computed from the definition currently being edited.
/// Multiple Drafts can be created from the same Engine at once; all Draft borrows
/// must end before saving.
#[spec::private_fields]
pub struct FormulaDraft<'engine> {}

#[spec::header]
impl FormulaDraft<'_> {
    pub fn state(&self) -> &FormulaDraftState;

    /// Queries completion, postfix completion, and signature help together.
    /// Candidates that would create a dependency cycle are still returned, but marked disabled.
    /// CompletionConfig::default() has preferred_limit 5; 0 disables preferred_indices.
    pub fn help(&self, cursor: TextOffset, config: CompletionConfig) -> CursorHelp;

    /// Returns edits attached to a diagnostic as suggestions; an unknown or non-current ID returns an empty list.
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// Returns a replacement edit for the entire expression; lexer/parser diagnostics prevent formatting, but semantic errors do not.
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// Atomically updates the expression, updating output_type, diagnostics, and tokens before returning.
    ///
    /// - allow: an invalid expression; inspect diagnostics through state().
    /// - error: validation of the Edits version, ranges, cursor, or non-overlap fails; Draft remains unchanged.
    pub fn update_expression(&mut self, update: ExpressionUpdate)
        -> Result<UpdateExpressionResult, UpdateExpressionError>;

    /// Does not automatically commit; when saving, pass the returned definition to FormulaEngine::upsert.
    pub fn into_definition(self) -> FormulaDefinition;
}
```

[FormulaEngine](formula-engine.md) defines shared types, the `create_draft` entry point, and editing and saving examples.
[Tokens and Span](formula-language.md#lexical-structure) are defined by the language spec.
Completion follows the [IDE return structures](../../ide/src/lib.rs) and [candidate types](../../ide/src/completion/mod.rs);
SignatureHelp follows the [signature structures](../../ide/src/signature/mod.rs) and [display segments](../../ide/src/display.rs).

```rust spec=formula_draft.h.rs
/// UTF-8 byte offset.
#[derive(derive_more::From)]
pub struct TextOffset(pub usize);

#[derive(Clone, Copy)]
pub struct DraftVersion(pub u64);

pub struct FormulaDraftState {
    /// Incremented by 1 when Replace changes text or Edits succeeds.
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    /// None when no definite type can be inferred; Unknown/Null is not returned.
    pub output_type: Option<ValueType>,
    /// Syntax and semantic diagnostics, independent of cursor; includes direct and indirect self-reference problems.
    pub diagnostics: Vec<ExpressionDiagnostic>,
    /// Lexical tokens of the current definition.expression, retaining comments, newlines, and Eof.
    pub tokens: Vec<Token>,
}
pub struct ExpressionDiagnostic {
    pub id: DiagnosticId,
    /// Location in the current Draft's expression.
    pub span: Span,
    pub message: String,
}
pub struct DiagnosticId(pub String);

pub struct CursorHelp {
    /// Use this version as FormulaEdit.base_version when applying this completion's edits.
    pub base_version: DraftVersion,
    pub completion: CompletionResult,
    pub signature_help: Option<SignatureHelp>,
}
pub struct FormulaEdit {
    /// Must equal the current state.version, preventing stale edits from being applied to a modified expression.
    pub base_version: DraftVersion,
    /// All ranges are based on the expression before modification and must not overlap.
    pub edits: Vec<TextEdit>,
}
pub enum ExpressionUpdate {
    /// Replaces the current expression; base_version is not required.
    Replace(String),
    Edits {
        edit: FormulaEdit,
        /// Coordinates in the expression before modification, relocated according to the edits.
        cursor: TextOffset,
    },
}
pub struct QuickFix {
    pub title: String,
    /// version is bound to the Draft that produced the diagnostic.
    pub edit: FormulaEdit,
}
pub struct UpdateExpressionResult {
    pub state: FormulaDraftState,
    /// Replace returns the new expression.len(); for an empty string this is 0. Edits returns the relocated coordinate.
    pub cursor: TextOffset,
}

/// The expression contains a lexer/parser diagnostic and cannot be formatted.
pub struct FormatError;

pub enum UpdateExpressionError {
    VersionMismatch,
    /// Out of bounds or not on a UTF-8 character boundary.
    InvalidCursor,
    /// Reversed or out-of-bounds range, or an endpoint not on a UTF-8 character boundary.
    InvalidEditRange,
    OverlappingEdits,
}
```

A completion's primary/additional edits are combined into one `FormulaEdit` and applied through `update_expression()`.
The completion cursor uses `CompletionItem.cursor`, defaulting to the position after the text inserted by the primary edit;
then account for additional edits before the primary edit to obtain the post-edit coordinate for the editor.

## Current: IDE Behavior

The current service processes source per request and does not persist FormulaDraft. This section owns editing behavior;
JS serialization, coordinate validation, and exceptions are defined by the [WASM API](wasm-api.md).

```text
diagnostics
  order = parser ++ lexer ++ semantic; no global sorting/deduplication; an individual stage may merge entries with the same span.
  parser may attach recovery actions; lexer/semantic currently do not; actions are never applied automatically.
  message is human-readable text, not a stable error code.

completion: candidate set
  start of expression / empty argument position       → properties, not/true/false, supported functions
  inside identifier/not/true/false or at recognized prefix end → same
  after complete identifier/literal/right paren, and not a prefix → == != >= > <= < + - * /, postfix-capable functions
  receiver.                                          → postfix functions compatible with receiver type
  receiver type unknown                              → all postfix candidates
  inside string                                      → no completion; signature help may still be available
  entire document contains only horizontal whitespace → start candidates only when cursor=0; newline does not use this special case
  % ^ && || are language operators, but are not in the after-atom candidate set.

completion: matching and edits
  Prefix recognition compares lowercase prefixes but checks full matches using original spelling; a complete mixed-case name may still take the prefix path.
  Prefix completion replaces the entire identifier; ordinary insertion uses an empty range at the cursor.
  Function insertion includes parentheses, with cursor inside them; not/true/false have a trailing space; a property inserts prop("Name").
  A property's cursor is after the call; the name is inserted directly without escaping quotes/backslashes, and may produce invalid source.
  A disabled property retains its reason, but has no primary edit/cursor and is excluded from preferred.

completion: ordering and preferred
  A known concrete parameter type only reorders; it does not filter. Skip this step when types are unknown/generic or no parameter mapping exists.
  Bucket consecutive items by kind: enabled first, then by type match; sort buckets by best match, with ties using a fixed kind order.
  Later query sorting may change this order: it applies only to non-empty queries whose normalized form contains ASCII letters/digits/_/whitespace;
  comparison ignores ASCII case and _: exact > contains > ordered subsequence.
  Break ties by compactness, kind, then original order; function () and a postfix's leading . do not take part in query matching.
  For ordinary expressions, retain non-matching items; after-dot filters non-matches.
  A query containing non-ASCII, non-whitespace characters skips query sorting, preserves the previous order, and yields preferred=[].
  preferred_indices point into the final list, in final order, with at most preferred_limit enabled matching functions/properties.
  No query, limit=0, or no matches → preferred=[].

signature help
  Considers only the innermost unclosed (, which must be preceded by a known function, with the cursor after (.
  Inner grouping/unknown functions suppress fallback to outer calls; no help before the opening paren or after leaving the call; a missing closing paren is allowed.
  Returns one signature, active_signature=0; parameter inference affects display, and unknown/generic parameters may remain.
  A postfix receiver is handled separately and does not occupy a displayed parameter index.
  Top-level commas determine active_parameter; nested commas are ignored, empty arguments still select a slot, and repeat parameters map to display slots.
  With no mapping → last displayed parameter; zero parameters → 0. This fallback currently has no dedicated regression test.

format
  Any lexer/parser diagnostic → failure; semantic diagnostics do not prevent formatting.
  Full and deterministic; formatting is idempotent for covered syntax. Indent by 2 spaces, use conventional spaces around binary/ternary operators and commas, and end with one newline.
  Preserve attached comments; inline only when inlining is permitted and indentation plus rendered UTF-8 byte length is <= 80; otherwise use multiple lines.
  Atomic expressions are exempt from this width check; there are no formatting options.
  Current returns the complete source and relocates the cursor as a whole-document replacement edit: internal positions usually move to zero, while the end follows the new end.
  Planned format_edits returns edits; the Current return structure cannot be treated as its structure.

apply edits
  All ranges are based on the original source; stable-sort by (start,end), then apply in reverse order.
  Non-empty ranges must not overlap; adjacent ranges and zero-width inserts at the same position are allowed, with the latter preserving caller order.
  Invalid/overlapping ranges → the entire operation fails.
  edit.end <= cursor       → shift by the length difference
  start < cursor < end     → move to the replacement start
  cursor == start          → remain before the replacement
  edit after cursor        → cursor unchanged
  Therefore an insertion at the cursor places it after the inserted text; a replacement start remains before the replacement text.
```

Implementation anchors: [help](../../ide/src/lib.rs), [signature](../../ide/src/signature/mod.rs),
[format](../../ide/src/format.rs), [edit](../../ide/src/edit.rs),
[byte editing](../../ide/src/text_edit.rs). Candidate ordering does not define UI display, automatic acceptance, or selection policy.

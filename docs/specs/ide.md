---
doc_id: specs.ide
title: "FormulaDraft: editor capabilities"
language: en
source_language: zh-CN
counterpart: ./ide.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: needs-update
last_verified: 2026-09-23
---

# FormulaDraft: editor capabilities

[简体中文](ide.zh-CN.md) · [Specification index](README.md)

> Planned: FormulaDraft is not implemented. Completion, signature help, and text editing follow the Current IDE behavior in the final section.

## FormulaDraft

```rust spec=formula_draft.h.rs
pub use analyzer::{Span, TextEdit, Token};
pub use ide::{
    CompletionConfig, CompletionItem, CompletionKind, CompletionResult,
    DisplaySegment, SignatureHelp, SignatureItem,
};

/// Borrows Engine immutably for analysis; dependencies with the same ID are calculated from the definition being edited.
/// Multiple Drafts may be created from one Engine; end all Draft borrows before saving.
#[spec::private_fields]
pub struct FormulaDraft<'engine> {}

#[spec::header]
impl FormulaDraft<'_> {
    pub fn state(&self) -> &FormulaDraftState;

    /// Queries completion, postfix completion, and signature help together.
    /// Candidates that would create a dependency cycle are still returned, but marked disabled.
    /// CompletionConfig::default() has preferred_limit 5; 0 disables preferred_indices.
    pub fn help(&self, cursor: TextOffset, config: CompletionConfig) -> CursorHelp;

    /// Returns suggestions attached to a diagnostic; unknown IDs and IDs from an earlier version return an empty list.
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// Returns a replacement edit for the entire expression; lexer/parser diagnostics block formatting, semantic errors do not.
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// Atomically updates expression, then updates output_type, diagnostics, and tokens before returning.
    ///
    /// - allow: Invalid expression; inspect diagnostics through state().
    /// - error: Edits fails version, range, cursor, or non-overlap validation; Draft remains unchanged.
    pub fn update_expression(&mut self, update: ExpressionUpdate)
        -> Result<UpdateExpressionResult, UpdateExpressionError>;

    /// Does not commit automatically; pass the returned definition to FormulaEngine::upsert when saving.
    pub fn into_definition(self) -> FormulaDefinition;
}
```

[FormulaEngine](formula-engine.md) defines shared types, the `create_draft` entry point, and editing and saving examples.
[Token and Span](formula-language.md#lexical-structure) are defined by the language spec.
Completion follows the [IDE result structures](../../ide/src/lib.rs) and [candidate types](../../ide/src/completion/mod.rs);
SignatureHelp follows the [signature structures](../../ide/src/signature/mod.rs) and [display segments](../../ide/src/display.rs).

```rust spec=formula_draft.h.rs
/// UTF-8 byte offset.
#[derive(derive_more::From)]
pub struct TextOffset(pub usize);

#[derive(Clone, Copy)]
pub struct DraftVersion(pub u64);

pub struct FormulaDraftState {
    /// Increments by 1 when Replace changes the text or Edits succeeds.
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    /// None when no concrete type can be inferred; never Unknown/Null.
    pub output_type: Option<ValueType>,
    /// Syntax and semantic diagnostics, independent of cursor; includes direct and transitive self-reference problems.
    pub diagnostics: Vec<ExpressionDiagnostic>,
    /// Lexical tokens for the current definition.expression, retaining comments, newlines, and Eof.
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
    /// Use this version as FormulaEdit.base_version when applying these completion edits.
    pub base_version: DraftVersion,
    pub completion: CompletionResult,
    pub signature_help: Option<SignatureHelp>,
}
pub struct FormulaEdit {
    /// Must equal current state.version to prevent stale edits from being applied to a changed expression.
    pub base_version: DraftVersion,
    /// All ranges refer to the pre-edit expression and must not overlap.
    pub edits: Vec<TextEdit>,
}
pub enum ExpressionUpdate {
    /// Replaces the current expression without requiring base_version.
    Replace(String),
    Edits {
        edit: FormulaEdit,
        /// Position in the pre-edit expression, rebased through the edits.
        cursor: TextOffset,
    },
}
pub struct QuickFix {
    pub title: String,
    /// Version bound to the Draft that produced the diagnostic.
    pub edit: FormulaEdit,
}
pub struct UpdateExpressionResult {
    pub state: FormulaDraftState,
    /// Replace returns the new expression.len(), or 0 for an empty string; Edits returns the rebased position.
    pub cursor: TextOffset,
}

/// Cannot format an expression with lexer/parser diagnostics.
pub struct FormatError;

pub enum UpdateExpressionError {
    VersionMismatch,
    /// Out of bounds or not on a UTF-8 character boundary.
    InvalidCursor,
    /// Reversed/out-of-bounds range or endpoint not on a UTF-8 character boundary.
    InvalidEditRange,
    OverlappingEdits,
}
```

Primary and additional completion edits are combined into one `FormulaEdit` and applied with `update_expression()`.
The completion cursor comes from `CompletionItem.cursor`, or defaults to after the primary edit's inserted text;
then account for additional edits before the primary edit to get the post-edit position for the editor.

## Current: IDE behavior

Current services process source per request; they do not retain a FormulaDraft. This section owns editing behavior;
[WASM API](wasm-api.md) owns JS serialization, coordinate validation, and exceptions.

```text
diagnostics
  Order = parser ++ lexer ++ semantic; no global sort/deduplication. A phase may coalesce same-span reports.
  Parser diagnostics may carry recovery actions; lexer/semantic diagnostics currently do not. Never auto-applied.
  Messages are human-readable text, not stable error codes.

completion: candidates
  Expression start / empty argument                       → properties, not/true/false, supported functions
  Inside identifier/not/true/false or recognized prefix end → same
  After complete identifier/literal/closing paren, not prefix → == != >= > <= < + - * /, postfix-capable functions
  receiver.                                               → receiver-compatible postfix functions
  Unknown receiver type                                   → all postfix candidates
  Inside string                                           → no completion; signature help may remain
  Entire document is horizontal whitespace                 → start candidates only at cursor=0; not a newline special case
  % ^ && || are language operators but are not offered after an atom.

completion: matching and edits
  Prefix detection compares lowercase prefixes but tests exactness against original spelling.
  A complete mixed-case name can therefore still take the prefix path.
  Prefix completion replaces the entire identifier; ordinary insertion uses an empty range at cursor.
  Functions insert parentheses with cursor inside; not/true/false append a space; properties insert prop("Name").
  Property cursor follows the call; names are inserted verbatim, without escaping quotes/backslashes, possibly invalid source.
  Disabled properties retain a reason but have no primary edit/cursor and are never preferred.

completion: ranking and preferred
  Known concrete argument types reorder rather than filter; unknown/generic/unmapped slots skip this step.
  Contiguous kind buckets: enabled first, then stronger type matches; buckets rank by best match with fixed-kind ties.
  Later query ranking can change that order: only nonempty normalized ASCII alphanumeric/_/whitespace queries qualify.
  Matching ignores ASCII case and _: exact > contains > ordered subsequence.
  Ties use compactness, kind, then original order; function () and postfix leading . are excluded from query matching.
  Ordinary expression completion keeps nonmatches; after-dot completion removes them.
  Non-ASCII non-whitespace query characters → skip query ranking, preserve earlier order, preferred=[].
  preferred_indices reference the final list in final order, up to preferred_limit enabled matching functions/properties.
  No query, limit=0, or no match → preferred=[].

signature help
  Only the innermost unmatched ( is considered; a known function must precede it, with cursor after (.
  Inner grouping/unknown functions suppress outer fallback; none before ( or after leaving the call; missing ) is tolerated.
  One signature, active_signature=0; inferred arguments affect display, which may retain unknown/generic types.
  A postfix receiver is handled separately and does not occupy a displayed parameter index.
  Top-level commas select active_parameter; nested commas do not. Empty arguments select slots; repeat slots project to display slots.
  No mapping → last displayed parameter; zero parameters → 0. This fallback has no dedicated regression test yet.

format
  Any lexer/parser diagnostic → failure; semantic diagnostics do not block formatting.
  Whole-source, deterministic; formatting covered syntax is idempotent. Two-space indentation, conventional binary/ternary/comma spacing, one final newline.
  Attached comments retained; inline only when permitted and indentation + rendered UTF-8 byte length <= 80, otherwise multiline.
  Atoms bypass that width decision; no formatting options.
  Current returns full source and rebases cursor through a whole-source replacement: interior positions usually reach zero, end follows the new end.
  Planned format_edits returns edits; the Current result shape is not its result shape.

apply edits
  Every range refers to original source; stable sort by (start,end), then apply in reverse.
  Nonempty ranges cannot overlap; adjacent ranges and same-position zero-width insertions are allowed, with caller order preserved.
  Invalid/overlapping ranges → whole operation fails.
  edit.end <= cursor       → shift by length delta
  start < cursor < end     → move to replacement start
  cursor == start          → stay before replacement
  edit after cursor        → no effect
  Thus insertion at cursor leaves it after inserted text; replacement at cursor leaves it before replacement text.
```

Implementation anchors: [help](../../ide/src/lib.rs), [signature](../../ide/src/signature/mod.rs),
[format](../../ide/src/format.rs), [edit](../../ide/src/edit.rs),
[byte edits](../../ide/src/text_edit.rs). Candidate ranking does not define UI presentation, auto-commit, or selection policy.

---
doc_id: specs.ide
title: "FormulaDraft: editor services"
language: en
source_language: zh-CN
counterpart: ./ide.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: needs-update
last_verified: 2026-09-19
---

# FormulaDraft: editor services

[简体中文](ide.zh-CN.md) · [Specification index](README.md)

> Planned: FormulaDraft is not implemented; Token, Completion, SignatureHelp, and error enums need further definition. The final section preserves Current IDE behavior separately.

## FormulaDraft

```rust
/// Borrows Engine immutably for analysis; same-ID dependencies follow the definition being edited.
/// Multiple Drafts may borrow one Engine; saving requires ending every Draft's borrow first.
pub struct FormulaDraft<'engine> {}

impl FormulaDraft<'_> {
    pub fn state(&self) -> &FormulaDraftState;

    /// Replace expression and update output_type, diagnostics, and tokens before returning.
    /// A changed expression advances version, invalidating earlier FormulaEdits.
    ///
    /// - allow: Invalid expression; inspect diagnostics through state().
    pub fn set_expression(&mut self, expression: String);

    /// Query completion, postfix completion, and signature help together.
    /// Candidates that would create a dependency cycle remain present but disabled.
    pub fn help(&self, cursor: TextOffset) -> CursorHelp;

    /// Unknown diagnostic IDs or IDs from an earlier version return an empty list.
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// Requires formattable syntax, not semantic validity.
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// Validate version, pre-edit ranges, cursor, and non-overlap, then apply every edit atomically.
    /// Rebase cursor, update analysis, and advance version before returning.
    ///
    /// - allow: Invalid expression; inspect diagnostics through state().
    /// - error: Validation fails; Draft remains unchanged.
    pub fn apply_edits(&mut self, edit: FormulaEdit, cursor: TextOffset)
        -> Result<ApplyEditsResult, ApplyEditsError>;

    /// Does not commit automatically; pass the returned definition to FormulaEngine::upsert to save it.
    pub fn into_definition(self) -> FormulaDefinition;
}
```

[FormulaEngine](formula-engine.md) defines shared types, the `create_draft` entry point, and the editing and saving example.

```rust
/// UTF-8 byte offset.
pub struct TextOffset(usize);
pub struct DraftVersion(u64);

pub struct FormulaDraftState {
    /// Advances when expression changes or apply_edits succeeds.
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    /// None when no concrete type can be inferred; never Unknown/Null.
    pub output_type: Option<Type>,
    /// Cursor-independent; includes direct and transitive self-reference.
    pub diagnostics: Vec<FormulaDiagnostic>,
    pub tokens: Vec<Token>,
}
pub struct CursorHelp {
    pub completions: Vec<Completion>,
    pub signature_help: Option<SignatureHelp>,
}
pub struct TextEdit {
    /// All edits refer to the same pre-edit expression.
    pub range: Span,
    pub new_text: String,
}
pub struct FormulaEdit {
    /// Must equal current state.version.
    pub base_version: DraftVersion,
    /// Ranges must not overlap.
    pub edits: Vec<TextEdit>,
}
pub struct QuickFix {
    pub title: String,
    /// Version bound to the Draft that produced the diagnostic.
    pub edit: FormulaEdit,
}
pub struct ApplyEditsResult {
    pub state: FormulaDraftState,
    /// Position in the edited expression.
    pub cursor: TextOffset,
}
```

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
  A postfix receiver is separate and excluded from displayed parameter indices.
  Top-level commas select active_parameter; nested commas do not. Empty arguments select slots; repeat slots project to display slots.
  No mapping → last displayed parameter; zero parameters → 0. This fallback has no dedicated regression test yet.

format
  Any lexer/parser diagnostic → failure; semantic diagnostics do not block formatting.
  Whole-source, deterministic, idempotent for covered syntax; two-space indentation, conventional binary/ternary/comma spacing, one final newline.
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

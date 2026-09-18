---
doc_id: specs.formula-draft
title: "FormulaDraft: editor services"
language: en
source_language: zh-CN
counterpart: ./formula-draft.zh-CN.md
implementation_status: planned
document_status: draft
translation_status: synced
last_verified: 2026-09-18
---

# FormulaDraft: editor services

[简体中文](formula-draft.zh-CN.md) · [Specification index](README.md)

> Planned: FormulaDraft is not implemented. The final section preserves Current IDE behavior separately; its DTOs do not automatically become the new API.

## Detached draft

```rust
// Created by FormulaEngine::create_draft(definition); edits one expression.
// Captures Schema, other formula types, and the dependency graph; never follows later Engine mutations.
// The candidate replaces same-ID dependency edges before type/cycle checks; new formula IDs are allowed.
// Property ID collision → CreateDraftError; invalid source → a Draft with diagnostics.
// No implicit Engine mutation; v1 makes no incremental-compilation performance guarantee.
pub struct FormulaDraft { /* private */ }

impl FormulaDraft {
    pub fn state(&self) -> &FormulaDraftState;

    // Combined completion, postfix completion, and signature help; cursor may be queried repeatedly.
    // Candidates that would create a dependency cycle remain present but disabled.
    pub fn help(&self, cursor: TextOffset) -> CursorHelp;
    // Does not modify source; unknown or stale diagnostic ID → empty list.
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;
    // Does not modify source; requires formattable syntax, not semantic validity.
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    // The only Draft mutation: validate version, original-source ranges, cursor, and non-overlap atomically.
    // Success → apply every edit, rebase cursor, recompile, advance version.
    // New source may be invalid; diagnostics belong to the new state. Failure → Err, Draft unchanged.
    pub fn apply_edits(&mut self, edit: FormulaEdit, cursor: TextOffset)
        -> Result<ApplyEditsResult, ApplyEditsError>;

    pub fn into_definition(self) -> FormulaDefinition; // Consumes Draft without committing
}
```

[Engine](formula-runtime.md) defines shared types and the `create_draft` entry point.

```rust
pub struct TextOffset(usize); // UTF-8 byte offset
pub struct DraftVersion(u64);

pub struct FormulaDraftState {
    pub version: DraftVersion, // Advances after every successful apply_edits
    pub definition: FormulaDefinition,
    pub output_type: Option<Type>, // No concrete inference → None, never Unknown/Null
    pub diagnostics: Vec<FormulaDiagnostic>, // Cursor-independent; includes direct/transitive self-reference
    pub tokens: Vec<Token>,
}
pub struct CursorHelp {
    pub completions: Vec<Completion>,
    pub signature_help: Option<SignatureHelp>,
}
pub struct TextEdit {
    pub range: Span, // All edits refer to the same pre-edit source
    pub new_text: String,
}
pub struct FormulaEdit {
    pub base_version: DraftVersion, // Must equal current state.version
    pub edits: Vec<TextEdit>, // No overlap
}
pub struct QuickFix {
    pub title: String,
    pub edit: FormulaEdit, // Version bound to the Draft that produced the diagnostic
}
pub struct ApplyEditsResult {
    pub state: FormulaDraftState,
    pub cursor: TextOffset, // Position in the edited source
}
// Final Rust fields for Token, Completion, SignatureHelp, and error variants remain unspecified.
// Current behavior/DTOs are preserved below and in WASM API, not substituted as aliases for these new types.
```

## Commit or discard

```rust
let mut draft = engine.create_draft(formula)?;
let result = draft.apply_edits(edit, cursor)?; // Changes source and reanalyzes, not merely cursor rebasing

if save {
    // Engine reanalyzes against its latest state rather than trusting the Draft's old snapshot.
    let change = engine.upsert_formula(draft.into_definition())?;
    let state = engine.state(); // This and ChangeResult determine the commit outcome
} else {
    drop(draft); // Uncommitted source never entered Engine
}
```

## Current: IDE behavior

Current services process source per request; they do not retain a FormulaDraft. This section owns editing behavior;
[WASM API](formula-runtime-wasm.md) owns JS serialization, coordinate validation, and exceptions.

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

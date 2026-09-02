---
doc_id: specs.formula-references
title: "How do formulas reference properties, and what happens after a rename?"
language: en
source_language: en
counterpart: ./formula-references.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Formula references

[简体中文](formula-references.zh-CN.md)

This Current specification defines how a formula names an input property, which property inputs an integration must provide, and what a property rename does. It also makes the absence of formula identity explicit so property references are not mistaken for persistent formula references.

## Reference a property by its exact name

The only property-reference form is `prop("Name")`. `prop` takes exactly one double-quoted string literal. A computed string such as `prop("Na" + "me")`, a non-string argument, additional arguments, or postfix syntax such as `value.prop("Name")` is not a property reference.

The decoded string must exactly match a property name supplied in the analysis or evaluation context. Matching is case-sensitive: `prop("Title")` and `prop("title")` refer to different names. A name that is not present produces a semantic error during analysis and prevents preparation against that context. [`validate_prop_call`](../../analyzer/src/analysis/mod.rs) and [`Context::lookup`](../../analyzer/src/analysis/mod.rs) are the analysis anchors for these rules.

Property names must be unique within a supplied context for behavior to be specified. If an integration supplies duplicate names, which duplicate is selected is unspecified and must not be treated as a compatibility guarantee.

## Supply every statically referenced property

Preparation discovers property references across the complete expression before row evaluation. It produces each referenced name once, in the order of its first appearance in the source. Repeating `prop("A")` does not create a second requirement.

Discovery does not follow runtime branch selection. For example, `true ? prop("A") : prop("B")` requires both `A` and `B`, even though evaluation only executes the `A` branch. An integration must provide every discovered property with the expected type and row layout before evaluation starts. A missing required property rejects the input as a whole; it is not the same as a present property whose value is null for one row.

The required-property construction is anchored in [`evaluator/src/planner/planner.rs`](../../evaluator/src/planner/planner.rs). Completion, deduplication, first-source ordering, and unselected-branch discovery are covered together in [`evaluator/tests/runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs).

## Resolve a rename by updating source and preparing again

A property reference stores a name in formula source; it does not carry a persistent property identity. Renaming a property in the supplied context therefore does not rewrite formula source or retarget an already prepared formula.

When unchanged source is analyzed or prepared again against a context where `Old` was renamed to `New`, `prop("Old")` is missing. To refer to the renamed property, the integration must change the source to `prop("New")` and prepare it again. A previously prepared formula keeps only the input requirements created for that preparation and does not observe later context changes automatically.

This behavior prevents an accidental identity guarantee: a host application may implement its own rename transaction, but the current project neither coordinates that transaction nor promises that a name change preserves a property reference.

## Formula ID and formula name do not coexist in the production contract

The production analyzer, editor services, WASM boundary, and evaluator expose formula source and property context, but they do not define `FormulaId`, `FormulaName`, a formula-rename API, or a persisted formula-reference DTO. There is therefore no production feature that requires a formula ID and formula name to coexist, and neither can be removed from a production identity model because that model does not currently exist.

The `FormulaId` type under [`examples/vite/src/app/types.ts`](../../examples/vite/src/app/types.ts) only selects one of the demo's formula panels. It is example UI state, not a formula identity contract. Applications that need stored formulas, stable identity across formula renames, or references between formulas must define those facilities outside the current project boundary.

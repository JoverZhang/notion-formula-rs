---
doc_id: project.glossary
title: "Project glossary"
language: en
implementation_status: current
document_status: stable
last_verified: 2026-09-02
---

# Project glossary

This file is the authority for project terminology. Documentation may explain a concept in more
depth, but it should keep these names and preserve code identifiers exactly.

| Term | Code anchor or example | Meaning | Avoid |
| --- | --- | --- | --- |
| formula | `prepare_formula` | The expression represented at a lifecycle stage as source, an analyzed expression, or a prepared plan. A formula is not a persisted record with its own ID. | equation |
| Formula engine | `analyzer`, `ide`, `evaluator` | The project capability that analyzes formulas, assists editing, and evaluates prepared expressions. It is a system-level concept, not a Rust type or a persisted formula record. | formula when the engine is intended |
| formula source | `source: &str` | The formula text supplied to the analyzer. Rust components store it as UTF-8; an external interface may define another coordinate system for offsets. | content when formula text is intended |
| builtin declaration DSL | `builtin_functions!` | The domain-specific syntax embedded in Rust for declaring builtin categories, signatures, support markers, and related metadata. `builtin_fn_macros` parses, validates, and expands this Current DSL; proposed future DSLs are not Current features. | builtin catalog |
| token | `Token` | A syntax unit emitted by the lexer. | word |
| trivia | comments and newlines | Tokens retained for source fidelity but skipped when they do not participate in syntax or semantic analysis. | whitespace when comments are included |
| diagnostic | `Diagnostic` | A lexical, syntactic, or semantic problem associated with a source span. | exception |
| code action | `CodeAction` | An action attached to a diagnostic whose `TextEdit` list implements a quick fix. | edit; fix when no edit is provided |
| span | `Span` | A half-open source range `[start, end)`. The interface that exposes it must define its coordinate unit. | interval without a coordinate unit |
| semantic analysis | `SemanticMap` | AST normalization, type inference, and call validation performed after parsing. | parsing |
| execution plan | `ExecPlan` | Evaluator-owned intermediate representation lowered from an analyzed expression. | AST |
| columnar evaluation | `Column`, `EvalBlock` | Evaluation of a row batch through typed columns, masks, and per-row result state after inputs have been prepared. | row-by-row property lookup |
| row batch | `RowBatch` | Ordered rows evaluated under one prepared formula and one runtime snapshot. | table |
| execution mask | `Mask` | Rows active for a particular control-flow step. | null bitmap |
| validity | `Validity` | Whether each successful row contains a non-null value. | row success |
| row success | `EvalBlock.ok` | Whether evaluation succeeded for each row, independently of its execution mask and validity. | validity |
| input slot | `InputSlot` | Dense input identity local to one prepared layout. | property ID |
| required-column manifest | `RequiredColumn[]` | Complete, deduplicated property dependencies that a caller must prepare before evaluation. | runtime lookup list |
| host system | `Property`, `EvalInputsBuilder` | The integrating application that owns non-formula fields, supplies their names and types for analysis, and provides typed column values before evaluation. | evaluator when referring to external data ownership |

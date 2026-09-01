---
doc_id: project.glossary
title: "Project glossary"
language: en
implementation_status: current
document_status: stable
last_verified: 2026-09-01
---

# Project glossary

This file is the authority for project terminology. Documentation may explain a concept in more
depth, but it should keep these names and preserve code identifiers exactly.

| Term | Code anchor or example | Meaning | Avoid |
| --- | --- | --- | --- |
| formula | `prepare_formula` | Formula source together with an analyzed or prepared representation. A formula is not a persisted record with its own ID. | equation |
| formula source | `source: &str` | The formula text supplied to the analyzer. Rust components store it as UTF-8; an external interface may define another coordinate system for offsets. | content when formula text is intended |
| token | `Token` | A syntax unit emitted by the lexer. | word |
| trivia | comments and newlines | Tokens retained for source fidelity but skipped when they do not participate in syntax or semantic analysis. | whitespace when comments are included |
| diagnostic | `Diagnostic` | A syntax or semantic problem associated with a source span. | exception |
| code action | `CodeAction` | An edit attached to a diagnostic that can implement a quick fix. | fix when no edit is provided |
| span | `Span` | A half-open source range `[start, end)`. The interface that exposes it must define its coordinate unit. | interval without a coordinate unit |
| semantic analysis | `SemanticMap` | Type inference and call validation performed after parsing. | parsing |
| execution plan | `ExecPlan` | Evaluator-owned intermediate representation lowered from an analyzed expression. | AST |
| row batch | `RowBatch` | Ordered rows evaluated under one prepared formula and one runtime snapshot. | table |
| execution mask | `Mask` | Rows active for a particular control-flow step. | null bitmap |
| validity | `Validity` | Whether each successful row contains a non-null value. | row success |
| row success | `EvalBlock.ok` | Whether evaluation succeeded for each row, independently of its execution mask and validity. | validity |
| input slot | `InputSlot` | Dense input identity local to one prepared layout. | property ID |
| required-column manifest | `RequiredColumn[]` | Complete, deduplicated property dependencies that a caller must prepare before evaluation. | runtime lookup list |

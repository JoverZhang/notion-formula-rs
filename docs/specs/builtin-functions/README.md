---
doc_id: specs.builtin-functions
title: "Builtin function specification"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Builtin function specification

[简体中文](README.zh-CN.md)

Builtin functions extend the formula language with text, number, date, list, control-flow, and row-context operations. This document defines the common behavior that formula authors and integrations may rely on. It does not replace the declarations that define the available functions.

## The Rust declarations are the catalog

[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs) is the only complete catalog of builtin names and signatures. Declaration order is stable and is preserved when the supported callable set is constructed. Documentation must not copy or render a second complete list.

A declaration marked `#[unsupported]` is excluded from the callable set. Calling its name has the same observable result as calling any other unknown function: analysis reports an unknown function. There is no separate unsupported-function category in the formula interface.

The catalog is shared by semantic analysis, editor services, and evaluator contract generation. A function is part of the current formula surface only when its declaration is supported; a Rust declaration alone is not a promise that every syntactically similar upstream function exists here.

## Calls follow the declared argument shape

Supported functions use ordinary calls such as `length(value)`. A subset also supports postfix syntax such as `"hello".substring(1)`. Postfix syntax is equivalent to placing the receiver in the first argument slot. It is available only when the signature has a deterministic first slot and still expects at least one argument position beyond that receiver. A parsed member call that does not meet this rule is not silently treated as a normal call.

Signatures can combine fixed parameters, optional parameters, repeated parameter groups, and a tail after the repeated groups. A call must first match that shape. Too few, too many, or incomplete repeated arguments produce an argument-shape diagnostic before individual argument types are checked.

Some parameters bind a generic type, accept a union of types, or represent an implicit function used by list and binding operations. The same binding is reused across the call, so observed argument types can refine later parameters and the result type. [`builtin_fn/src/resolution.rs`](../../../builtin_fn/src/resolution.rs) is the implementation anchor for these shared shape and type rules.

## Analysis is useful but not a runtime gate

Semantic analysis checks known argument types against the resolved signature. An incomplete expression, an unknown value, or a type that still contains `unknown` remains indeterminate rather than becoming an immediate type mismatch. This supports editor feedback while a formula is being written, but it does not prove that every row will evaluate successfully.

Shape errors take precedence over argument type errors. Once the shape is valid, a known incompatible argument produces a type diagnostic. Diagnostic prose is descriptive rather than a machine-readable compatibility key.

The analyzer applies these rules in [`analyzer/src/analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs). Formula syntax and general operator behavior are owned by the formula-language specification rather than this document.

## Evaluation may be eager or controlled

Ordinary value functions evaluate and materialize their active arguments before running the function. Controlled functions receive unevaluated plans and decide which branches, bound expressions, or list elements are needed for each active row. This is what lets conditional and callback-based functions avoid errors in work that was not selected.

Null and failure behavior is defined by each function family; there is no universal rule that every builtin propagates null. For example, some functions interpret null as empty input, while ordinary typed operations commonly return null when a required value is null. Invalid regexes, dates, numeric domains, or other arguments can fail the affected row.

Runtime failure is row-local. A failing active row does not turn successful rows into failures, and an expression skipped by controlled evaluation does not contribute an error. The value and controlled execution paths are anchored in [`evaluator/src/kernels/value.rs`](../../../evaluator/src/kernels/value.rs) and [`evaluator/src/kernels/controlled.rs`](../../../evaluator/src/kernels/controlled.rs).

## Time and row identity come from one evaluation

`now()` reads the evaluation's frozen timestamp. `today()` derives local midnight from the same timestamp and configured timezone offset. Repeated calls within one evaluation therefore share the same clock snapshot.

`id()` returns the current row identifier as text. It is not a formula identifier and does not establish formula identity, persistence, or rename behavior.

## Contract boundaries

The stable surface described here is the formula-visible call and evaluation behavior. It excludes:

- a generated Markdown catalog or a duplicated name/signature list;
- the declaration DSL, procedural macros, signature projection structures, and generated evaluator ABI;
- stability guarantees for Rust `pub` APIs;
- a promise that every function behaves like Notion or any other upstream system; and
- exhaustive guarantees inferred only from the presence of a catalog fixture.

Representative catalog invariants live in [`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs). Per-function runtime examples live under [`evaluator/tests/builtins/`](../../../evaluator/tests/builtins/), but those fixtures do not turn undocumented failure cases into compatibility promises.

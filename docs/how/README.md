---
doc_id: how.index
title: "Implementation map"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Implementation map

[简体中文](README.zh-CN.md)

The workspace turns one set of builtin declarations and one formula source into analysis
artifacts, editor responses, or evaluated rows. Two relationship types connect the crates:

```text
Declaration and build-time path
builtin_fn_macros --macro expansion--> builtin_fn
builtin_fn --supported declarations--> evaluator/build.rs --generated ABI--> evaluator

Request and data path
source --analysis--> analyzer --resolved expression--> evaluator
                        |
                        +--analysis artifacts--> ide
                              |
               analyzer + ide --WASM facade--> analyzer_wasm --DTOs--> Vite example
```

`builtin_fn_macros` validates and lowers each declaration block. `builtin_fn` owns the
structured catalog and shared call resolution. `analyzer` produces syntax and semantic
artifacts. `ide` projects those artifacts into editor help and edits, while `evaluator`
uses its build script to generate the builtin ABI, then lowers validated formulas into prepared
synchronous execution. `analyzer_wasm` converts Analyzer and IDE data across the JavaScript
boundary; the Vite example supplies presentation policy.

## Continue with the owning component

- [builtin_fn_macros](builtin_fn_macros/README.md): declaration parsing, validation, lowering,
  and diagnostics
- [builtin_fn](builtin_fn/README.md): catalog construction, parameter shapes, generics, and
  call resolution
- [analyzer](analyzer/README.md): lexing, parsing, recovery, semantic analysis, spans, and
  diagnostics
- [ide](ide/README.md): completion, signature help, formatting, and edit application
- [evaluator](evaluator/README.md): preparation, input contracts, masks, kernels, and
  generated builtin dispatch
- [analyzer_wasm](analyzer_wasm/README.md): UTF-16 conversion, DTO adaptation, exports, and
  packaging
- [Vite example](examples/vite/README.md): CodeMirror integration, presentation state, and
  browser tests

User-visible guarantees belong in the [specifications](../specs/README.md). Use the
[testing guide](../contributing/testing.md) to choose checks for an implementation change.

---
doc_id: specs.index
title: "Current behavior specifications"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Current behavior specifications

[简体中文](README.zh-CN.md)

Choose the specification by the behavior you need to rely on:

- [Formula language](formula-language.md): source forms, operators, analysis, and evaluation
  outcomes
- [Formula references](formula-references.md): property and formula reference behavior,
  including rename handling
- [Builtin functions](builtin-functions/README.md): call shapes, type resolution, controlled
  evaluation, and row-local failures
- [Editor services](editor-services.md): diagnostics, completion, signature help, formatting,
  and edit application
- [WASM API](wasm-api.md): JavaScript-facing configuration, DTOs, UTF-16 positions,
  operations, and errors

These documents specify current user-visible behavior. Read the
[implementation map](../how/README.md) when you need internal Rust types, algorithms, or crate
boundaries. The [documentation policy](../../DOCUMENTATION.md) defines how the two layers are
maintained.

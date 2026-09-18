---
doc_id: specs.index
title: "Specification index"
language: en
source_language: zh-CN
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-18
---

# Specification index

[简体中文](README.zh-CN.md)

Five consumer-facing boundaries, not a crate inventory. Each rule has one owner; other documents link to it.

| Document | Contract owned | Status |
| --- | --- | --- |
| [FormulaEngine](formula-runtime.md) | Definitions, dependency compilation, state, columnar evaluation | Planned |
| [WASM API](formula-runtime-wasm.md) | Worker, thin clients, DTOs, coordinates, lifetime | Planned; Current Analyzer kept separately |
| [IDE / FormulaDraft](formula-draft.md) | Drafts, help, quick fixes, format, edits, commit/discard | Planned; Current IDE kept separately |
| [Formula grammar](formula-language.md) | EBNF, property references, operators, nulls, failure boundaries | Current |
| [Builtins](builtin-functions.md) | Supported functions, signature notation, call rules | Current |

```text
Current = observable behavior of existing implementations.
Planned = an interface awaiting implementation; a synced English version does not mean it has shipped.
Bodyless Rust impl blocks are interface declarations, not independently compilable ordinary Rust modules.
Only explicitly marked header=<key> blocks feed header generation; these specifications are not wired into it yet.
```

See [spec-codegen](../contributing/spec-codegen.md) for generation rules
and the [implementation guides](../how/README.md) for algorithms and internal modules.

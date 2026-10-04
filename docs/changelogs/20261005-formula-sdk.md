---
doc_id: changelog.20261005-formula-sdk
title: "Reuse formula clients and query input dependencies"
language: en
source_language: en
counterpart: ./20261005-formula-sdk.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Reuse formula clients and query input dependencies

[简体中文](20261005-formula-sdk.zh-CN.md)

- Type: Added
- Component: FormulaEngine and browser SDK

## Summary

`@notion-formula/sdk` packages the Engine/Draft client, module Worker, TypeScript declarations,
and WASM assets. The Vite example consumes this package. The repository also pins AppFlowy-Web
as an example submodule and provides local SDK staging and build commands for its migration.

`FormulaEngine::required_inputs`, the WASM session's `required_inputs`, and the browser client's
`requiredInputs` expose the transitive Input dependencies of selected formulas. Ready formulas
have complete static dependency sets; NotReady formulas expose known references only.

## Compatibility notes

- Import reusable browser clients and Engine DTOs from `@notion-formula/sdk` instead of files
  under `examples/vite`. Existing evaluation, FIFO, Draft, and resource-release behavior is preserved.
- The query is additive. Evaluation still requires columns for every registered Input, and invalid
  query selections use the existing `EvaluateInputError` / `EVALUATE_INPUT` contract.
- Adding the AppFlowy submodule does not complete its formula migration; progress is tracked in #67.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current FormulaEngine](../specs/formula-engine.md)
- [Current WASM API](../specs/wasm-api.md)
- [Build the AppFlowy example](../contributing/appflowy-example.md)

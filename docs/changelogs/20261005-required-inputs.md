---
doc_id: changelog.20261005-required-inputs
title: "Query formula input dependencies"
language: en
source_language: en
counterpart: ./20261005-required-inputs.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# Query formula input dependencies

[简体中文](20261005-required-inputs.zh-CN.md)

- Type: Added
- Component: FormulaEngine, WASM, and browser SDK

## Summary

`FormulaEngine::required_inputs`, the WASM session's `required_inputs`, and the
browser client's `requiredInputs` expose the transitive Input dependencies of
selected formulas. Hosts can use this query to choose which external values to load.
Ready formulas have complete static dependency sets; NotReady formulas expose known references only.

## Compatibility notes

The query is additive. Evaluation still requires columns for every registered Input,
and invalid query selections use the existing `EvaluateInputError` / `EVALUATE_INPUT` contract.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Current FormulaEngine](../specs/formula-engine.md)
- [Current WASM API](../specs/wasm-api.md)

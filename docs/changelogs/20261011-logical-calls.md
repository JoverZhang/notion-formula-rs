---
doc_id: changelog.20261011-logical-calls
title: "Support lazy logical calls and word operators"
language: en
source_language: en
counterpart: ./20261011-logical-calls.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# Support lazy logical calls and word operators

[简体中文](20261011-logical-calls.zh-CN.md)

- Type: Added
- Component: FormulaEngine, Draft analysis and builtin functions

## Summary

Formulas accept `and(...)` and `or(...)` calls with one or more Boolean/null expressions,
and the contextual infix spellings `a and b` and `a or b`. Calls evaluate from left to right
and skip expressions after the result is determined. Formatting preserves the word spelling,
and completion offers expressions after a word operator.

## Compatibility notes

- Precedence, Boolean/null behavior and skipped-branch errors follow the existing `&&` and `||` rules.
- The function names remain identifiers in prefix position. `not(true)` continues to use the prefix operator.
- Engine and WASM data representations do not change.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin declarations](../specs/builtin-functions.md)
- [Language rules](../specs/formula-language.md)

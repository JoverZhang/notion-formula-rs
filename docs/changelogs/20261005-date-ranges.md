---
doc_id: changelog.20261005-date-ranges
title: "Preserve date ranges and time visibility"
language: en
source_language: en
counterpart: ./20261005-date-ranges.zh-CN.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Preserve date ranges and time visibility

[简体中文](20261005-date-ranges.zh-CN.md)

- Type: Added
- Component: FormulaEngine, date functions and WASM DTOs

## Summary

`DateValue` preserves start, optional end and time visibility across Engine columns, nested values
and the browser Worker. `dateRange`, `dateStart` and `dateEnd` are now supported; shifting a range
preserves both endpoints, and `dateBetween` uses local midnight for dates whose time is hidden.

## Compatibility notes

- Native `Value::Date(i64)`, `Column::Date(ColumnData<i64>)` and JavaScript `Date` bigint payloads
  remain valid as single dates with visible time. Integrations matching every value or column
  variant must handle the additional `DateValue` variant; its WASM endpoints use exact bigints.
- Both representations retain the `Date` semantic type and column kind. Date-only `parseDate`
  and `today` now return metadata with hidden time; `format` hides their clock and displays both
  range endpoints. Generic text conversion retains scalar timestamps and both raw range endpoints.
- Copying input endpoints remains lossless even outside the supported calendar range. Date
  operations validate both endpoints and return `DateOutOfRange` row errors when needed.

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [FormulaEngine value contract](../specs/formula-engine.md#type-definitions)
- [Date functions](../specs/builtin-functions.md#date)
- [WASM DTOs](../specs/wasm-api.md)

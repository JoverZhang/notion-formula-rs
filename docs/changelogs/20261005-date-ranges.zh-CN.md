---
doc_id: changelog.20261005-date-ranges
title: "保留日期范围与时间显示标记"
language: zh-CN
source_language: en
counterpart: ./20261005-date-ranges.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 保留日期范围与时间显示标记

[English](20261005-date-ranges.md)

- Type: Added
- Component: FormulaEngine、日期函数与 WASM DTO

## Summary

`DateValue` 在 Engine 列、嵌套值和浏览器 Worker 之间保留起点、可选终点和时间显示标记。
`dateRange`、`dateStart` 和 `dateEnd` 现已可用；平移日期范围时保留两个端点，`dateBetween`
对隐藏时间的日期按本地零点计算。

## Compatibility notes

- 原生 `Value::Date(i64)`、`Column::Date(ColumnData<i64>)` 及 JavaScript `Date` bigint 载荷
  继续有效，表示显示时间的单个日期。穷举值或列变体的集成代码需要处理新增的 `DateValue`
  变体；其 WASM 端点使用精确的 bigint。
- 两种形式的语义类型和列种类仍为 `Date`。只含日期的 `parseDate` 与 `today` 现在返回隐藏时间
  的元数据；`format` 隐藏其时分，并显示范围的两个端点。通用文本转换继续保留单个日期的时间戳，
  并完整保留范围的两个原始端点。
- 输入端点即使超出支持的公历范围，仍可原样复制。日期操作会校验两个端点，必要时返回
  `DateOutOfRange` 行错误。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [FormulaEngine 值契约](../specs/formula-engine.zh-CN.md#类型定义)
- [日期函数](../specs/builtin-functions.zh-CN.md#date)
- [WASM DTO](../specs/wasm-api.zh-CN.md)

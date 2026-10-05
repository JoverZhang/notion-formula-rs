---
doc_id: changelog.20261005-appflowy-builtins
title: "补齐原生公式内置函数兼容性"
language: zh-CN
source_language: en
counterpart: ./20261005-appflowy-builtins.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 补齐原生公式内置函数兼容性

[English](20261005-appflowy-builtins.md)

- Type: Added
- Component: FormulaEngine、Draft 分析与内置函数

## Summary

原生公式新增惰性 `and`/`or` 函数与中缀运算符、顺序异构 `lets`、保留纯文本的 `style`/`unstyle`，以及列表回调
中从零开始的 `index`。`ifs` 允许省略 else，`formatNumber` 允许省略 precision 并支持
`humanize`；`formatDate` 支持方括号字面量、周编号、序数与星期标记。字符串加法现推断为 String。

## Compatibility notes

- Boolean 和空值规则仍与原生逻辑运算符一致。省略 else 的 `ifs` 在未匹配时返回普通 null；
  样式名称照常求值并检查类型，随后忽略。
- `concat` 仍至少接受两个列表，数值聚合函数仍至少接受一个参数。`link` 仍不支持，
  `not(true)` 使用已有的前缀运算符。
- Engine 与 WASM DTO 结构不变。顺序绑定降级不会改变 Draft 属性引用的原始顺序与源码范围。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin 声明](../specs/builtin-functions.zh-CN.md)
- [语言规则](../specs/formula-language.zh-CN.md)

---
doc_id: changelog.20261011-logical-calls
title: "支持惰性逻辑函数与单词运算符"
language: zh-CN
source_language: en
counterpart: ./20261011-logical-calls.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# 支持惰性逻辑函数与单词运算符

[English](20261011-logical-calls.md)

- Type: Added
- Component: FormulaEngine、Draft 分析与内置函数

## Summary

公式支持接受一个或多个 Boolean/null 表达式的 `and(...)`、`or(...)` 调用，
以及中缀写法 `a and b`、`a or b`。函数从左到右求值，在结果确定后跳过剩余表达式。
格式化保留单词写法，补全会在单词运算符后提供表达式候选。

## Compatibility notes

- 优先级、Boolean/null 行为和跳过分支的错误规则沿用已有的 `&&`、`||` 约定。
- 这些函数名在前缀位置仍是标识符。`not(true)` 继续使用前缀运算符。
- Engine 与 WASM 的数据表示不变。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Builtin 声明](../specs/builtin-functions.zh-CN.md)
- [语言规则](../specs/formula-language.zh-CN.md)

---
doc_id: specs.formula-language
title: "公式语言支持哪些源码与求值行为？"
language: zh-CN
source_language: en
counterpart: ./formula-language.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 公式语言

[English](formula-language.md)

这份 Current 规格说明公式作者与集成方可以依赖的公式 source 和非 builtin 求值行为。内容包括 literal、expression form、operator、null、lazy branch，以及 best-effort analysis 与受支持 evaluation 之间的边界。单个 builtin 的 call 规则、editor 展示和 transport DTO 不在本文范围内。

Notion-style syntax 只是起始词汇，并不表示兼容性承诺。只有本文明确写出的形式和行为才属于受支持的公式语言。

## 公式 source 只支持这些形式

- Boolean literal 为 `true` 和 `false`。
- Number literal 由十进制数字组成，可以带小数部分；小数点后必须至少有一位数字。也可以带 `e` 或 `E` exponent，exponent 前可以有正负号，后面必须至少有一位数字。例如 `12`、`3.14` 和 `2.5e-3`。
- String literal 使用双引号，支持 `\n`、`\t`、`\"` 和 `\\` 四种 escape。
- List literal 在方括号内放置逗号分隔的 expression，例如 `[1, "x"]`，不支持 trailing comma。
- Identifier 以 `_` 或 Unicode alphabetic character 开头，后续可以是 `_` 或 Unicode alphanumeric character。小写的 `true`、`false` 和 `not` 是 reserved word。
- 圆括号用于 grouping。普通 call 写作 `name(arg1, ...)`；受支持的 function 还可能允许 `receiver.name(arg1, ...)`。只写 member name 而不跟 call parentheses 不受支持。Call argument list 也不允许 trailing comma。
- `//` 开始 line comment，`/* ... */` 包围 block comment；token 之间可以换行。

Source 中没有 null literal 或 date literal。Null 可以来自 input data 或 function result，date 可以来自带 type 的 property 或 function。定义这些形式的 lexer 和 expression parser 位于 [`analyzer/src/lexer/`](../../analyzer/src/lexer/) 与 [`analyzer/src/parser/`](../../analyzer/src/parser/)。

## 按确定的优先级应用 operator

Expression 从高到低按以下优先级结合：

| 优先级 | 形式 | 结合性 |
| --- | --- | --- |
| 最高 | call 与 postfix-call suffix | 在语法允许时从左到右 chaining |
|  | `^` | 右结合 |
|  | prefix `!`、`not`、`-` | prefix |
|  | `*`、`/`、`%` | 左结合 |
|  | `+`、`-` | 左结合 |
|  | `<`、`<=`、`>=`、`>` | 左结合 |
|  | `==`、`!=` | 左结合 |
|  | `&&` | 左结合 |
|  | `||` | 左结合 |
| 最低 | `condition ? then : otherwise` | 右结合 |

圆括号可以覆盖以上顺序。特别是，`2 ^ 3 ^ 2` 表示 `2 ^ (3 ^ 2)`，`-2 ^ 2` 表示 `-(2 ^ 2)`，连续 ternary 会从 `otherwise` branch 向右结合。[`BinOp::infix_binding_power`](../../analyzer/src/parser/ast.rs) 是这套顺序在 parser 中的实现入口。

## Analysis 只提供 best-effort 结果

Analysis 需要在 source 尚未写完时继续提供有用结果。Lexer 或 parser diagnostic 可以与恢复后的 syntax 同时存在；semantic analysis 可以把未绑定 identifier 或无法确定的 operator 标为 `unknown`；两个已知但不同的 branch type 可以形成 union type。

这些结果不是 evaluation 一定成功的编译期证明。例如，`"count: " + 3` 可能推断为 `unknown`，但仍能得到 text；未绑定 identifier 在 analysis 时也可能保持 `unknown`，直到某个 row 真正执行它时才失败。当前 inference 规则的实现入口是 [`analyzer/src/analysis/infer.rs`](../../analyzer/src/analysis/infer.rs)。

受支持的 evaluation 要求 source 没有 lexer 或 parser diagnostic，并且 expression 能通过 semantic preparation。Parser 即使恢复出 syntax tree，也不表示错误 source 可以进入受支持的 evaluation。反过来，inference 不够精确本身也不等于拒绝。Formula diagnostic 的文字用于解释问题，不是 machine-readable compatibility key；集成方不得依赖某句英文的精确写法来分支。

## 普通 operator 逐 row 求值

Operand 成功求值且非 null 后，普通 operator 遵循以下规则：

| Operator | 受支持 operand | Result |
| --- | --- | --- |
| unary `-` | number | 取负后的 number |
| unary `!`、`not` | boolean | 取反后的 boolean |
| `+` | 两个 number | 数值相加 |
| `+` | 任一 operand 为 text | 把另一个 operand 转为 text 后拼接 |
| `-`、`*`、`/`、`%`、`^` | 两个 number | 减法、乘法、除法、余数或幂运算 |
| `==`、`!=` | 任意两个非 null value | value 相等或不等；不同 kind 的 value 不相等 |
| `<`、`<=`、`>=`、`>` | 两个可排序的 number、两个 text、两个 boolean 或两个 date | 同 kind 排序 |

Text 拼接时，整数不会带 `.0`，boolean 使用小写 `true` 或 `false`，date 使用 epoch-millisecond integer，list 使用方括号和逗号分隔，其中每个 item 递归使用同一转换。Number 按数值排序，text 按字典序排序，boolean 中 `false` 位于 `true` 之前，date 按时间先后排序。`NaN` 不可排序；只要 relational comparison 的任一 operand 为 `NaN`，对应 row 就会产生 type failure。

除以零或对零取余会让对应 row 失败。其他不受支持的 operand 组合会产生 row-level type failure。Equality 是例外：两个非 null value 的 kind 不同时，结果是不相等，而不是失败。这些规则实现在 [`evaluator/src/runtime/operators.rs`](../../evaluator/src/runtime/operators.rs)。

## 区分 null、failure 与被跳过的工作

Null 表示成功但没有 value，不是 evaluation error。Operand 已经成功求值时，unary operator 和普通非 logical binary operator 遇到必需 operand 为 null 会返回 null。

求值顺序会决定哪些结果可观察：

- 非 logical binary operator 会为到达该 expression 的每个 row 求值两侧 operand。即使另一侧为 null，任一 operand 的 error 仍会让该 row 失败。
- List literal 会求值所有 item。只要一个 item 失败，该 row 就失败；如果没有 failure、但至少一个 item 为 null，整个 list expression 为 null。
- `left && right` 支持 boolean 或 null operand。只有 `left` 为 `true` 时才求值 `right`；左侧为 `false` 或 null 时，不求值右侧并直接得到 `false`。若左侧为 `true`、右侧为 null，则结果为 null。
- `left || right` 支持 boolean 或 null operand。左侧为 `true` 时跳过右侧并得到 `true`；左侧为 `false` 或 null 时求值右侧，并返回其 boolean 或 null 结果。
- `condition ? then : otherwise` 在 condition 为 `true` 时只求值 `then`，为 `false` 或 null 时只求值 `otherwise`。受支持的 condition domain 只有 boolean 与 null；其他 condition type 的行为不属于本文契约。

被 `&&`、`||` 或 ternary branch 跳过的 expression 不会产生 runtime error。这种 laziness 只作用于 expression execution；property reference 的发现规则由 [formula-reference 规格](formula-references.zh-CN.md)维护。

## 分开处理整条公式失败与 row failure

如果 source、property context 或 required input 无法完成 preparation，系统会在产生任何 row result 之前拒绝公式或本次 evaluation。Evaluation 开始后，divide-by-zero 和 type error 等 runtime failure 只影响真正执行到它的 row；其他 row 仍可以返回 value 或 null。因此 null 不表示 failure，一个 row 失败也不会让成功的 row 一起失效。

[`evaluator/tests/runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs) 覆盖了彼此独立的 null 与 row-failure outcome。Builtin function 可以定义更具体的 null 和 control-flow behavior；这些例外由 builtin function 规格维护。

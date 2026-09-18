---
doc_id: specs.formula-language
title: "公式文法与求值规则"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-language.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-18
---

# 公式文法与求值规则

[English](formula-language.md) · [规格索引](README.zh-CN.md)

Current：描述本仓库接受的完整表达式，不承诺与上游 Notion 完全兼容。IDE 对残缺源码的恢复不扩展此文法。

## EBNF

```ebnf
(* | 选择；, 连接；[ ] 可选；{ } 重复；? ... ? 为词法条件；- 为集合差。 *)
(* trivia 中的 \t、\r、\n 分别表示 tab、CR、LF 控制字符。 *)
source      = expression, EOF ;
expression  = conditional ;
conditional = disjunction, [ "?", expression, ":", conditional ] ;
disjunction = conjunction, { "||", conjunction } ;
conjunction = equality, { "&&", equality } ;
equality    = comparison, { ( "==" | "!=" ), comparison } ;
comparison  = addition, { ( "<" | "<=" | ">" | ">=" ), addition } ;
addition    = product, { ( "+" | "-" ), product } ;
product     = unary, { ( "*" | "/" | "%" ), unary } ;
unary       = ( "!" | "not" | "-" ), unary | power ;
power       = postfix, [ "^", unary ] ;
postfix     = primary, { ".", identifier, arguments } ;
primary     = number | string | boolean | identifier, [ arguments ]
            | "(", expression, ")" | "[", [ expressions ], "]" ;
arguments   = "(", [ expressions ], ")" ;
expressions = expression, { ",", expression } ; (* 不接受尾逗号 *)

boolean     = "true" | "false" ;
number      = digits, [ ".", digits ], [ ( "e" | "E" ), [ "+" | "-" ], digits ] ;
digits      = digit, { digit } ;
digit       = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
string      = '"', { string-char | escape }, '"' ;
string-char = ? 除双引号、反斜杠以外的 Unicode scalar；包括原始换行 ? ;
escape      = '\', ( "n" | "t" | '"' | '\' ) ;
identifier  = identifier-token - keyword ;
identifier-token = ( "_" | letter ), { "_" | alphanumeric } ;
letter      = ? Rust char::is_alphabetic ? ;
alphanumeric = ? Rust char::is_alphanumeric ? ;
keyword     = "true" | "false" | "not" ;

(* token 之间允许 trivia；词法 token 内部不允许插入 trivia。 *)
trivia        = " " | "\t" | "\r" | "\n" | line-comment | block-comment ;
line-comment  = "//", { ? 除 LF 以外的 scalar ? } ;
block-comment = "/*", ? 到第一个 */ 为止的文本；不可嵌套 ?, "*/" ;
EOF           = ? 输入结束 ? ;
```

```text
-2^2        == -(2^2)       // ^ 高于前缀运算符；^ 和 ?: 右结合，其余二元运算符左结合
2^3^2       == 2^(3^2)
2^-2        == 2^(-2)
a?b:c?d:e   == a?b:(c?d:e)

3.method()                  // 数字中的 . 后必须紧跟数字，才属于小数部分
.5                          // 不支持
f(1).method(2)              // 可链式 member call；能否求值还取决于 builtin 的 postfix 能力
(f)(1), f()(1), value.field // 不支持：普通 call 的 callee 必须是 identifier；无裸 member access
null、date literal          // 不支持；空值和日期由 property 或函数产生
```

## Property reference

```text
prop("Name")                // 恰好一个双引号字符串字面量，按解码后的文本查找
prop("Na" + "me")           // 不是有效 property reference
prop(1), prop("A", "B")     // 同上
x.prop("Name")             // 不作为 property reference 识别

lookup
  Current 使用 context 中的精确、区分大小写的 property name。
  名称必须唯一；重复名称的选择结果未定义。缺失名称 → semantic diagnostic，prepare 失败。
  prepare 收集所有分支中的引用，包括运行时将跳过的分支；去重后按源码首次出现顺序排列。
  每个必需输入都要存在且满足类型/列布局；缺失输入不等于 null。

rename
  不自动改写 source，也不将已 prepare 的公式重新指向新名称。
  宿主负责更新 source 并重新 prepare；已有 prepared formula 仍要求原 context 的输入。

boundary
  Current 产品没有持久化 FormulaId/FormulaName、formula reference 或 rename API。
  demo 的 FormulaId 只是界面身份；Planned Engine 的 ID/依赖模型不能当作 Current 行为。
```

Planned 定义见 [FormulaEngine](formula-runtime.zh-CN.md)。

## 运算与空值

```text
op             非 null 操作数与成功结果
-x             number → number
!x / not x     boolean → boolean
a + b          两个 number → 相加；任一为 string → 两侧转文本后拼接
a - * / % ^ b  两个 number → number；除数/模数为 0 → 行错误
a == b / !=    任意非 null 值；不同 value kind 不相等
a < <= >= > b  同 kind 的 number/string/boolean/date → boolean；NaN 无法排序 → 行类型错误

表外的一元/非逻辑二元非 null 操作数组合 → 行类型错误；==/!= 接受不同 kind，不属于此类。
比较顺序：number 按数值，string 按字典序，boolean 为 false < true，date 按时间先后。
文本转换：整数无 .0，boolean 小写，date 为 epoch milliseconds 整数，
          list 用方括号和逗号包围递归转换后的元素。

非逻辑运算：两侧都求值；只要有错误就报错，否则有 null 就返回 null。
一元运算：null → null。
list literal：求值所有元素；有错误则报错，否则任一 null → 整个 list expression 为 null。

a && b：a=true → 求值 b；a=false/null → false，跳过 b；求值到 b=null → null。
a || b：a=true → true，跳过 b；a=false/null → 求值 b，结果为 boolean/null。
a ? b : c：a=true → b；a=false/null → c；condition 只接受 boolean/null。
被跳过的 expression 不产生行错误；这不改变 prepare 时发现全部 property 的规则。
```

## 分析与失败边界

```text
analyze(source)   → 尽力恢复诊断、tokens、类型；不是“可执行证明”
prepare / 输入校验 → source/schema/输入问题在逐行求值前失败
evaluate          → 运行时问题是逐行错误；其他行可继续

Current 推断允许 unknown、union，以及嵌套 unknown。
未知标识符或不确定推断不必立即拒绝；语法诊断阻止求值。
例如 "count: " + 3 可推断为 unknown，但运行时仍可拼接文本。
诊断 message 不是机器接口；Planned Engine Ready 必须有明确 Type，是更严格的目标契约。
```

函数调用签名和受控求值见 [builtin](builtin-functions.zh-CN.md)。
实现锚点：[lexer](../../analyzer/src/lexer/mod.rs)、[parser](../../analyzer/src/parser/expr.rs)、
[优先级](../../analyzer/src/parser/ast.rs)、[运算符](../../evaluator/src/runtime/operators.rs)、
[引用与短路测试](../../evaluator/tests/runtime_structure.rs)。

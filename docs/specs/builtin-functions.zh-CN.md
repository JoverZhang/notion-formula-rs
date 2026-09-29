---
doc_id: specs.builtin-functions
title: "Builtin 函数签名"
language: zh-CN
source_language: zh-CN
counterpart: ./builtin-functions.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-29
---

# Builtin 函数签名

[English](builtin-functions.md) · [规格索引](README.zh-CN.md)

Current：下表列出全部受支持声明；执行入口仍由 [builtins.rs](../../builtin_fn/src/builtins.rs) 驱动。
`builtin` 代码块是签名目录，不是可执行的公式，也尚未接入 Markdown → Rust 生成。
下方 Planned 章节规定 FormulaEngine 待实现的行为，不表示当前 evaluator 已支持。

## 签名记法

以下 EBNF 定义本页使用的记法，不是内部 macro DSL 的完整文法。

```ebnf
signature  = name, [ generics ], "(", [ parameters ], ")", "->", type, ";" ;
generics   = "<", generic, { ",", generic }, ">" ;
generic    = name, [ ":", "Variant" ] ;
parameters = parameter, { ",", parameter }, [ "," ] ;
parameter  = name, [ "?" ], ":", type
           | "repeat", "(", "min", "=", integer, ")", "{", parameters, "}" ;
type       = member, { "|", member } ;
member     = ( scalar | name | "Ident", "<", type, ">"
             | "(", [ bindings ], ")", "->", type ), { "[]" } ;
bindings   = name, ":", type, { ",", name, ":", type } ;
scalar     = "number" | "string" | "boolean" | "date" | "any" ;
name       = ? ASCII identifier; keywords are allowed in this notation ? ;
integer    = ? Nonnegative decimal integer ? ;
```

```text
x?: T                  // 可省略参数，不是“所有 T 不可为 null”
T[] / A | B            // list / union；T、U 在同一次调用内绑定
Ident<T>               // 绑定标识符，不是普通字符串参数
() -> T                // 延迟表达式
(current: T) -> U      // 带隐式绑定的表达式；调用者不写 lambda 语法
repeat(min = n) {...}  // 整组参数重复至少 n 次；不是一个 list 参数
                       // 声明中的尾逗号不意味着公式调用允许尾逗号

Current flat：递归去掉嵌套 list 层，收集并归一化 union 叶子类型，结果仍为 list。
      没有可用 list 类型时沿用默认返回类型；不能只按表面的 T[] → T[] 推断。
```

替代此规则的单层展开与类型推断见 [Planned flat](#planned-flat)。

## 支持的函数

### General

```builtin
if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T;
ifs<T: Variant>(repeat(min = 1) { condition: boolean, value: () -> T }, else: () -> T) -> T;
empty(value?: any) -> boolean;
length(value: string | any[]) -> number;
format(value: any) -> string;
equal(a: any, b: any) -> boolean;
unequal(a: any, b: any) -> boolean;
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;
```

### Text

```builtin
substring(text: string, start: number, end?: number) -> string;
contains(text: string, search: string) -> boolean;
test(text: string, regex: string) -> boolean;
match(text: string, regex: string) -> string[];
replace(text: string, regex: string, replacement: string) -> string;
replaceAll(text: string, regex: string, replacement: string) -> string;
lower(text: string) -> string;
upper(text: string) -> string;
trim(text: string) -> string;
repeat(text: string, times: number) -> string;
padStart(text: string | number, length: number, pad: string) -> string;
padEnd(text: string | number, length: number, pad: string) -> string;
concat<T>(repeat(min = 2) { lists: T[] }) -> T[];
join<T>(list: T[], separator: string) -> string;
split(text: string, separator: string) -> string[];
```

### Number

```builtin
formatNumber(value: number, format: string, precision: number) -> string;
add(a: number, b: number) -> number;
subtract(a: number, b: number) -> number;
multiply(a: number, b: number) -> number;
mod(a: number, b: number) -> number;
pow(base: number, exp: number) -> number;
divide(a: number, b: number) -> number;
min(repeat(min = 1) { values: number | number[] }) -> number;
max(repeat(min = 1) { values: number | number[] }) -> number;
sum(repeat(min = 1) { values: number | number[] }) -> number;
median(repeat(min = 1) { values: number | number[] }) -> number;
mean(repeat(min = 1) { values: number | number[] }) -> number;
abs(value: number) -> number;
round(value: number, places?: number) -> number;
ceil(value: number) -> number;
floor(value: number) -> number;
sqrt(value: number) -> number;
cbrt(value: number) -> number;
exp(value: number) -> number;
ln(value: number) -> number;
log10(value: number) -> number;
log2(value: number) -> number;
sign(value: number) -> number;
pi() -> number;
e() -> number;
toNumber(value: any) -> number;
```

### Date

```builtin
now() -> date;
today() -> date;
minute(date: date) -> number;
hour(date: date) -> number;
day(date: date) -> number;
date(date: date) -> number;
week(date: date) -> number;
month(date: date) -> number;
year(date: date) -> number;
dateAdd(date: date, amount: number, unit: string) -> date;
dateSubtract(date: date, amount: number, unit: string) -> date;
dateBetween(a: date, b: date, unit: string) -> number;
timestamp(date: date) -> number;
fromTimestamp(timestamp: number) -> date;
formatDate(date: date, format: string) -> string;
parseDate(text: string) -> date;
```

### List

```builtin
at<T>(list: T[], index: number) -> T;
first<T>(list: T[]) -> T;
last<T>(list: T[]) -> T;
slice<T>(list: T[], start: number, end?: number) -> T[];
splice<T>(list: T[], startIndex: number, deleteCount: number, repeat(min = 0) { items: T }) -> T[];
sort<T>(list: T[]) -> T[];
reverse<T>(list: T[]) -> T[];
unique<T>(list: T[]) -> T[];
includes<T>(list: T[], value: T) -> boolean;
map<T, U>(list: T[], mapper: (current: T) -> U) -> U[];
filter<T>(list: T[], predicate: (current: T) -> boolean) -> T[];
find<T>(list: T[], predicate: (current: T) -> boolean) -> T;
findIndex<T>(list: T[], predicate: (current: T) -> boolean) -> number;
some<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
every<T>(list: T[], predicate: (current: T) -> boolean) -> boolean;
count<T>(list: T[], predicate: (current: T) -> boolean) -> number;
flat<T>(list: T[]) -> T[];
```

### Special

```builtin
id() -> string;
```

```text
People 当前无受支持函数。以下声明不进入可调用/补全集合：
  and / or / not          // 用 && / || / not 运算符表达
  lets                    // 缺少异构、顺序绑定模型
  link / style / unstyle  // 尚无 Link / StyledText 类型
  dateRange / dateStart / dateEnd // 尚无 DateRange 类型
  name / email            // 尚无 person 名称/邮件输入
调用 unsupported 声明按未知函数处理，不提供独立的 unsupported 错误类别。
类别顺序固定为 General、Text、Number、Date、People、List、Special；各类别保留声明顺序。
```

## 调用与执行

```text
shape → type → execution
  正式求值前须通过语法和语义校验；unknown 本身不等于校验失败。
  先校验固定/可选/repeat/尾部参数形态，再校验类型；已知类型不匹配产生 diagnostic。
  泛型、union 和隐式函数参数参与整次调用的类型绑定。
  unknown（包括嵌套 unknown）表示未确定，不立即视为类型不匹配；通过分析不保证逐行成功。

postfix
  首参数槽位确定，且接收 receiver 后仍有其他参数位置，才具备 postfix 能力。
  receiver.f(args) 等价于 f(receiver, args)，还需类型兼容。
  parser 接受 member-call 语法，不表示任何 builtin 都能 postfix；不支持时不回退成普通调用。

evaluation
  value 参数先求值；受控分支、binder、callback 参数按函数语义选择求值。
  对活动行先完整构造 list，再执行 callback；构造错误不能被 callback 短路隐藏。
  callback 只处理活动的行/元素组合；find/findIndex/some/every 可提前结束，不推广到其他函数。
  null 规则按函数族定义，不能从签名推导统一 null 传播规则。
  无效 regex、日期或值域等运行时问题是行错误；未执行的分支/元素不产生错误。

runtime
  now()   → 本次求值冻结的时间
  today() → 同一时间快照与时区偏移所对应的本地零点
  id()    → 当前行 ID 的文本，不是 formula ID
```

## Planned 空值与列表

```builtin
empty() -> any;
empty(value: any) -> boolean;
```

```text
empty()：返回空值，推断类型为 Unknown。
empty(value)：沿用当前空值判断规则。
map：按输入顺序逐元素求值，输出长度与输入相同。
join：空元素转为空字符串，保留分隔符。
map / join：列表参数为空时返回空值。
执行错误：列表元素或已执行的 callback 出错时，当前行失败，记录 RowError。

join([1, empty(), 2], ",") → "1,,2"
```

空值表示某个位置没有值，不引入 Null 类型。算术中的空值传播沿用文法规则。
观察依据见 [Notion 实验](../experiments/notion-empty-semantics/README.zh-CN.md)；API 的列表字符串输出模型不作为求值规则。

## Planned 聚合、sort、flat 与 repeat

本节的空值处理只作用于普通 null。已经执行的参数或列表构造产生的错误，先按
[FormulaEngine 行错误规则](formula-engine.zh-CN.md)传播；不能把失败对应的空位置当作可忽略的普通 null。
`Unknown`、嵌套 `Unknown` 和 `Union` 沿用上文的分析规则；通过分析后，运行时实际值仍须满足当前操作的类型要求，
否则产生 `RuntimeError::InvalidValueType`。类型正确的 NaN、Infinity 和有符号零沿用
[Planned Number](formula-language.zh-CN.md#planned-number)，仅在函数明确限制值域时产生值错误。

下列表格中的 `null` 表示普通空值，不是公式字面量。

### Planned 数值聚合

`sum`、`mean`、`median`、`min`、`max` 保留至少一个参数的签名。按参数顺序收集 Number：
Number 参数贡献一个值，List 参数按元素顺序贡献其中的 Number；跳过 null 参数和 null 元素，空列表不贡献值。
不递归展开嵌套列表。非 null 参数须为 Number 或 List，非 null 列表元素须为 Number；先完成这些类型检查，再计算结果。
因此，空值、NaN 或空列表都不能掩盖其他位置的类型错误。参数整体不符合要求时，错误的 `expected` 为
`Union(Number, List(Number))`；列表元素不符合要求时，`expected` 为 `Number`。`actual` 为出错值的实际类型。

| 函数 | 收集到的 Number 非空 | 没有收集到 Number |
| --- | --- | --- |
| `sum` | 从 +0 开始，按收集顺序相加 | +0 |
| `mean` | 上述总和除以 Number 的个数 | 普通 null，无行错误 |
| `median` | 按数值排序后取中间值；偶数个值取中间两数之和再除以 2 | 普通 null，无行错误 |
| `min` / `max` | 使用 Planned Number 中对应的 Math 规则 | 普通 null，无行错误 |

所有五个函数都保留零值；NaN 和正负 Infinity 不是空值。完成类型检查后，只要收集到 NaN，结果就是 NaN，
不产生行错误。`median` 的排序中 -Infinity 最小、+Infinity 最大，-0 排在 +0 前；其余计算遵循 Number 规则。
结果的推断类型始终为 `Number`，包括结果全为 null 的情况。

| 调用或条件 | 结果 |
| --- | --- |
| `sum([2, empty(), 4])` / `sum(2, empty(), 4)` | 6 |
| `mean([2, empty(), 4])` / `median([2, empty(), 4])` | 3 |
| `min([2, empty(), 4])` / `max([2, empty(), 4])` | 2 / 4 |
| `sum([])` / `sum(empty())` | +0 |
| `mean([])` / `median([empty()])` / `min(empty())` / `max([])` | 普通 null |
| `sum([1, 0 / 0])` / `min([1, 0 / 0])` | NaN，无行错误 |
| 声明为 `List(Unknown)` 的输入在某行包含字符串或嵌套列表 | 该行产生 `InvalidValueType` |

### Planned sort

列表参数为 null 时返回普通 null。否则先移除外层列表中的 null 元素，再按当前的非 null 值排序规则排序；
保留零值和所有其他非 null 元素，不修改嵌套列表。空列表或全 null 列表返回 `[]`。
推断结果仍为输入列表类型，删除空位置不根据某一行的值缩窄元素类型。

```text
sort([2, empty(), 1])              → [1, 2]
sort([1, empty(), 0, -1, empty()]) → [-1, 0, 1]
sort([empty(), empty()])           → []
sort([])                          → []
```

### Planned flat

列表参数为 null 时返回普通 null。否则按输入顺序只展开一层：元素是列表时，依次加入该列表的直接元素；
其他元素原样加入，包括 null。保留加入元素的顺序和空位置，不继续展开更深的列表。空列表返回 `[]`。

```text
flat([1, empty(), 2])       → [1, null, 2]
flat([[1, empty()], [2]])   → [1, null, 2]
flat([[], empty(), [2]])    → [null, 2]
flat([[empty()], []])       → [null]
flat([[[]]])               → [[]]
flat([[[1]], [2]])         → [[1], 2]
```

类型推断与同一层展开对应。对输入 `List(T)`，结果为 `List(F(T))`：

```text
F(List(U))          = U                         // 只移除这一层，不再对 U 调用 F
F(Union(T1, …, Tn)) = normalize(Union(F(T1), …, F(Tn)))
F(T)                = T                         // 包括 Unknown
```

这里的 `normalize` 只归一化 Union：展开直接嵌套的 Union、去重并按既有类型顺序排列；
单成员归为该成员，零成员归为 Unknown。它不移除 List 层，也不丢弃 Unknown 成员。
整个输入类型为 Unknown 时，结果为 `List(Unknown)`。输入类型是 Union 时，
对其中的 List 和 Unknown 分支应用上述规则，再归一化合并后的结果元素类型；其他分支不贡献成功结果的类型，
仍须遵循通常的分析与运行时类型检查。这些推断规则不免除运行时对外层 List 的检查。

| 输入类型 | 输出类型 |
| --- | --- |
| `List(Number)` | `List(Number)` |
| `List(List(List(Number)))` | `List(List(Number))` |
| `List(Union(Number, List(String)))` | `List(Union(Number, String))` |
| `List(List(Unknown))` | `List(Unknown)` |
| `List(Union(Unknown, List(Number)))` | `List(Union(Number, Unknown))` |
| `Union(List(Number), List(List(String)))` | `List(Union(Number, String))` |
| `List(Unknown)`，包括 `[]` | `List(Unknown)` |

### Planned repeat

`repeat(text, times)` 的运行时类型要求分别为 String 和 Number。完成参数求值与非 null 值的类型检查后，
只要任一参数为 null，就返回普通 null。两者均非 null 时，`times` 必须有限，然后计算：

```text
repetitions = min(10000, max(0, ceil(times)))
```

结果是 `text` 原样连接 `repetitions` 次。负数与正负零得到空字符串，正小数向上取整；
上限是重复 10,000 次，不是字符串长度上限。空字符串仍须通过有限次数检查。
NaN、+Infinity、-Infinity 次数产生 `RuntimeError::InvalidValue`，`actual` 保留原始 `Value::Number(times)`，
`constraint` 说明次数必须有限；展示文案不是稳定的机器判定字段。
结果的推断类型为 `String`，不因普通 null 或行错误改变。

| 调用或条件 | 结果 |
| --- | --- |
| `repeat("ab", -1.9)` / `repeat("ab", 0)` | `""`，无行错误 |
| `repeat("ab", 0.1)` / `repeat("ab", 1.1)` | `"ab"` / `"abab"` |
| `repeat("ab", 10001)` | 10,000 份 `"ab"` |
| `repeat("", 3)` | `""` |
| `repeat("ab", empty())` | 普通 null |
| `repeat("ab", 0 / 0)` / `repeat("", 1 / 0)` | `InvalidValue`，载荷分别为 NaN / +Infinity |

这些契约参考[列表空值与 repeat 实验](../experiments/notion-list-repeat-semantics/README.zh-CN.md)。
除 `sum` 外的全空聚合返回普通 null、`repeat` 空参数返回普通 null，以及非有限次数返回值错误，是本项目明确选择的解释；
API null 本身不能证明 Notion 内部采用这些分类。本项目也保留 Unknown 可通过分析的规则，
不复制 Notion 对实验中某些 `[]`、`empty()` 表达式的创建阶段拒绝。

公式调用语法见[文法](formula-language.zh-CN.md)。
实现锚点：[类型解析](../../builtin_fn/src/resolution.rs)、
[value kernels](../../evaluator/src/kernels/value.rs)、
[受控 kernels](../../evaluator/src/kernels/controlled.rs)。
本页不承诺上游 Notion 兼容性或内部 Rust `pub` API 稳定性；测试样例不是每个函数的穷尽语义定义。

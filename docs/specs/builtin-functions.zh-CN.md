---
doc_id: specs.builtin-functions
title: "Builtin 函数签名"
language: zh-CN
source_language: zh-CN
counterpart: ./builtin-functions.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-18
---

# Builtin 函数签名

[English](builtin-functions.md) · [规格索引](README.zh-CN.md)

Current：下表列出全部受支持声明；执行入口仍由 [builtins.rs](../../builtin_fn/src/builtins.rs) 驱动。
`builtin` 代码块是签名目录，不是可执行的公式，也尚未接入 Markdown → Rust 生成。

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

flat：递归去掉嵌套 list 层，收集并归一化 union 叶子类型，结果仍为 list。
      没有可用 list 类型时沿用默认返回类型；不能只按表面的 T[] → T[] 推断。
```

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

公式调用语法见[文法](formula-language.zh-CN.md)。
实现锚点：[类型解析](../../builtin_fn/src/resolution.rs)、
[value kernels](../../evaluator/src/kernels/value.rs)、
[受控 kernels](../../evaluator/src/kernels/controlled.rs)。
本页不承诺上游 Notion 兼容性或内部 Rust `pub` API 稳定性；测试样例不是每个函数的穷尽语义定义。

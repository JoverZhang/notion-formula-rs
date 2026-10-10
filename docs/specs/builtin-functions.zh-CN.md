---
doc_id: specs.builtin-functions
title: "Builtin 函数签名"
language: zh-CN
source_language: zh-CN
counterpart: ./builtin-functions.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-01
---

# Builtin 函数签名

[English](builtin-functions.md) · [规格索引](README.zh-CN.md)

声明的执行入口由 [builtins.rs](../../builtin_fn/src/builtins.rs) 驱动。
`builtin` 代码块是签名目录，尚未接入 Markdown → Rust 生成。

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
(current: T, index: number) -> U      // 带隐式绑定的表达式；调用者不写 lambda 语法
repeat(min = n) {...}  // 整组参数重复至少 n 次；不是一个 list 参数
                       // 声明中的尾逗号不意味着公式调用允许尾逗号
```

## 支持的函数

### General

```builtin
if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T;
/// 按顺序求值条件，跳过未选中的值；省略 else 时，未匹配的行返回普通 null。
ifs<T: Variant>(repeat(min = 1) { condition: boolean, value: () -> T }, else?: () -> T) -> T;
/// 表达式只接受 Boolean/null；从左到右短路，空值规则与 && / || 相同。
and(repeat(min = 1) { condition: () -> boolean }) -> boolean;
or(repeat(min = 1) { condition: () -> boolean }) -> boolean;

/// 无参返回 null；有参时判断值是否为空。
/// 返回类型由 resolve_empty 按实参个数确定。
#[resolver(resolve_empty)]
empty(value?: any) -> any;
length(value: string | any[]) -> number;
format(value: any) -> string;
equal(a: any, b: any) -> boolean;
unequal(a: any, b: any) -> boolean;
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;
/// 按顺序绑定名称，各值类型独立；后续绑定可读取或遮蔽已有名称。
/// 返回类型由最终表达式在该词法作用域中的类型决定。
lets(repeat(min = 1) { var: Ident<any>, value: any }, expr: () -> any) -> any;
```

#### empty 类型推断

```rust
fn resolve_empty(input: &builtin_fn::ResolverInput<'_>) -> builtin_fn::Ty {
    use builtin_fn::Ty;

    match input.arguments {
        [] => Ty::Unknown,
        [_] => Ty::Boolean,
        _ => input.default_return_ty.clone(),
    }
}
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

/// 参数求值、非 null 值的类型检查通过后，任一参数为 null 则返回 null。
/// 否则 times 须有限，包括 text 为空字符串的情况；非有限次数报 InvalidValue，actual 为原始 Value::Number(times)。
/// 重复 min(10000, max(0, ceil(times))) 次；10,000 是次数上限，不是输出长度上限。
/// repeat("ab", -1.9) → ""；repeat("ab", 1.1) → "abab"。
repeat(text: string, times: number) -> string;
padStart(text: string | number, length: number, pad: string) -> string;
padEnd(text: string | number, length: number, pad: string) -> string;
/// 所有样式名称照常求值并检查类型，随后原样返回文本（含普通 null）；忽略样式元数据。
style(text: string, repeat(min = 0) { styles: string }) -> string;
unstyle(text: string, repeat(min = 0) { styles: string }) -> string;
/// 至少接受两个列表；标量文本拼接使用 +。
concat<T>(repeat(min = 2) { lists: T[] }) -> T[];

/// null 元素转为空字符串并保留分隔符；列表参数为 null 时返回 null。
/// join([1, empty(), 2], ",") → "1,,2"。
join<T>(list: T[], separator: string) -> string;
split(text: string, separator: string) -> string[];
```

### Number

数值运算遵循 [Number 规则](formula-language.zh-CN.md#current-number)。

```builtin
/// 格式：number/decimal、number_with_commas/commas、percent/%、scientific、humanize、usd/eur/gbp/jpy/cny/krw/inr/cad/aud/chf。
/// format 去除首尾空白并转为小写；未知格式产生 InvalidValue。显式 null precision 返回 null。
/// precision 必须为 0..=1000000 内的有限值，截断后最多保留 100 位。
/// 省略 precision 时，decimal/commas/percent 最多保留 10 位小数，humanize 最多一位（K/M/B/T），
/// scientific 保留 10 位，货币保留两位，jpy/krw 保留零位；显式 precision 保留末尾零。
formatNumber(value: number, format: string, precision?: number) -> string;
add(a: number, b: number) -> number;
subtract(a: number, b: number) -> number;
multiply(a: number, b: number) -> number;
mod(a: number, b: number) -> number;
pow(base: number, exp: number) -> number;
divide(a: number, b: number) -> number;

// 以下五个聚合函数忽略 null 参数和 null 元素，只展开参数列表一层。
// 先检查所有非 null 值，再计算：参数类型不符时 InvalidValueType.expected 为 Union(Number, List(Number))，
// 元素类型不符时 expected 为 Number；actual 为出错值的实际类型。
// NaN 与 Infinity 参与统计；任一 Number 为 NaN 时结果为 NaN，无行错误。

/// 按 Math.min 规则取最小值；没有 Number 时返回普通 null。
min(repeat(min = 1) { values: number | number[] }) -> number;

/// 按 Math.max 规则取最大值；没有 Number 时返回普通 null。
max(repeat(min = 1) { values: number | number[] }) -> number;

/// 从 +0 开始，按参数和元素顺序相加；没有 Number 时返回 +0。
/// sum([2, empty(), 4]) 与 sum(2, empty(), 4) 均为 6；sum([]) 为 +0。
sum(repeat(min = 1) { values: number | number[] }) -> number;

/// 数值升序的中间值；偶数个值取中间两数之和再除以 2。-0 排在 +0 前；没有 Number 时返回普通 null。
median(repeat(min = 1) { values: number | number[] }) -> number;

/// sum 的结果除以 Number 个数；没有 Number 时返回普通 null。
/// mean([2, empty(), 4]) 为 3；mean([]) 为 null。
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
/// 本次求值冻结的时间。
now() -> date;
/// 同一时间快照与时区偏移对应的本地零点。
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
/// 在运行时固定偏移时区中，使用 Moment 日历、时钟、序数、星期与周编号标记格式化 start。
/// [text] 为字面量；W/WW/Wo/GGGG 使用 ISO 周，w/ww/wo/gggg 使用英语区域的周日始周规则。
/// 百分号为字面量；2024-03-05 的 [Week] W 输出 "Week 10"。
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

/// 移除外层 null 元素，再按当前非 null 值的排序规则排序；不修改嵌套列表。
/// 列表参数为 null 时返回 null；空列表或全 null 列表返回 []。不按行值缩窄推断类型。
sort<T>(list: T[]) -> T[];
reverse<T>(list: T[]) -> T[];
unique<T>(list: T[]) -> T[];
includes<T>(list: T[], value: T) -> boolean;

/// 按输入顺序逐元素求值，保持列表长度；列表参数为 null 时返回 null。
/// 所有回调隐式绑定 current 与从零开始的 index；嵌套回调遮蔽两者，退出后恢复。
map<T, U>(list: T[], mapper: (current: T, index: number) -> U) -> U[];
filter<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> T[];
find<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> T;
findIndex<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> number;
some<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> boolean;
every<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> boolean;
count<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> number;

/// 按输入顺序只展开一层，保留 null；列表参数为 null 时返回 null，空列表返回 []。
/// flat([[1, empty()], [2]]) → [1, null, 2]；flat([[[]]]) → [[]]。
/// 返回类型由下方 resolve_flat 定义。
#[resolver(resolve_flat)]
flat<T>(list: T[]) -> T[];
```

#### flat 类型推断

```rust
use builtin_fn::{ArgumentObservation, ResolverInput, Ty, normalize_union};

fn resolve_flat(input: &ResolverInput<'_>) -> Ty {
    fn flatten_element(ty: &Ty) -> Ty {
        match ty {
            // 只移除这一层 List，不继续展开 inner。
            Ty::List(inner) => inner.as_ref().clone(),
            Ty::Union(members) => normalize_union(members.iter().map(flatten_element)),
            other => other.clone(),
        }
    }

    fn collect_elements(ty: &Ty, elements: &mut Vec<Ty>) {
        match ty {
            Ty::List(inner) => elements.push(flatten_element(inner)),
            Ty::Unknown => elements.push(Ty::Unknown),
            Ty::Union(members) => {
                for member in members {
                    collect_elements(member, elements);
                }
            }
            // 类型不符由调用校验处理，不贡献成功结果的类型。
            _ => {}
        }
    }

    let Some(ArgumentObservation::Typed(argument)) = input.arguments.first() else {
        return input.default_return_ty.clone();
    };
    let mut elements = Vec::new();
    collect_elements(argument, &mut elements);
    Ty::List(Box::new(normalize_union(elements)))
}
```

### Special

```builtin
/// 当前行 ID，不是 Formula ID。
id() -> string;
```

```text
People 当前无受支持函数。以下声明不进入可调用/补全集合：
  not                     // 使用 not 前缀运算符，包括 not(true)
  link                    // 尚无 Link 类型
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
  只需有确定的首参数槽位；receiver 可以提供唯一的实参，如 values.sum()。
  receiver.f(args) 等价于 f(receiver, args)，还需类型兼容。
  parser 接受 member-call 语法，不表示任何 builtin 都能 postfix；不支持时不回退成普通调用。

evaluation
  value 参数先求值；受控分支、binder、callback 参数按函数语义选择求值。
  对活动行先完整构造 list，再执行 callback；构造错误不能被 callback 短路隐藏。
  callback 只处理活动的行/元素组合；find/findIndex/some/every 可提前结束，不推广到其他函数。
  null 规则见各函数的声明注释。
  无效 regex、日期或值域等运行时问题是行错误；未执行的分支/元素不产生错误。
```

列表中的普通 null 像 [JavaScript 数组](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.flat)一样保留位置；各函数的差异见声明注释，观察依据见[空值实验](../experiments/notion-empty-semantics/README.zh-CN.md)和[列表与 repeat 实验](../experiments/notion-list-repeat-semantics/README.zh-CN.md)。
执行失败按 [FormulaEngine](formula-engine.zh-CN.md) 的行错误规则传播，不作为普通 null 忽略。

公式调用语法见[文法](formula-language.zh-CN.md)。
实现锚点：[类型解析](../../builtin_fn/src/resolution.rs)、
[value kernels](../../evaluator/src/kernels/value.rs)、
[受控 kernels](../../evaluator/src/kernels/controlled.rs)。
本页不承诺上游 Notion 兼容性或内部 Rust `pub` API 稳定性；测试样例不是每个函数的穷尽语义定义。

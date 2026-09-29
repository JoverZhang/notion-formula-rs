---
doc_id: specs.builtin-functions
title: "Builtin function signatures"
language: en
source_language: zh-CN
counterpart: ./builtin-functions.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
translation_model: gpt-6-luna
translation_review_model: gpt-6-astra
last_verified: 2026-09-29
---

# Builtin function signatures

[简体中文](builtin-functions.zh-CN.md) · [Specification index](README.md)

Current: the catalog below contains every supported declaration; [builtins.rs](../../builtin_fn/src/builtins.rs)
still drives executable registration. The `builtin` blocks are signatures, not formula expressions,
and are not yet inputs to Markdown-to-Rust generation.
The Planned sections below define upcoming FormulaEngine behavior, not support in the current evaluator.

## Signature notation

This EBNF defines the notation used on this page, not the complete internal macro DSL.

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
x?: T                  // An omittable argument, not a claim that all T values are non-null
T[] / A | B            // List / union; T and U bind within one call
Ident<T>               // Binding identifier, not an ordinary string argument
() -> T                // Deferred expression
(current: T) -> U      // Expression with an implicit binding; callers do not write lambda syntax
repeat(min = n) {...}  // Repeat the whole argument group at least n times; not one list argument
                       // Trailing commas in declarations do not permit them in formula calls

Current flat: recursively strips nested list layers, collects and normalizes union leaf types, and returns a list.
      Without usable list type information it retains the default return type; surface T[] → T[] alone is insufficient.
```

The replacement one-level behavior and type inference are defined under [Planned flat](#planned-flat).

## Supported functions

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
People currently has no supported functions. These declarations are excluded from callable/completion sets:
  and / or / not          // Expressed by && / || / not operators
  lets                    // No heterogeneous sequential binder model
  link / style / unstyle  // No Link / StyledText types yet
  dateRange / dateStart / dateEnd // No DateRange type yet
  name / email            // No person-name/email runtime inputs yet
Unsupported declarations are treated like unknown functions, without a separate unsupported error class.
Category order is General, Text, Number, Date, People, List, Special; declaration order is preserved within each.
```

## Calls and execution

```text
shape → type → execution
  Supported evaluation requires successful syntax and semantic validation; unknown alone is not validation failure.
  Validate fixed/optional/repeat/trailing argument shape before types; known type mismatches produce diagnostics.
  Generics, unions, and implicit function parameters participate in call-wide type binding.
  Unknown, including nested unknown, is indeterminate rather than an immediate mismatch; analysis success does not ensure row success.

postfix
  Requires a deterministic first parameter slot and another argument position after consuming the receiver.
  receiver.f(args) is equivalent to f(receiver, args), subject to type compatibility.
  Parsing a member call does not make every builtin postfix-capable; unsupported member calls do not fall back to ordinary calls.

evaluation
  Value arguments are evaluated first; controlled branches, binders, and callbacks evaluate as their function semantics require.
  Lists are fully constructed for active rows before callbacks; callback short-circuiting cannot hide construction errors.
  Callbacks visit only active row/element pairs; find/findIndex/some/every may stop early, not a promise for other functions.
  Null rules belong to function families; signatures do not imply one universal null-propagation rule.
  Invalid regexes, dates, or value domains cause row errors; unexecuted branches/elements contribute no errors.

runtime
  now()   → frozen evaluation time
  today() → local midnight for that same snapshot and timezone offset
  id()    → current row ID as text, not formula ID
```

## Planned Empty Values and Lists

```builtin
empty() -> any;
empty(value: any) -> boolean;
```

```text
empty() returns a null value with inferred type Unknown.
empty(value) follows the current null-checking rules.
map evaluates each element in input order and returns a list with the same length as the input.
join converts null elements to empty strings and preserves their separators.
If the list argument to map or join is null, the function returns null.
If a list element or an executed callback errors, the current row fails and records a RowError.

join([1, empty(), 2], ",") → "1,,2"
```

Null means that a position has no value; it does not introduce a Null type. Null propagation in arithmetic follows the grammar rules.
See the [Notion experiment](../experiments/notion-empty-semantics/README.md) for the observation basis; the API's list-to-string output model does not define evaluation rules.

## Planned Aggregates, Sort, Flat, and Repeat

The null handling in this section applies only to ordinary nulls. Errors from evaluated arguments or list construction
propagate first under the [FormulaEngine row-error rules](formula-engine.md); a failure's null output position cannot
be treated as an ordinary null to skip. `Unknown`, nested `Unknown`, and `Union` follow the analysis rules above;
after analysis succeeds, runtime values must still meet the operation's type requirements or produce
`RuntimeError::InvalidValueType`. Type-correct NaN, Infinity, and signed zero follow
[Planned Number](formula-language.md#planned-number); they cause value errors only where a function explicitly restricts its value domain.

In the examples below, `null` denotes an ordinary missing value, not a formula literal.

### Planned Numeric Aggregates

`sum`, `mean`, `median`, `min`, and `max` keep their signatures requiring at least one argument. Collect Numbers in
argument order: a Number argument contributes one value, and a List argument contributes its Numbers in element order.
Skip null arguments and null elements; empty lists contribute no values. Do not recursively flatten nested lists.
Each non-null argument must be a Number or List, and each non-null list element must be a Number; finish these type
checks before computing the result. Thus nulls, NaN, and empty lists cannot hide a type error elsewhere.
For an incompatible whole argument, the error's `expected` is `Union(Number, List(Number))`; for an incompatible
list element, `expected` is `Number`. The `actual` field is the offending value's runtime type.

| Function | At least one Number collected | No Numbers collected |
| --- | --- | --- |
| `sum` | Add in collection order, starting at +0 | +0 |
| `mean` | Divide that sum by the number of Numbers | Ordinary null, with no row error |
| `median` | Sort numerically and take the middle value; for an even count, add the two middle values and divide by 2 | Ordinary null, with no row error |
| `min` / `max` | Use the corresponding Math rules in Planned Number | Ordinary null, with no row error |

All five functions retain zero values; NaN and positive or negative Infinity are not nulls. After type checking,
any collected NaN makes the result NaN, with no row error. For `median` ordering, -Infinity is smallest, +Infinity
is largest, and -0 precedes +0; other calculations follow the Number rules.
The inferred result type is always `Number`, including when every result is null.

| Call or condition | Result |
| --- | --- |
| `sum([2, empty(), 4])` / `sum(2, empty(), 4)` | 6 |
| `mean([2, empty(), 4])` / `median([2, empty(), 4])` | 3 |
| `min([2, empty(), 4])` / `max([2, empty(), 4])` | 2 / 4 |
| `sum([])` / `sum(empty())` | +0 |
| `mean([])` / `median([empty()])` / `min(empty())` / `max([])` | Ordinary null |
| `sum([1, 0 / 0])` / `min([1, 0 / 0])` | NaN, with no row error |
| An input declared as `List(Unknown)` contains a string or nested list in a row | `InvalidValueType` for that row |

### Planned sort

A null list argument returns ordinary null. Otherwise, remove null elements from the outer list, then apply the
current ordering of non-null values. Retain zeros and all other non-null elements; do not modify nested lists.
An empty or all-null list returns `[]`. The inferred result remains the input list type; removing null positions
does not narrow the element type based on a particular row's values.

```text
sort([2, empty(), 1])              → [1, 2]
sort([1, empty(), 0, -1, empty()]) → [-1, 0, 1]
sort([empty(), empty()])           → []
sort([])                          → []
```

### Planned flat

A null list argument returns ordinary null. Otherwise, flatten exactly one level in input order: a list element
contributes its direct elements, and any other element is copied as-is, including null. Preserve the contributed
elements' order and null positions without expanding deeper lists. An empty list returns `[]`.

```text
flat([1, empty(), 2])       → [1, null, 2]
flat([[1, empty()], [2]])   → [1, null, 2]
flat([[], empty(), [2]])    → [null, 2]
flat([[empty()], []])       → [null]
flat([[[]]])               → [[]]
flat([[[1]], [2]])         → [[1], 2]
```

Type inference follows the same one-level expansion. For input `List(T)`, the result is `List(F(T))`:

```text
F(List(U))          = U                         // Remove only this layer; do not apply F to U again
F(Union(T1, …, Tn)) = normalize(Union(F(T1), …, F(Tn)))
F(T)                = T                         // Including Unknown
```

Here `normalize` only normalizes Unions: flatten directly nested Unions, deduplicate, and use the existing type order;
collapse a single member to that member and zero members to Unknown. It neither removes List layers nor drops
Unknown members. An entirely Unknown input yields `List(Unknown)`. For a Union input, apply these rules to its List
and Unknown alternatives, then normalize their combined result element types; other alternatives contribute no
successful result type and remain subject to normal analysis and runtime type checks. These inference rules do not
waive the runtime check that the outer value is a List.

| Input type | Output type |
| --- | --- |
| `List(Number)` | `List(Number)` |
| `List(List(List(Number)))` | `List(List(Number))` |
| `List(Union(Number, List(String)))` | `List(Union(Number, String))` |
| `List(List(Unknown))` | `List(Unknown)` |
| `List(Union(Unknown, List(Number)))` | `List(Union(Number, Unknown))` |
| `Union(List(Number), List(List(String)))` | `List(Union(Number, String))` |
| `List(Unknown)`, including `[]` | `List(Unknown)` |

### Planned repeat

`repeat(text, times)` requires runtime types String and Number, respectively. After evaluating the arguments and
checking the types of non-null values, return ordinary null if either argument is null. Otherwise, `times` must
be finite, and the repetition count is:

```text
repetitions = min(10000, max(0, ceil(times)))
```

The result concatenates `repetitions` unchanged copies of `text`. Negative counts and either signed zero produce
an empty string; positive fractions round up. The limit is 10,000 repetitions, not an output string length limit.
Empty text still requires a finite count. NaN, +Infinity, or -Infinity counts produce `RuntimeError::InvalidValue`,
with `actual` retaining the original `Value::Number(times)` and `constraint` explaining that the count must be finite;
the display wording is not a stable machine-readable discriminator.
The inferred result type is `String`, unchanged by ordinary nulls or row errors.

| Call or condition | Result |
| --- | --- |
| `repeat("ab", -1.9)` / `repeat("ab", 0)` | `""`, with no row error |
| `repeat("ab", 0.1)` / `repeat("ab", 1.1)` | `"ab"` / `"abab"` |
| `repeat("ab", 10001)` | 10,000 copies of `"ab"` |
| `repeat("", 3)` | `""` |
| `repeat("ab", empty())` | Ordinary null |
| `repeat("ab", 0 / 0)` / `repeat("", 1 / 0)` | `InvalidValue`, with NaN / +Infinity payloads, respectively |

These contracts draw on the [list-null and repeat experiments](../experiments/notion-list-repeat-semantics/README.md).
Ordinary null for all-empty aggregates other than `sum` and for null `repeat` arguments, and value errors for nonfinite counts, are explicit
project choices; API null alone cannot establish those internal classifications in Notion. The project also retains
its analysis rules allowing Unknown, rather than adopting Notion's creation-time rejection of some tested `[]`
and `empty()` expressions.

See [grammar](formula-language.md) for formula-call syntax.
Implementation anchors: [type resolution](../../builtin_fn/src/resolution.rs),
[value kernels](../../evaluator/src/kernels/value.rs),
[controlled kernels](../../evaluator/src/kernels/controlled.rs).
This page promises neither upstream Notion compatibility nor internal Rust `pub` API stability;
test fixtures are not exhaustive per-function semantics.

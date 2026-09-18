---
doc_id: specs.builtin-functions
title: "Builtin function signatures"
language: en
source_language: zh-CN
counterpart: ./builtin-functions.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-18
---

# Builtin function signatures

[简体中文](builtin-functions.zh-CN.md) · [Specification index](README.md)

Current: the catalog below contains every supported declaration; [builtins.rs](../../builtin_fn/src/builtins.rs)
still drives executable registration. The `builtin` blocks are signatures, not formula expressions,
and are not yet inputs to Markdown-to-Rust generation.

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

flat: recursively strips nested list layers, collects and normalizes union leaf types, and returns a list.
      Without usable list type information it retains the default return type; surface T[] → T[] alone is insufficient.
```

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

See [grammar](formula-language.md) for formula-call syntax.
Implementation anchors: [type resolution](../../builtin_fn/src/resolution.rs),
[value kernels](../../evaluator/src/kernels/value.rs),
[controlled kernels](../../evaluator/src/kernels/controlled.rs).
This page promises neither upstream Notion compatibility nor internal Rust `pub` API stability;
test fixtures are not exhaustive per-function semantics.

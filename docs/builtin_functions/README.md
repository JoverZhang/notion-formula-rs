# Built-in functions

## Overview

This document lists the builtin functions known to the analyzer (**type signatures only**). It is intended for type checking, completion, and signature help.

### Current limitations (important)

- **`DateRange` is not supported**
  - A distinct `DateRange` type exists in the signatures below, but the analyzer does **not** currently support it (type checking, inference, and related APIs are incomplete).
- **Rich text types are not supported**
  - `Link` and `StyledText` exist in the signatures below, but these types are **not** modeled yet.
- **Lambda/function expression typing is not supported**
  - Functions requiring lambda predicates/mappers (`find`/`filter`/`map`/etc.) cannot be fully supported yet.
- **Binder/literal variable semantics are not supported**
  - `let` / `lets` style variable-binding semantics are not modeled yet.
- **Some APIs require “shape-level typing” that we do not model**
  - Example: `flat(list)` conceptually depends on the flattening depth (or on the element nesting level) to compute the precise return type.
  - We currently do **not** model this “depth/level” relationship. The signature is therefore intentionally approximate.

### Status markers used below

This document is **spec-first**. We keep target signatures in stable order and annotate known implementation gaps.

- `TODO-op`: intentionally represented by operators/keywords instead of builtin call signatures.
- `TODO-missing`: signature is in target spec, but not modeled in builtins yet.
- `TODO-type`: blocked by missing type-model support (`DateRange`, `Link`, `StyledText`, etc.).
- `TODO-lambda`: blocked by missing lambda/function expression typing.
- `TODO-binder`: blocked by missing variable-binder semantics.
- `TODO-flat`: blocked by missing depth-sensitive `flat` typing.
- Unmarked signatures: no known model-level blocker.

### Notation and philosophy

- We intentionally use `any` / `any[]` in places where generics would add noise but not meaningful constraints.
  - Example: `length(value: string | any[]) -> number` instead of `length<T>(value: string | T[])`.
- We still use generics where they are semantically meaningful:
  - Branch-like functions (`if`/`ifs`) where the result type depends on branch types.
  - List transformers (`map`) where output element type depends on a lambda result.

---

## Syntax

### Signature shape

- General form: `name<...>(args...) -> return`
- Types:
  - primitives: `number`, `string`, `boolean`, `date`, `null`, `any`
  - lists: `T[]`, `any[]`
  - unions: `A | B`

### Optional arguments

- `arg?: type` means the argument can be omitted.

### Generic binding modes

`<T: Plain>` vs `<T: Variant>` controls how the analyzer binds `T` during inference:

- **Plain**
  - ignores `unknown`/`any` arguments when binding
  - tolerates conflicts permissively
- **Variant** (branch-like)
  - unions branch types
  - if any binding is `unknown`/`any`, the result becomes `unknown`/`any`

> Keep this section synchronized with the analyzer implementation.

### Variadic vs repeat groups

We use two notations:

- **Variadic arguments** (simple “rest args”):
  - `fn(a: X, ...rest: X[]) -> Y`
- **Repeat groups** (tuples repeated as a unit):
  - Example: `ifs(condition1, value1, condition2, value2, ..., else)`
  - Meaning: the `(conditionN, valueN)` group repeats **one or more times**, followed by a final `else`.

### Member-call sugar (postfix calls)

For some builtins, `receiver.fn(a, b)` is analyzed like `fn(receiver, a, b)`.

---

## General (12)

```rust
if<T: Variant>(condition: boolean, then: T, else: T) -> T

// Repeat group: (conditionN, valueN) repeated 1+ times, followed by else.
ifs<T: Variant>(condition1: boolean, value1: T, ..., else: T) -> T

// TODO-op: currently represented by operators (`&&` / `||`) instead of builtin call signatures.
and(condition1: boolean, ...) -> boolean
or(condition1: boolean, ...) -> boolean

// TODO-op: currently represented by prefix keyword operator `not`.
not(condition: boolean) -> boolean

empty(value?: any) -> boolean
length(value: string | any[]) -> number
format(value: any) -> string

equal(a: any, b: any) -> boolean
unequal(a: any, b: any) -> boolean

// TODO-binder: Ident/binder semantics are not modeled yet.
let(var: Ident<any>, value: any, expr: (var: any) -> any) -> any

// Repeat group: (varN, valueN) repeated 1+ times, then expr.
// TODO-binder: precise binder typing is not modeled yet.
lets(var1: Ident<any>, value1: any, var2: Ident<any>, value2: any, ..., expr: (var1: any, var2: any, ...) -> any) -> any
```

---

## Text (18)

```rust
substring(text: string, start: number, end?: number) -> string
contains(text: string, search: string) -> boolean
test(text: string, regex: string) -> boolean
match(text: string, regex: string) -> string[]
replace(text: string, regex: string, replacement: string) -> string
replaceAll(text: string, regex: string, replacement: string) -> string
lower(text: string) -> string
upper(text: string) -> string
trim(text: string) -> string
repeat(text: string, times: number) -> string

// TODO-missing
padStart(text: string | number, length: number, pad: string) -> string
padEnd(text: string | number, length: number, pad: string) -> string

// TODO-type: `Link` type is not modeled yet.
link(label: string, url: string) -> Link

// TODO-type: `StyledText` type is not modeled yet.
style(text: string, styles1: string, styles2: string, ...) -> StyledText

// TODO-type: `StyledText` is not modeled yet.
unstyle(text: string | StyledText, styles?: string) -> string

concat(lists1: any[], lists2: any[], ...) -> any[]
join(list: any[], separator: string) -> string
split(text: string, separator: string) -> string[]
```

---

## Number (26)

```rust
// TODO-missing
formatNumber(value: number, format: string, precision: number) -> string

add(a: number, b: number) -> number
subtract(a: number, b: number) -> number
multiply(a: number, b: number) -> number
mod(a: number, b: number) -> number
pow(base: number, exp: number) -> number
divide(a: number, b: number) -> number

min(values1: number | number[], values2: number | number[], ...) -> number
max(values1: number | number[], values2: number | number[], ...) -> number
sum(values1: number | number[], values2: number | number[], ...) -> number
median(values1: number | number[], values2: number | number[], ...) -> number
mean(values1: number | number[], values2: number | number[], ...) -> number

abs(value: number) -> number
round(value: number, places?: number) -> number
ceil(value: number) -> number
floor(value: number) -> number
sqrt(value: number) -> number
cbrt(value: number) -> number
exp(value: number) -> number
ln(value: number) -> number
log10(value: number) -> number
log2(value: number) -> number
sign(value: number) -> number

pi() -> number
e() -> number

toNumber(value: any) -> number
```

---

## Date (19)

```rust
now() -> date
today() -> date

minute(date: date) -> number
hour(date: date) -> number
day(date: date) -> number
date(date: date) -> number
week(date: date) -> number
month(date: date) -> number
year(date: date) -> number

dateAdd(date: date, amount: number, unit: string) -> date
dateSubtract(date: date, amount: number, unit: string) -> date
dateBetween(a: date, b: date, unit: string) -> number

// TODO-type: `DateRange` type is not supported yet.
dateRange(start: date, end: date) -> DateRange
dateStart(range: DateRange) -> date
dateEnd(range: DateRange) -> date

timestamp(date: date) -> number
fromTimestamp(timestamp: number) -> date
formatDate(date: date, format: string) -> string
parseDate(text: string) -> date
```

---

## People (2)

```rust
name(person: any) -> string
email(person: any) -> string
```

---

## List (17)

```rust
at(list: any[], index: number) -> any
first(list: any[]) -> any
last(list: any[]) -> any
slice(list: any[], start: number, end?: number) -> any[]

// TODO-missing
splice(list: any[], startIndex: number, deleteCount: number, ...items: any[]) -> any[]

sort(list: any[]) -> any[]
reverse(list: any[]) -> any[]
unique(list: any[]) -> any[]
includes(list: any[], value: any) -> boolean

// TODO-lambda: requires lambda/function expression typing.
find(list: any[], expr: (current: any) -> boolean) -> any
findIndex(list: any[], expr: (current: any) -> boolean) -> number
filter(list: any[], expr: (current: any) -> boolean) -> any[]
some(list: any[], expr: (current: any) -> boolean) -> boolean
every(list: any[], expr: (current: any) -> boolean) -> boolean

// TODO-lambda: requires lambda/function expression typing.
map(list: any[], expr: (current: any) -> any) -> any[]

// `flat(list)` is the only supported call form.
// TODO-flat: precise nesting-depth -> return-depth typing is not modeled.
flat(list: any[]) -> any[]

// TODO-lambda: requires lambda/function expression typing.
count(list: any[], expr: (current: any) -> boolean) -> number
```

---

## Special (1)

```rust
id(page?: any) -> string
```

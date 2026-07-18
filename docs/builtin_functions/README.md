# Built-in functions

## Overview

This document is the rendered builtin catalog used by the analyzer, IDE, and documentation,
and is the input planned for evaluator contract generation. Each category is declared once
with the `builtin_functions!` DSL; the marked region below is regenerated from those
declarations.

The catalog contains both supported and unsupported declarations. Unsupported declarations
remain visible and documented here, but are excluded from `builtins_functions()` and from
the supported set that evaluator generation will consume.

### Current unsupported declarations

- `DateRange`, `Link`, and `StyledText` are not represented by the semantic type model.
- `lets` requires a heterogeneous sequential binder-pack model. The single-binder `let`
  form and lambda-based list functions are supported.
- `name` and `email` require person data that is not present in the runtime input contract.
- `and`, `or`, and `not` are represented by language operators instead of executable
  builtin calls.

### Status markers used below

`// Unsupported: ...` is generated from the declaration's required doc comment. Unmarked
signatures are supported semantic declarations and form the input set for the future
evaluator contract generator.

### Notation and philosophy

- We intentionally use `any` / `any[]` in places where generics would add noise but not meaningful constraints.
  - Example: `length(value: string | any[]) -> number` instead of `length<T>(value: string | T[])`.
- We still use generics where they are semantically meaningful:
  - Branch-like functions (`if`/`ifs`) where the result type depends on branch types.
  - List transformers (`map`) where output element type depends on a lambda result.
- A function-level resolver may only refine the return type after ordinary shape and
  generic resolution. `flat` uses this seam to derive its nested-list result precisely.

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

### Repeat groups

Declarations use explicit `repeat(min = N) { ... }` blocks with logical base names such as
`condition`, `value`, or `lists`. Numbering is presentation only: the renderer produces
`condition1`, `condition2`, and `...`. A repeat block may appear by itself, after a fixed
head, before a fixed tail, or between both.

### Member-call sugar (postfix calls)

For some builtins, `receiver.fn(a, b)` is analyzed like `fn(receiver, a, b)`.

### Regeneration

The text outside the markers is maintained by hand. The catalog region is deterministic:

```bash
cargo run -p builtin_fn --bin builtin_catalog -- --check
cargo run -p builtin_fn --bin builtin_catalog -- --write
```

---

<!-- BEGIN GENERATED BUILTIN CATALOG -->

## General (12)

```rust
if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T
ifs<T: Variant>(condition1: boolean, value1: () -> T, condition2: boolean, value2: () -> T, ..., else: () -> T) -> T
// Unsupported: Currently expressed by the `&&` operator rather than a builtin call.
and(condition1: boolean, condition2: boolean, ...) -> boolean
// Unsupported: Currently expressed by the `||` operator rather than a builtin call.
or(condition1: boolean, condition2: boolean, ...) -> boolean
// Unsupported: Currently expressed by the `not` prefix operator rather than a builtin call.
not(condition: boolean) -> boolean
empty(value?: any) -> boolean
length(value: string | any[]) -> number
format(value: any) -> string
equal(a: any, b: any) -> boolean
unequal(a: any, b: any) -> boolean
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U
// Unsupported: Precise sequential binder typing requires a heterogeneous binder-pack model.
lets(var1: Ident<any>, value1: any, var2: Ident<any>, value2: any, ..., expr: () -> any) -> any
```

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
padStart(text: string | number, length: number, pad: string) -> string
padEnd(text: string | number, length: number, pad: string) -> string
// Unsupported: The semantic type model does not yet represent `Link`.
link(label: string, url: string) -> Link
// Unsupported: The semantic type model does not yet represent `StyledText`.
style(text: string, styles1: string, styles2: string, ...) -> StyledText
// Unsupported: The semantic type model does not yet represent `StyledText`.
unstyle(text: string | StyledText, styles?: string) -> string
concat<T>(lists1: T[], lists2: T[], ...) -> T[]
join<T>(list: T[], separator: string) -> string
split(text: string, separator: string) -> string[]
```

## Number (26)

```rust
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
// Unsupported: The semantic type model does not yet represent `DateRange`.
dateRange(start: date, end: date) -> DateRange
// Unsupported: The semantic type model does not yet represent `DateRange`.
dateStart(range: DateRange) -> date
// Unsupported: The semantic type model does not yet represent `DateRange`.
dateEnd(range: DateRange) -> date
timestamp(date: date) -> number
fromTimestamp(timestamp: number) -> date
formatDate(date: date, format: string) -> string
parseDate(text: string) -> date
```

## People (2)

```rust
// Unsupported: Runtime inputs do not currently provide a person's display name.
name(person: any) -> string
// Unsupported: Runtime inputs do not currently provide a person's email address.
email(person: any) -> string
```

## List (17)

```rust
at<T>(list: T[], index: number) -> T
first<T>(list: T[]) -> T
last<T>(list: T[]) -> T
slice<T>(list: T[], start: number, end?: number) -> T[]
splice<T>(list: T[], startIndex: number, deleteCount: number, items1: T, items2: T, ...) -> T[]
sort<T>(list: T[]) -> T[]
reverse<T>(list: T[]) -> T[]
unique<T>(list: T[]) -> T[]
includes<T>(list: T[], value: T) -> boolean
map<T, U>(list: T[], mapper: (current: T) -> U) -> U[]
filter<T>(list: T[], predicate: (current: T) -> boolean) -> T[]
find<T>(list: T[], predicate: (current: T) -> boolean) -> T
findIndex<T>(list: T[], predicate: (current: T) -> boolean) -> number
some<T>(list: T[], predicate: (current: T) -> boolean) -> boolean
every<T>(list: T[], predicate: (current: T) -> boolean) -> boolean
count<T>(list: T[], predicate: (current: T) -> boolean) -> number
flat<T>(list: T[]) -> T[]
```

## Special (1)

```rust
id() -> string
```
<!-- END GENERATED BUILTIN CATALOG -->

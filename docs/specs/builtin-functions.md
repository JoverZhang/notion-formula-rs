---
doc_id: specs.builtin-functions
title: "Builtin function signatures"
language: en
source_language: zh-CN
counterpart: ./builtin-functions.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-01
---

# Builtin function signatures

[简体中文](builtin-functions.zh-CN.md) · [Specification index](README.md)

Declarations execute through [builtins.rs](../../builtin_fn/src/builtins.rs).
The `builtin` blocks are a signature catalog and are not yet connected to Markdown → Rust generation.

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
(current: T, index: number) -> U      // Expression with an implicit binding; callers do not write lambda syntax
repeat(min = n) {...}  // Repeat the whole argument group at least n times; not one list argument
                       // Trailing commas in declarations do not permit them in formula calls
```

## Supported functions

### General

```builtin
if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T;
/// Evaluate conditions in order; skip unselected values. Without else, an unmatched row returns ordinary null.
ifs<T: Variant>(repeat(min = 1) { condition: boolean, value: () -> T }, else?: () -> T) -> T;
/// Require Boolean/null expressions and short-circuit left to right using the && / || null rules.
and(repeat(min = 1) { condition: () -> boolean }) -> boolean;
or(repeat(min = 1) { condition: () -> boolean }) -> boolean;

/// Returns null with no arguments; otherwise checks whether the value is empty.
/// Return type is inferred by resolve_empty from argument count.
#[resolver(resolve_empty)]
empty(value?: any) -> any;
length(value: string | any[]) -> number;
format(value: any) -> string;
equal(a: any, b: any) -> boolean;
unequal(a: any, b: any) -> boolean;
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;
/// Bind names sequentially with independent value types; later bindings see and may shadow earlier names.
/// Infer the result type from the final expression in that lexical scope.
lets(repeat(min = 1) { var: Ident<any>, value: any }, expr: () -> any) -> any;
```

#### empty type inference

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

/// After argument evaluation and non-null value type checks, return null if either argument is null.
/// Otherwise times must be finite, even for empty text; a nonfinite count produces InvalidValue with the original Value::Number(times) as actual.
/// Repeat min(10000, max(0, ceil(times))) times; 10,000 caps the repetition count, not output length.
/// repeat("ab", -1.9) → ""; repeat("ab", 1.1) → "abab".
repeat(text: string, times: number) -> string;
padStart(text: string | number, length: number, pad: string) -> string;
padEnd(text: string | number, length: number, pad: string) -> string;
concat<T>(repeat(min = 2) { lists: T[] }) -> T[];

/// Convert null elements to empty strings and retain separators; a null list argument returns null.
/// join([1, empty(), 2], ",") → "1,,2".
join<T>(list: T[], separator: string) -> string;
split(text: string, separator: string) -> string[];
```

### Number

Numeric operations follow the [Number rules](formula-language.md#current-number).

```builtin
formatNumber(value: number, format: string, precision: number) -> string;
add(a: number, b: number) -> number;
subtract(a: number, b: number) -> number;
multiply(a: number, b: number) -> number;
mod(a: number, b: number) -> number;
pow(base: number, exp: number) -> number;
divide(a: number, b: number) -> number;

// The following five aggregates skip null arguments and null elements, expanding argument lists only one level.
// Check all non-null values before computing: for an argument type mismatch, InvalidValueType.expected is Union(Number, List(Number));
// for an element mismatch, expected is Number; actual is the offending value's actual type.
// NaN and Infinity participate in the calculation; any NaN Number makes the result NaN, without a row error.

/// Take the minimum according to Math.min; without any Numbers, return ordinary null.
min(repeat(min = 1) { values: number | number[] }) -> number;

/// Take the maximum according to Math.max; without any Numbers, return ordinary null.
max(repeat(min = 1) { values: number | number[] }) -> number;

/// Add in argument and element order, starting at +0; without any Numbers, return +0.
/// sum([2, empty(), 4]) and sum(2, empty(), 4) both return 6; sum([]) is +0.
sum(repeat(min = 1) { values: number | number[] }) -> number;

/// Take the middle value in numeric ascending order; for an even count, add the two middle values and divide by 2. -0 precedes +0; without any Numbers, return ordinary null.
median(repeat(min = 1) { values: number | number[] }) -> number;

/// Divide the sum result by the number of Numbers; without any Numbers, return ordinary null.
/// mean([2, empty(), 4]) is 3; mean([]) is null.
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
/// Frozen time for this evaluation.
now() -> date;
/// Local midnight for the same time snapshot and timezone offset.
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

/// Remove outer null elements, then apply the current ordering of non-null values; do not modify nested lists.
/// A null list argument returns null; an empty or all-null list returns []. Do not narrow the inferred type by row values.
sort<T>(list: T[]) -> T[];
reverse<T>(list: T[]) -> T[];
unique<T>(list: T[]) -> T[];
includes<T>(list: T[], value: T) -> boolean;

/// Evaluate each element in input order, preserving list length; a null list argument returns null.
/// Every callback binds current and zero-based index; nested callbacks shadow and then restore both names.
map<T, U>(list: T[], mapper: (current: T, index: number) -> U) -> U[];
filter<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> T[];
find<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> T;
findIndex<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> number;
some<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> boolean;
every<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> boolean;
count<T>(list: T[], predicate: (current: T, index: number) -> boolean) -> number;

/// Flatten only one level in input order, retaining nulls; a null list argument returns null and an empty list returns [].
/// flat([[1, empty()], [2]]) → [1, null, 2]; flat([[[]]]) → [[]].
/// Return type is defined by resolve_flat below.
#[resolver(resolve_flat)]
flat<T>(list: T[]) -> T[];
```

#### flat type inference

```rust
use builtin_fn::{ArgumentObservation, ResolverInput, Ty, normalize_union};

fn resolve_flat(input: &ResolverInput<'_>) -> Ty {
    fn flatten_element(ty: &Ty) -> Ty {
        match ty {
            // Remove only this List layer without flattening inner.
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
            // Call validation handles type mismatches; they contribute no successful result type.
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
/// ID of the current row, not Formula ID.
id() -> string;
```

```text
People currently has no supported functions. These declarations are excluded from callable/completion sets:
  not                     // Expressed by the not prefix operator, including not(true)
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
  See each function's declaration comments for null rules.
  Invalid regexes, dates, or value domains cause row errors; unexecuted branches/elements contribute no errors.
```

Ordinary nulls retain their positions in lists, as in [JavaScript arrays](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-array.prototype.flat). See declaration comments for function differences, and the [empty-value experiment](../experiments/notion-empty-semantics/README.md) and [list and repeat experiment](../experiments/notion-list-repeat-semantics/README.md) for observations.
Execution failures propagate under the [FormulaEngine](formula-engine.md) row-error rules and are not ignored as ordinary nulls.

See [grammar](formula-language.md) for formula-call syntax.
Implementation anchors: [type resolution](../../builtin_fn/src/resolution.rs),
[value kernels](../../evaluator/src/kernels/value.rs),
[controlled kernels](../../evaluator/src/kernels/controlled.rs).
This page promises neither upstream Notion compatibility nor internal Rust `pub` API stability;
test fixtures are not exhaustive per-function semantics.

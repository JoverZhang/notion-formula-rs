---
doc_id: specs.formula-language
title: "What source and evaluation behavior does the formula language support?"
language: en
source_language: en
counterpart: ./formula-language.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Formula language

[简体中文](formula-language.zh-CN.md)

This Current specification defines the formula source and non-builtin evaluation behavior that formula authors and integrations may rely on. It covers literals, expression forms, operators, nulls, lazy branches, and the boundary between best-effort analysis and supported evaluation. Builtin-specific calls, editor presentation, and transport DTOs are outside its scope.

Notion-style syntax is a starting vocabulary, not a compatibility promise. Only the forms and behavior documented here belong to the supported language.

## Write formulas from these source forms

- Boolean literals are `true` and `false`.
- Number literals contain decimal digits, may have a fractional part whose dot is followed by at least one digit, and may have an `e` or `E` exponent with an optional sign and at least one exponent digit. Examples include `12`, `3.14`, and `2.5e-3`.
- String literals use double quotes. The supported escapes are `\n`, `\t`, `\"`, and `\\`.
- List literals contain comma-separated expressions in brackets, such as `[1, "x"]`. A trailing comma is not supported.
- Identifiers start with `_` or a Unicode alphabetic character, followed by `_` or Unicode alphanumeric characters. `true`, `false`, and `not` are reserved lowercase words.
- Parentheses group an expression. Calls use `name(arg1, ...)`; supported functions may also allow `receiver.name(arg1, ...)`. A member name without the following call parentheses is not supported. Call argument lists do not allow a trailing comma.
- `//` starts a line comment, `/* ... */` encloses a block comment, and newlines may appear between tokens.

There is no source literal for null or date values. Nulls can enter through input data or function results, and dates enter through typed properties or functions. The lexer and expression parser that define these forms live under [`analyzer/src/lexer/`](../../analyzer/src/lexer/) and [`analyzer/src/parser/`](../../analyzer/src/parser/).

## Apply operators in the defined order

From highest to lowest precedence, expressions bind in this order:

| Precedence | Form | Associativity |
| --- | --- | --- |
| Highest | call and postfix-call suffixes | left-to-right chaining where the syntax permits |
|  | `^` | right |
|  | prefix `!`, `not`, `-` | prefix |
|  | `*`, `/`, `%` | left |
|  | `+`, `-` | left |
|  | `<`, `<=`, `>=`, `>` | left |
|  | `==`, `!=` | left |
|  | `&&` | left |
|  | `||` | left |
| Lowest | `condition ? then : otherwise` | right |

Parentheses override this order. In particular, `2 ^ 3 ^ 2` means `2 ^ (3 ^ 2)`, `-2 ^ 2` means `-(2 ^ 2)`, and chained ternaries associate through the `otherwise` branch. [`BinOp::infix_binding_power`](../../analyzer/src/parser/ast.rs) is the parser anchor for this ordering.

## Treat analysis as best effort

Analysis is designed to remain useful while source is being edited. Lexer or parser diagnostics can coexist with recovered syntax, semantic analysis can assign `unknown` to an unbound identifier or an indeterminate operator, and branches of different known types can produce a union type.

These results are not compile-time proof that evaluation will succeed. For example, `"count: " + 3` can infer `unknown` and still evaluate to text, while an unbound identifier can remain `unknown` during analysis and fail when a row reaches it. The current inference rules are anchored in [`analyzer/src/analysis/infer.rs`](../../analyzer/src/analysis/infer.rs).

Supported evaluation starts from source with no lexer or parser diagnostics and an expression accepted by semantic preparation. A recovered syntax tree does not make malformed source supported for evaluation. Conversely, imprecise inference by itself is not rejection. Formula diagnostic prose explains a problem but is not a machine-readable compatibility key; integrations must not branch on the exact English sentence.

## Evaluate ordinary operators row by row

After their operands evaluate successfully and are non-null, ordinary operators have these meanings:

| Operators | Supported operands | Result |
| --- | --- | --- |
| unary `-` | number | negated number |
| unary `!`, `not` | boolean | negated boolean |
| `+` | two numbers | numeric addition |
| `+` | either operand is text | text concatenation after converting the other operand to text |
| `-`, `*`, `/`, `%`, `^` | two numbers | subtraction, multiplication, division, remainder, or exponentiation |
| `==`, `!=` | any two non-null values | value equality or inequality; values of different kinds are unequal |
| `<`, `<=`, `>=`, `>` | two orderable numbers, two texts, two booleans, or two dates | same-kind ordering |

Text concatenation renders integral numbers without a `.0`, booleans as lowercase `true` or `false`, dates as their epoch-millisecond integer, and lists as bracketed comma-separated values whose items use the same conversion. Ordering is numeric for numbers, lexical for text, `false` before `true` for booleans, and chronological for dates. `NaN` is not an orderable number; a relational comparison involving it produces a row-level type failure.

Division or remainder by zero fails the affected row. Other unsupported operand combinations produce a row-level type failure. Equality is the exception: different non-null value kinds compare as unequal rather than failing. These rules are implemented in [`evaluator/src/runtime/operators.rs`](../../evaluator/src/runtime/operators.rs).

## Distinguish null, failure, and skipped work

Null is a successful absence of a value, not an evaluation error. After operands have evaluated without an error, unary operators and ordinary non-logical binary operators return null when a required operand is null.

Evaluation order changes what can be observed:

- Non-logical binary operators evaluate both operands for every row that reaches the expression. An error from either operand fails that row even if the other operand is null.
- A list literal evaluates every item. If any item fails, the row fails; otherwise, if any item is null, the whole list expression is null.
- `left && right` supports boolean or null operands. It evaluates `right` only when `left` is `true`; `false` or null on the left returns `false` without evaluating the right side. A null right side reached from `true` returns null.
- `left || right` supports boolean or null operands. `true` skips the right side and returns `true`; `false` or null evaluates the right side and returns its boolean or null result.
- `condition ? then : otherwise` evaluates only `then` for `true`, and only `otherwise` for `false` or null. The supported condition domain is boolean or null; behavior for other condition types is not part of this contract.

An expression that is skipped by `&&`, `||`, or a ternary branch cannot contribute a runtime error. This laziness applies to expression execution, not to discovery of referenced properties, which is defined in the [formula-reference specification](formula-references.md).

## Keep whole-formula and row failures separate

A source, property context, or required input that cannot be prepared rejects the formula or evaluation before a row result exists. Once evaluation starts, runtime failures such as divide-by-zero and type errors remain local to the row that reaches them; other rows can still return values or null. Null therefore does not mean failure, and one failing row does not invalidate successful rows.

The distinct null and row-failure outcomes are exercised by [`evaluator/tests/runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs). Builtin functions may define more specific null and control-flow behavior; their specification owns those exceptions.

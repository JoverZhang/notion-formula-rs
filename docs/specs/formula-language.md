---
doc_id: specs.formula-language
title: "Formula grammar and evaluation rules"
language: en
source_language: zh-CN
counterpart: ./formula-language.zh-CN.md
implementation_status: current
document_status: stable
translation_status: needs-update
last_verified: 2026-09-18
---

# Formula grammar and evaluation rules

[简体中文](formula-language.zh-CN.md) · [Specification index](README.md)

Current: complete expressions accepted by this repository, not a promise of full upstream Notion compatibility.
IDE recovery for incomplete source does not extend this grammar.

## EBNF

```ebnf
(* | choice; , concatenation; [ ] optional; { } repetition; ? ... ? lexical condition; - set difference. *)
(* In trivia, \t, \r, and \n denote tab, CR, and LF control characters. *)
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
expressions = expression, { ",", expression } ; (* No trailing comma *)

boolean     = "true" | "false" ;
number      = digits, [ ".", digits ], [ ( "e" | "E" ), [ "+" | "-" ], digits ] ;
digits      = digit, { digit } ;
digit       = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
string      = '"', { string-char | escape }, '"' ;
string-char = ? Any Unicode scalar except double quote and backslash; includes raw newlines ? ;
escape      = '\', ( "n" | "t" | '"' | '\' ) ;
identifier  = identifier-token - keyword ;
identifier-token = ( "_" | letter ), { "_" | alphanumeric } ;
letter      = ? Rust char::is_alphabetic ? ;
alphanumeric = ? Rust char::is_alphanumeric ? ;
keyword     = "true" | "false" | "not" ;

(* Trivia may separate tokens, never split a lexical token. *)
trivia        = " " | "\t" | "\r" | "\n" | line-comment | block-comment ;
line-comment  = "//", { ? Any scalar except LF ? } ;
block-comment = "/*", ? Text up to the first */; no nesting ?, "*/" ;
EOF           = ? End of input ? ;
```

```text
-2^2        == -(2^2)       // ^ binds above prefix operators; ^ and ?: associate right, other binary operators left
2^3^2       == 2^(3^2)
2^-2        == 2^(-2)
a?b:c?d:e   == a?b:(c?d:e)

3.method()                  // A number consumes . only when a digit immediately follows it
.5                          // Unsupported
f(1).method(2)              // Member calls chain; evaluation also requires builtin postfix support
(f)(1), f()(1), value.field // Unsupported: ordinary calls require an identifier callee; no bare member access
null and date literals      // Unsupported; nulls and dates enter through properties or functions
```

## Property references

```text
prop("Name")                // Exactly one double-quoted string literal; lookup uses decoded text
prop("Na" + "me")           // Not a valid property reference
prop(1), prop("A", "B")     // Likewise
x.prop("Name")             // Not recognized as a property reference

lookup
  Current uses exact, case-sensitive property names from the context.
  Names must be unique; selection among duplicates is unspecified. Missing name → semantic diagnostic, prepare fails.
  prepare collects references from every branch, including runtime-skipped branches; deduplicates in first-source-occurrence order.
  Every required input must exist and match its type/column layout; missing input is not null.

rename
  No automatic source rewriting or retargeting of prepared formulas.
  The host updates source and prepares again; existing prepared formulas still require the original context's inputs.

boundary
  Current production has no persisted FormulaId/FormulaName, formula references, or rename API.
  The demo's FormulaId is UI identity only; the Planned Engine ID/dependency model is not Current behavior.
```

See [FormulaEngine](formula-engine.md) for Planned definitions.

## Operators and nulls

```text
op             Non-null operands and successful result
-x             number → number
!x / not x     boolean → boolean
a + b          two numbers → addition; either operand a string → stringify both and concatenate
a - * / % ^ b  two numbers → number; division/remainder by zero → row error
a == b / !=    any non-null values; different value kinds are unequal
a < <= >= > b  same-kind number/string/boolean/date → boolean; NaN is unordered → row type error

Other non-null unary/non-logical binary operand combinations → row type errors; ==/!= permit different kinds.
Comparison order: numeric for numbers, lexical for strings, false < true for booleans, chronological for dates.
Stringification: integers omit .0, booleans are lowercase, dates are epoch-millisecond integers,
                 lists use brackets and commas around recursively formatted elements.

Non-logical operators evaluate both sides: any error wins; otherwise any null produces null.
Unary operators: null → null.
List literals evaluate every element: any error fails; otherwise any null makes the whole list expression null.

a && b: a=true → evaluate b; a=false/null → false and skip b; reached b=null → null.
a || b: a=true → true and skip b; a=false/null → evaluate b, yielding boolean/null.
a ? b : c: a=true → b; a=false/null → c; condition accepts only boolean/null.
Skipped expressions produce no row errors; prepare still discovers every property reference.
```

## Analysis and failure boundaries

```text
analyze(source)     → best-effort diagnostics, tokens, and types; not proof of executability
prepare / validation → source/schema/input problems fail before row evaluation
evaluate            → runtime failures are row-local; other rows can continue

Current inference permits unknown, unions, and nested unknown.
Unbound identifiers or indeterminate inference need not be rejected immediately; syntax diagnostics block evaluation.
For example, "count: " + 3 can infer unknown while still concatenating at runtime.
Diagnostic messages are not a machine interface; Planned Engine Ready requires concrete Type, a stricter target contract.
```

See [builtins](builtin-functions.md) for signatures and controlled evaluation.
Implementation anchors: [lexer](../../analyzer/src/lexer/mod.rs), [parser](../../analyzer/src/parser/expr.rs),
[precedence](../../analyzer/src/parser/ast.rs), [operators](../../evaluator/src/runtime/operators.rs),
[reference and short-circuit tests](../../evaluator/tests/runtime_structure.rs).

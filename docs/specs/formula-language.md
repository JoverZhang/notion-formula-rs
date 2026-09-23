---
doc_id: specs.formula-language
title: "Formula Grammar and Evaluation Rules"
language: en
source_language: zh-CN
counterpart: ./formula-language.zh-CN.md
implementation_status: current
document_status: stable
translation_status: needs-update
last_verified: 2026-09-23
---

# Formula Grammar and Evaluation Rules

[简体中文](formula-language.zh-CN.md) · [Specification index](README.md)

Current: Describes the complete expressions accepted by this repository; compatibility with upstream Notion is not guaranteed. IDE recovery for incomplete source does not extend this grammar.

## EBNF

```ebnf
(* | means choice; , concatenation; [ ] optional; { } repetition; ? ... ? lexical condition; - set difference. *)
(* In trivia, \t, \r, and \n denote the tab, CR, and LF control characters, respectively. *)
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
expressions = expression, { ",", expression } ; (* Trailing commas are not accepted. *)

boolean     = "true" | "false" ;
number      = digits, [ ".", digits ], [ ( "e" | "E" ), [ "+" | "-" ], digits ] ;
digits      = digit, { digit } ;
digit       = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
string      = '"', { string-char | escape }, '"' ;
string-char = ? Any Unicode scalar other than double quote or backslash; includes raw newlines ? ;
escape      = '\\', ( "n" | "t" | '"' | '\\' ) ;
identifier  = identifier-token - keyword ;
identifier-token = ( "_" | letter ), { "_" | alphanumeric } ;
letter      = ? Rust char::is_alphabetic ? ;
alphanumeric = ? Rust char::is_alphanumeric ? ;
keyword     = "true" | "false" | "not" ;

(* Trivia is allowed between tokens; it cannot be inserted inside a lexical token. *)
trivia        = " " | "\t" | "\r" | "\n" | line-comment | block-comment ;
line-comment  = "//", { ? Any scalar other than LF ? } ;
block-comment = "/*", ? Text through the first */; nesting is not allowed ?, "*/" ;
EOF           = ? End of input ? ;
```

```text
-2^2        == -(2^2)       // ^ binds more tightly than prefix operators; ^ and ?: associate right, all other binary operators associate left
2^3^2       == 2^(3^2)
2^-2        == 2^(-2)
a?b:c?d:e   == a?b:(c?d:e)

3.method()                  // A . in a number belongs to the fractional part only when immediately followed by a digit
.5                          // Not supported
f(1).method(2)              // Chained member calls are allowed; whether they can be evaluated also depends on the builtin's postfix capability
(f)(1), f()(1), value.field // Not supported: a regular call's callee must be an identifier; bare member access is unavailable
null, date literal          // Not supported; null values and dates are produced by properties or functions
```

## Lexical Structure

The following are Rust Analyzer's lexical types; Planned FormulaDraft reuses these types.
Tokens are ordered by source position and retain comments, newlines, and Eof, but not spaces, tabs, or CR; a lexical error may stop scanning early.

```rust
/// UTF-8 byte range [start, end).
pub struct Span {
    pub start: u32,
    pub end: u32,
}

pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub enum TokenKind {
    Lt, Le, EqEq, Ne, Ge, Gt,
    AndAnd, OrOr, Bang, Not,
    Plus, Minus, Star, Slash, Percent, Caret,
    Dot, Comma, Colon, Pound, Question,
    OpenParen, CloseParen, OpenBracket, CloseBracket,
    Literal(Lit),
    Ident(Symbol),
    /// Symbol.text excludes the comment delimiters.
    DocComment(CommentKind, Symbol),
    Newline,
    /// span is an empty range at the end of the source.
    Eof,
}

pub struct Symbol {
    pub text: String,
}

pub struct Lit {
    pub kind: LitKind,
    /// Retains the original spelling; String includes double quotes and escape sequences are not decoded.
    pub symbol: Symbol,
}

pub enum LitKind { Bool, Number, String }
pub enum CommentKind { Line, Block }
```

[Token types](../../analyzer/src/lexer/token.rs) are provided by Analyzer;
a frontend can use kind and span to highlight the original expression or display `prop("id")` as a field label.

## Property Reference

```text
prop("Name")                // Exactly one double-quoted string literal; looked up by its decoded text
prop("Na" + "me")           // Not a valid property reference
prop(1), prop("A", "B")     // Same
x.prop("Name")             // Not recognized as a property reference

lookup
  Current uses the exact, case-sensitive property name from context.
  Names must be unique; selection behavior for duplicate names is undefined. Missing name → semantic diagnostic, prepare fails.
  prepare collects references from all branches, including branches skipped at runtime; deduplicates them and orders them by first source occurrence.
  Every required input must exist and satisfy its type/column layout; a missing input is not null.

rename
  Does not rewrite source automatically or retarget an already prepared formula to a new name.
  The host is responsible for updating source and preparing again; an existing prepared formula still requires inputs from the original context.

boundary
  Current product has no persistent FormulaId/FormulaName, formula reference, or rename API.
  The demo's FormulaId is only a UI identity; the Planned Engine ID/dependency model must not be treated as Current behavior.
```

See [FormulaEngine](formula-engine.md) for the Planned definition.

## Operators and Nulls

```text
op             non-null operands and successful result
-x             number → number
!x / not x     boolean → boolean
a + b          two numbers → addition; if either is a string → convert both sides to text, then concatenate
a - * / % ^ b  two numbers → number; divisor/modulus of 0 → row error
a == b / !=    any non-null values; different value kinds are unequal
a < <= >= > b  same kind among number/string/boolean/date → boolean; NaN cannot be ordered → row type error

Unary/non-logical binary operand combinations not listed above → row type error; ==/!= accept different kinds and are not in this category.
Comparison order: numbers by numeric value, strings lexicographically, booleans false < true, dates chronologically.
Text conversion: integers have no .0, booleans are lowercase, dates are epoch-millisecond integers,
                 lists use square brackets and commas around recursively converted elements.

Non-logical operators: evaluate both sides; if either errors, report an error; otherwise, if either is null, return null.
Unary operators: null → null.
List literal: evaluate every element; if any errors, report an error; otherwise, if any is null, the entire list expression is null.

a && b: a=true → evaluate b; a=false/null → false, skip b; if evaluation reaches b=null → null.
a || b: a=true → true, skip b; a=false/null → evaluate b, result is boolean/null.
a ? b : c: a=true → b; a=false/null → c; condition accepts only boolean/null.
A skipped expression produces no row error; this does not change the rule that all properties are discovered during prepare.
```

### Planned Number

FormulaEngine Number values, numeric operations, and comparisons follow
[ECMAScript Number](https://tc39.es/ecma262/multipage/ecmascript-data-types-and-values.html#sec-ecmascript-language-types-number-type);
numeric builtins with corresponding operations use the same rules, and `sqrt`, `ln`, etc. correspond to
[Math](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math-object).

```text
Inputs and results allow NaN, +Infinity, -Infinity, +0, and -0; they are not marked null and do not produce row errors.
1 / 0, divide(1, 0)         → +Infinity
-1 / 0                      → -Infinity
0 / 0, 1 % 0, sqrt(-1)       → NaN
ln(0)                       → -Infinity
1e308 * 1e308                → +Infinity
NaN in ==, <, <=, >, >=      → false
NaN in !=                    → true
+0 == -0                     → true

An individual function may still restrict argument values, for example repeat requires a finite, non-negative count.
```

## Analysis and Failure Boundaries

```text
analyze(source)   → best-effort recovery of diagnostics, tokens, and types; not "proof of executability"
prepare / input validation → source/schema/input issues fail before per-row evaluation
evaluate          → runtime issues are per-row errors; other rows can continue

Current inference allows unknown, unions, and nested unknown.
Unknown identifiers or uncertain inference need not be rejected immediately; syntax diagnostics prevent evaluation.
For example, "count: " + 3 may infer as unknown, but at runtime it can still concatenate text.
Diagnostic message is not a machine interface; Planned Engine Ready must have a definite ValueType, a stricter target contract.
```

See [builtins](builtin-functions.md) for function call signatures and controlled evaluation.
Implementation anchors: [lexer](../../analyzer/src/lexer/mod.rs), [parser](../../analyzer/src/parser/expr.rs),
[precedence](../../analyzer/src/parser/ast.rs), [operators](../../evaluator/src/runtime/operators.rs),
[reference and short-circuit tests](../../evaluator/tests/runtime_structure.rs).

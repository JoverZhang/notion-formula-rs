# analyzer (Design)

Design rationale for the `analyzer` crate.
For implementation details, see `analyzer/README.md`.

## Purpose

Recovering compiler for a Notion-like formula language: lex, parse, semantic analysis.
Produces AST + diagnostics for IDE use and downstream evaluation.

## Pipeline

```
  Source (UTF-8 string)
       |
       v
  Lexer ──> Tokens + Trivia + Lex diagnostics
       |      (analyzer/src/lexer/)
       |
       |    Token stream includes trivia and explicit Eof.
       |    See contracts.md "Token stream" for invariants.
       v
  Parser (Pratt) ──> AST (ExprKind tree) + Parse diagnostics
       |              (analyzer/src/parser/)
       |
       |    Recovers with ErrorExpr placeholders.
       |    Some diagnostics carry code actions (quick fixes).
       v
  Semantic Analysis ──> TypeMap + Semantic diagnostics
       |                 (analyzer/src/analysis/)
       |
       |    Uses builtin_fn Context for type inference.
       |    Validates call arity, types, prop() calls.
       v
  AnalyzeResult { diagnostics, tokens, output_type }
```

## Key types

| Type | Location | Role |
| --- | --- | --- |
| `Span` | `analyzer/src/span.rs` | Byte offset range `[start, end)` |
| `Spanned<T>` | `analyzer/src/span.rs` | Value with associated span |
| `Token` | `analyzer/src/lexer/token.rs` | Kind + span + value |
| `TokenQuery` | `analyzer/src/parser/tokenstream.rs` | Trivia-aware neighbor API (see `contracts.md`) |
| `ExprKind` | `analyzer/src/parser/ast.rs` | Closed set of expression forms |
| `Diagnostic` | `analyzer/src/diagnostics.rs` | Error/warning with span, labels, notes, actions |
| `CodeAction` | `analyzer/src/diagnostics.rs` | Quick-fix title + edits |
| `TextEdit` | `analyzer/src/text_edit.rs` | Byte-range edit `{ range, new_text }` |
| `Context` | `analyzer/src/analysis/mod.rs` | Properties + functions for semantic analysis |
| `TypeMap` | `analyzer/src/analysis/mod.rs` | ExprId -> Ty mapping |

## Contracts

### Recovery

- Parser inserts `ExprKind::Error` and keeps parsing on syntax errors.
- Parsing never stops because of local errors; still produces full AST + diagnostics.

### Trivia in AST

- Trivia (groups, newlines, comments) is kept in the AST so formatting can reuse the same structure.
- This avoids maintaining a separate CST in `ide`; for this lightweight grammar, the extra cost is acceptable.

### Diagnostic actions

- Some diagnostics carry code actions (e.g. missing parentheses or commas) for lightweight quick fixes.
- Builtin signature ownership is delegated to `builtin_fn`; `analyzer::semantic` re-exports the shared types.

### `prop("Name")` (special-cased)

`prop` is not modeled as a `FunctionSig`.

Rules:

- Expects exactly 1 argument.
- Argument must be a string literal.
- Name must exist in `Context.properties` (else emit a diagnostic).
- Where: `analyzer/src/analysis/mod.rs` (`validate_prop_call`)

### Postfix sugar (member-call)

The parser accepts member *calls*, `receiver.method(...)`, but not bare member access. Semantic
analysis supports the member-call form only for builtins in `postfix_capable_builtin_names()`.
That allowlist filters `builtins_functions()` through `is_postfix_capable`, which requires a
deterministic receiver parameter and at least one additional argument slot.

Before inference, `desugar_member_calls` rewrites an eligible call to
`method(receiver, ...)`. Type inference, call resolution, and validation then use the same path as
the prefix form. A receiver type mismatch is therefore reported as an ordinary argument mismatch.

Any `MemberCall` left after desugaring is invalid. Its children are still analyzed, its result type
is `Unknown`, and validation emits a diagnostic on the full member call:

- a known callable, including the special-cased `prop`, reports that it does not support postfix
  calls; and
- an unknown method reports an unknown function.

The desugaring pass lives in `analyzer/src/analysis/desugar.rs`; eligibility and validation live in
`analyzer/src/analysis/mod.rs`.

IDE completion for postfix (`receiver.$0` / `receiver.pre$0`):

- Start from the same postfix-capable builtin allowlist.
- Keep only functions where the first postfix parameter accepts receiver type (`ty_accepts`).
- If receiver infers to `Unknown`, keep the full postfix-capable set.
- Where: `ide/src/completion/items.rs`

## Source pointers

- Lexer: `analyzer/src/lexer/`
- Parser: `analyzer/src/parser/` (Pratt parser in `expr.rs`, AST in `ast.rs`)
- Semantic analysis: `analyzer/src/analysis/` (inference in `infer.rs`, validation in `mod.rs`)
- Diagnostics model: `analyzer/src/diagnostics.rs`
- Span/Spanned: `analyzer/src/span.rs`
- TextEdit: `analyzer/src/text_edit.rs`
- Entry points: `analyzer/src/lib.rs` (`analyze_syntax`, `analyze`, `infer_expr_with_map`, `format_diagnostics`)

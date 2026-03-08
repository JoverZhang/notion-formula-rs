# Drift Tracker / Open Questions

This file tracks unresolved specs, known gaps, and items we still need to converge.

- ~~Numeric grammar: we still need a clear spec and tests for decimals and scientific notation.~~ Resolved: lexer supports integers, floating-point decimals, and scientific notation with lookahead disambiguation for dots.
- ~~String escape grammar: escape rules and invalid-character handling still need to be specified precisely.~~ Resolved: lexer recognises `\n`, `\t`, `\"`, `\\`; invalid escapes emit a diagnostic and are kept verbatim. Parser unescapes in AST.
- ~~Identifier policy: charset scope and normalization strategy still need to be defined.~~ Resolved: identifiers use `is_alphabetic()` / `is_alphanumeric()` (Unicode Alphabetic property). Emoji and non-letter symbols are rejected.
- Postfix parity: postfix validation still needs to converge with inference/completion behavior (see `docs/design/builtins-and-types.md`).
- Evaluator runtime coverage: only literal evaluation and binary arithmetic (`+`, `-`, `*`, `/`) are implemented; `prop(...)`, `if(...)`, `&&`, `||`, comparisons, unary ops, and builtin function calls still need convergence.

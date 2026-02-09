# Tokens, spans, and token ranges

Core coordinate model used by `analyzer/`.

## Span (core)

- Type: `Span { start: u32, end: u32 }`
- Meaning: UTF-8 **byte** offsets into the original source.
- Semantics: half-open `[start, end)`.
- With valid boundaries, slicing is `&source[start..end]`.
- Code: `analyzer/src/lexer/token.rs`

## Token stream basics

- The lexer emits trivia tokens and an explicit EOF token.
- Trivia:
  - `TokenKind::DocComment(..)`
  - `TokenKind::Newline`
- EOF:
  - `TokenKind::Eof` with an empty span.
- Code: `analyzer/src/lexer/token.rs`

## `TokenRange` and `tokens_in_span`

`tokens_in_span(tokens, span)` maps a byte span to a half-open token index range:

- Return type: `TokenRange` (token indices, `[lo, hi)`).
- Handles:
  - empty spans (stable insertion-point behavior)
  - trivia tokens and EOF (EOF has an empty span)
- Code: `analyzer/src/lexer/token.rs` (`tokens_in_span`)

## Trivia token details

- `TokenKind::DocComment(CommentKind, Symbol)`:
  - `// ...` → `CommentKind::Line`
  - `/* ... */` → `CommentKind::Block`
- `TokenKind::Newline`
- Code: `analyzer/src/lexer/token.rs`

## Tests

- `tokens_in_span` behavior: `analyzer/src/tests/lexer/test_tokens_in_span.rs`
- Span/token invariants: `analyzer/src/tests/parser/test_invariants.rs`

# TokenQuery

`TokenQuery` is the canonical trivia-aware token neighbor API.

## Location

- `analyzer/src/parser/tokenstream.rs`

## What it does

`TokenQuery<'a>` centralizes:

- `Span` → `TokenRange` mapping
- trivia scanning and neighbor lookup
- half-open token index rules (`[lo, hi)`)

## API surface (stable)

- `range_for_span(span) -> TokenRange`
- `prev_nontrivia(idx)`
- `next_nontrivia(idx)`
- `first_in_range(range)`
- `last_in_range(range)`
- `leading_trivia_before(idx)`
- `trailing_trivia_until_newline_or_nontrivia(idx)`
- `bounds_usize(range)`

## Design intent

- One place for trivia/neighbor scanning.
- Avoid duplicated index math in formatter/comment attachment.

## Tests

- `analyzer/src/tests/parser/test_token_query.rs`


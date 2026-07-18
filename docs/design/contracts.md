# Contracts (Hard Rules)

This file records stable contracts across crates.
If a contract changes, it needs tests + docs + changelog.

## Spans and offsets

Rule: Core spans/offsets use UTF-8 bytes, with half-open ranges `[start, end)`.
Rule: `Span { start: u32, end: u32 }` -- UTF-8 byte offsets into source.
Rule: With valid boundaries, slicing is `&source[start..end]`.
Rule: DTO spans/edits use UTF-16 code units, with half-open ranges `[start, end)`.
Rule: Coordinate conversion only happens at the WASM boundary layer.
Rule: `Diagnostic.line`/`col` are computed from byte offsets during WASM conversion.
Where: `analyzer/src/span.rs`, `analyzer/src/lexer/token.rs`, `analyzer_wasm/src/dto/v1.rs`, `analyzer_wasm/src/offsets.rs`, `analyzer_wasm/src/span.rs`, `analyzer_wasm/src/converter/shared.rs`.

## Token stream

Rule: The token stream includes trivia (`DocComment`, `Newline`) and explicit `Eof`.
Rule: `Eof` has an empty span.
Rule: `TokenQuery` is the trivia-aware source-of-truth API for span-to-token mapping.

Trivia token kinds:

- `TokenKind::DocComment(CommentKind, Symbol)`:
  - `// ...` -> `CommentKind::Line`
  - `/* ... */` -> `CommentKind::Block`
- `TokenKind::Newline`

`TokenRange` and `tokens_in_span`:

- `tokens_in_span(tokens, span)` maps a byte span to a half-open token index range `[lo, hi)`.
- Handles empty spans (stable insertion-point behavior), trivia tokens, and EOF.
- Where: `analyzer/src/lexer/token.rs`

`TokenQuery` API surface (stable):

- `range_for_span(span) -> TokenRange`
- `prev_nontrivia(idx)` / `next_nontrivia(idx)`
- `first_in_range(range)` / `last_in_range(range)`
- `leading_trivia_before(idx)` / `trailing_trivia_until_newline_or_nontrivia(idx)`
- `bounds_usize(range)`
- Design intent: one place for trivia/neighbor scanning; avoids duplicated index math in formatter/comment attachment.
- Where: `analyzer/src/parser/tokenstream.rs`

Tests:

- `tokens_in_span`: `analyzer/src/tests/lexer/test_tokens_in_span.rs`
- Span/token invariants: `analyzer/src/tests/parser/test_invariants.rs`
- `TokenQuery`: `analyzer/src/tests/parser/test_token_query.rs`

## AST + syntax invariants

Rule: `ExprKind` is the closed set of expression forms.
Rule: The parser recovers with `ExprKind::Error` and keeps parsing.
Rule: Member access cannot be used bare; it must be `receiver.method(...)`.

## Diagnostics determinism

Rule: Diagnostics with the same span are deduped by priority.
Rule: `format_diagnostics` output order is stable (span, priority, message).

## Actions and edits

Rule: Quick fixes are exposed as `Diagnostic.actions: Vec<CodeAction>`.
Rule: The core edit model is `TextEdit { range, new_text }` in byte coordinates.
Rule: The WASM edit model uses UTF-16 coordinates.
Rule: `ide::format` and `ide::apply_edits` take a byte cursor and return `{ source, cursor }`.
Rule: WASM `format/apply_edits` take a UTF-16 cursor and return a UTF-16 cursor.
Rule: `ide::format` goes through the same byte-edit pipeline via one full-document `TextEdit`.
Rule: WASM only converts coordinates and forwards calls; failures are returned as `Err`.

## Signature help

Rule: Parameter shape follows `ParamShape { head, repeat, tail }`.
Rule: Output is structured `DisplaySegment[]`, rendered directly by the UI.
Rule: Active-parameter behavior is defined in `docs/signature-help.md`.

## WASM `AnalyzerConfig`

Rule: Constructor argument must be an object.
Rule: Unknown top-level fields are rejected.
Rule: Current schema is `{ properties?: Property[], preferred_limit?: number | null }`.
Rule: `preferred_limit = null` uses default `5`.
Rule: `functions` come from Rust built-ins; JS does not provide them.

## Evaluator row-batch runtime

These are the accepted target contracts for the evaluator. Current implementation status
is recorded in [`evaluator/README.md`](../../evaluator/README.md).

Rule: `Value` is data-only; row errors are externalized via `EvalBlock { ok, errors }`.
Rule: `ok[i] = false` means `values[i]` is placeholder-only and must not be consumed by callers.
Rule: `PreparedFormula::required_columns()` exposes every statically referenced property as a deduplicated plan-local `InputSlot`.
Rule: callers prepare every required column before evaluation; external async loading stays outside evaluator.
Rule: `EvalInputsBuilder` validates slot, ABI kind, batch length, and input layout before any kernel runs; failures return `InputContractError`.
Rule: execution mask, row `ok`, and null `Validity` are independent states.
Rule: `if(cond, then, else)`, `&&`, and `||` are mask-driven; right/branch sides are evaluated only for required rows.

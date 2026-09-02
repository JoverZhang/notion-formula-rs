---
doc_id: how.analyzer
title: "How the analyzer preserves useful results from incomplete formulas"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How the analyzer preserves useful results from incomplete formulas

[简体中文](README.zh-CN.md)

The `analyzer` crate turns a UTF-8 formula source string into syntax and semantic artifacts that
remain useful while the source is incomplete. This guide explains the recovery pipeline, the
coordinate and diagnostic models that hold it together, and the internal seams used by the IDE and
evaluator. It is for maintainers who need to change or debug lexing, parsing, or semantic analysis.

This guide describes the Current Rust implementation. It does not define the supported formula
language, builtin catalog, editor behavior, WASM API, or evaluator runtime contract; those
user-facing facts belong in `docs/specs/`.

## The pipeline preserves evidence instead of returning valid or invalid

Each stage keeps the artifacts it can produce and reports problems separately:

```text
UTF-8 source
    |
    | lex
    v
tokens + lexical diagnostics
    |
    | parse (skip trivia for grammar decisions)
    v
Expr, including Error nodes + parse diagnostics
    |
    | semantic analysis mutates Expr
    +-- desugar eligible MemberCall nodes
    +-- infer types and resolve builtin calls
    +-- validate calls
    v
root Ty + SemanticMap + semantic diagnostics
```

The diagram shows processing order, not one public return type. `analyze_syntax` exposes the
recovered expression, tokens, and syntax diagnostics. The convenience function `analyze` runs all
stages but returns only the tokens, combined diagnostics, and root output type. A caller that needs
the normalized expression and its `SemanticMap` composes the syntax and semantic entry points.

### Follow one formula through the stages

Suppose `Context` contains the supported builtin signatures and declares `Scores` as a list of
numbers. For this source:

```text
prop("Scores").map(current + 1)
```

the stages add meaning in this order:

1. The lexer emits source-ordered tokens and an explicit `Eof`. Comments and newlines would remain
   in the same stream as trivia.
2. The Pratt parser builds a `MemberCall` whose receiver is `prop("Scores")`.
3. `desugar_member_calls` recognizes `map` as postfix-capable and rewrites the node to
   `map(prop("Scores"), current + 1)`.
4. Inference uses the shared builtin resolver to project the mapper parameter. It binds `current`
   to `Number` in a temporary lexical scope, infers the body, and wraps that body in an
   `ImplicitLambda` node.
5. The final call resolution and expression types remain in `SemanticMap`; validation consumes
   that same resolution instead of binding the call again. The root type is `List(Number)`.

`builtin_fn` owns the signature model and call resolver. The analyzer owns the expression walk,
temporary scopes, AST rewrites, semantic facts, and source diagnostics that apply that model.

## Byte spans connect source, tokens, expressions, and edits

[`Span`](../../../analyzer/src/span.rs) and [`TextEdit`](../../../analyzer/src/text_edit.rs) use
half-open UTF-8 byte ranges, `[start, end)`. Analyzer-produced endpoints are valid character
boundaries in their source. A caller constructing either type must preserve that invariant; this
crate performs no UTF-16 conversion.

[`SourceMap::line_col`](../../../analyzer/src/source_map.rs) is a display helper, not another source
coordinate system. It clamps an input byte offset down to a valid character boundary and returns
1-based line and column values. The column counts Unicode scalar values (`char`), not bytes or
UTF-16 code units.

The lexer retains line comments, block comments, and newlines as trivia. It skips spaces, tabs, and
carriage returns, then always appends an empty `Eof` token at the source length. The parser skips
trivia when choosing grammar productions but returns the original token stream next to the AST.
Trivia therefore lives in the source and token stream, not in `ExprKind`.

Token ranges use a different unit. [`tokens_in_span`](../../../analyzer/src/lexer/token.rs) maps a
non-empty byte span to the half-open range of token indices whose spans intersect it. Such a range
does not include the empty `Eof` token. An empty or reversed span maps to the stable insertion point
before the first token whose start is at or after `span.start`; that point may be the `Eof` index.
[`TokenQuery`](../../../analyzer/src/parser/tokenstream.rs) layers trivia-aware neighbor and range
queries on the same rule so consumers do not need another token-index algorithm.

For successfully parsed constructs, a parent expression span is anchored by its first and last
non-trivia token and contains its child spans. Trivia between those anchors is therefore inside the
parent byte range even though it is absent from the AST. The span and token-range invariants are
covered by [`test_invariants.rs`](../../../analyzer/src/tests/parser/test_invariants.rs) and
[`test_tokens_in_span.rs`](../../../analyzer/src/tests/lexer/test_tokens_in_span.rs).

## Syntax recovery repairs structure, not meaning

The parser uses Pratt binding powers for prefix, binary, and ternary expressions. Lists are parsed
as primary expressions; a separate postfix loop consumes prefix calls and member calls after the
primary. When required syntax is missing, the parser can insert `ExprKind::Error` and scan to a
boundary such as a comma, colon, or closing delimiter. This keeps the surrounding expression
available to later tooling.

Delimiter and separator recovery may attach a `CodeAction` containing byte-based `TextEdit` values,
for example to insert a missing `)` or remove a trailing comma. A recovery action describes a local
source repair; it does not certify that the remaining formula is valid. Every AST consumer must
handle `Error` nodes.

The AST preserves expression structure rather than arbitrary call targets. Only an identifier can
be a prefix call callee. Member syntax reaches semantic analysis only in the call form
`receiver.method(...)`; bare `receiver.member` is diagnosed and recovered as an error expression.

## Semantic analysis normalizes before validating

[`analyze_expr_with_semantic_map`](../../../analyzer/src/analysis/mod.rs) applies three ordered
operations to a mutable expression:

1. **Desugar supported postfix calls.** A member call is rewritten only when its method appears in
   `postfix_capable_builtin_names()`. The allowlist starts from the supported signatures returned by
   `builtins_functions()` and applies `is_postfix_capable`. A non-empty parameter head requires at
   least two displayed parameters. When the head is empty, a repeat-first shape requires either two
   displayed parameters or minimum repeat groups that supply at least two positions. A tail-only
   shape is excluded. These rules provide one deterministic receiver slot and leave another
   argument position; later lookup and validation still use `Context.functions`. The filter and its
   semantic boundary tests live in [`analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs) and
   [`test_semantic.rs`](../../../analyzer/src/tests/analysis/test_semantic.rs).
2. **Infer best-effort facts.** Inference visits subexpressions, records their types, and retains the
   final `ResolvedFunctionSig` for each resolved builtin call. Indeterminate expressions become
   `Ty::Unknown`; inference does not emit diagnostics. Function-typed arguments are inferred in a
   temporary lexical scope and wrapped in synthetic `ImplicitLambda` nodes.
3. **Validate calls.** Validation checks the special `prop` form, unknown functions, unsupported
   postfix calls, argument shape, identifier positions, and argument types. A shape error produces
   one diagnostic for that call and suppresses its per-argument mismatch diagnostics.

`prop("Name")` is not represented by a builtin `FunctionSig`. Inference and validation handle it
directly by looking up the string literal in `Context.properties`.

The distinction between the canonical postfix allowlist and `Context.functions` matters for custom
Rust callers. A canonical postfix name can be desugared and then reported as unknown if the supplied
context omits that function. Conversely, a caller-defined function does not become postfix-capable
merely because it appears in the context.

Any `MemberCall` left after desugaring is invalid, but inference still visits its receiver and
arguments before assigning `Unknown` to the call. Validation reports a known callable, including
the special-cased `prop`, as not supporting postfix calls; an unknown method is reported as an
unknown function.

Semantic mutation is observable to callers that retain the AST. After the full pass, eligible
postfix calls are `Call` nodes and function-typed arguments may be `ImplicitLambda` nodes. The
inference-only helpers do not perform the complete desugar-infer-validate sequence.

## Diagnostic aggregation has a local and a final stage

[`Diagnostics`](../../../analyzer/src/diagnostics.rs) arbitrates diagnostics produced during
parsing. For an exact span, a higher-priority incoming parser diagnostic replaces the existing one.
At equal priority, an identical message merges and deduplicates labels, notes, and actions; a
different message leaves the existing diagnostic unchanged. This is why competing recovery paths
at one insertion point can produce one preferred parse error.

That one-per-span behavior is local to the `Diagnostics` accumulator. `analyze_syntax` appends lexer
diagnostics after parsing, and `analyze` later appends semantic diagnostics. The final vector is not
globally deduplicated by span.

`format_diagnostics` makes textual output deterministic by sorting diagnostics by start, end,
descending priority, and message. It also sorts labels while preserving the deduplicated emission
order of notes. Structured code actions remain attached to `Diagnostic.actions`; the text formatter
does not render them. Parser recovery and stable rendering are exercised by
[`test_errors.rs`](../../../analyzer/src/tests/parser/test_errors.rs) and the
[`diagnostics` golden suite](../../../analyzer/tests/diagnostics_golden.rs).

## Failures preserve artifacts up to a defined boundary

Recovery is best-effort rather than unlimited:

| Stage | Failure behavior | Artifact that remains useful |
| --- | --- | --- |
| Lexer | An invalid string escape emits a diagnostic and scanning continues. An unterminated string or block comment, or an unexpected character, stops scanning. | Tokens recognized before the stopping point, an explicit `Eof`, and lexical diagnostics |
| Parser | Missing or mismatched syntax emits diagnostics, may add actions, and inserts `Error` nodes or synchronizes at a delimiter. | The token stream and a best-effort expression |
| Inference | Unresolved identifiers, error nodes, and indeterminate operations become `Ty::Unknown`. | Types for visited expressions and resolved call records where available |
| Validation | Invalid `prop` calls, unknown functions, unsupported postfix calls, and signature mismatches emit semantic diagnostics. | The normalized expression and already-inferred semantic facts |

`analyze` still runs semantic analysis when syntax diagnostics exist. That behavior supports
interactive consumers, but the analyzer does not decide whether a diagnostic should prevent
execution. A stricter consumer must enforce its own handoff boundary.

## Choose the entry point by the artifact you need

| Entry point | Produces | Boundary |
| --- | --- | --- |
| `analyze_syntax(text)` | `ParseOutput { expr, diagnostics, tokens }` | Lexing and parsing only; `expr` may contain `Error` nodes. |
| `analyze(text, ctx)` | `AnalyzeResult { diagnostics, tokens, output_type }` | Runs the full pipeline but does not expose the mutated expression or `SemanticMap`. |
| `analyze_expr(expr, ctx)` | Root `Ty` and semantic diagnostics | Mutates an already parsed expression; syntax diagnostics remain with the caller. |
| `analyze_expr_with_semantic_map(expr, ctx)` | Root `Ty`, `SemanticMap`, and semantic diagnostics | Main seam for consumers that need the final resolved calls. |
| `infer_expr_with_map` / `infer_expr_with_semantic_map` | Best-effort type facts | Emit no diagnostics and do not run the full semantic sequence. |

The crate also re-exports the builtin semantic vocabulary through `analyzer::semantic`. That makes
the shared types convenient to consumers; it does not transfer ownership of declaration or call
projection rules from `builtin_fn` to the analyzer.

## Neighboring crates own behavior after analysis

- `builtin_fn` owns builtin declarations, `ParamShape`, generic binding, and
  `resolve_call_signature`.
- `ide` owns cursor interpretation, completion, signature presentation, formatting, and edit
  application. It consumes analyzer source, tokens, expressions, spans, and best-effort types.
- `analyzer_wasm` owns the JavaScript facade and every UTF-8/UTF-16 conversion.
- `evaluator` consumes a normalized expression and `SemanticMap` to prepare execution; runtime
  values and row failures are evaluator concerns.

Keeping trivia outside the AST makes the expression model smaller, but source-preserving consumers
must retain the source and token stream. Mutating the AST gives downstream consumers one normalized
call form and explicit lambda nodes, but callers cannot treat a parsed tree as immutable across
semantic analysis. `Error` and `Unknown` keep partial analysis useful at the cost of requiring every
consumer to choose its own strictness.

## Read the implementation by stage

- Public composition and result shapes: [`analyzer/src/lib.rs`](../../../analyzer/src/lib.rs)
- Lexical stopping behavior and tokens: [`lexer/mod.rs`](../../../analyzer/src/lexer/mod.rs) and
  [`lexer/token.rs`](../../../analyzer/src/lexer/token.rs)
- Pratt parsing and recovery: [`parser/expr.rs`](../../../analyzer/src/parser/expr.rs) and
  [`parser/ast.rs`](../../../analyzer/src/parser/ast.rs)
- Semantic sequencing, desugaring, and inference: [`analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs),
  [`analysis/desugar.rs`](../../../analyzer/src/analysis/desugar.rs), and
  [`analysis/infer.rs`](../../../analyzer/src/analysis/infer.rs)
- Representative semantic coverage: [`test_semantic.rs`](../../../analyzer/src/tests/analysis/test_semantic.rs),
  [`test_implicit_lambda.rs`](../../../analyzer/src/tests/analysis/test_implicit_lambda.rs), and
  [`test_resolved_calls.rs`](../../../analyzer/src/tests/analysis/test_resolved_calls.rs)

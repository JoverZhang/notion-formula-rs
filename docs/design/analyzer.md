---
doc_id: architecture.analyzer
title: "How does the analyzer keep incomplete formulas useful to tooling?"
language: en
source_language: en
counterpart: ./analyzer.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-31
---

# Recoverable formula analysis

[简体中文](analyzer.zh-CN.md)

This Current document explains how the `analyzer` crate turns formula source into useful syntax
and semantic artifacts even while the user is still typing. It is for contributors who need to
change or debug the lexer, parser, or semantic passes without breaking the editor and evaluator
that consume their results.

The scope starts with a UTF-8 source string and ends with the analyzer's recovered expression,
tokens, diagnostics, and semantic facts. The exact language grammar, cross-crate coordinate
contracts, builtin catalog, IDE behavior, and evaluator runtime are owned by their respective
documents and are linked where they meet this pipeline.

## The pipeline preserves evidence instead of accepting or rejecting a formula

The analyzer does not reduce its answer to “valid” or “invalid.” Each stage preserves the useful
artifacts it can produce and records problems as diagnostics:

```text
UTF-8 source
    │
    │ lex
    ▼
tokens + lex diagnostics
    │
    │ parse (trivia is skipped for grammar decisions)
    ▼
Expr, including Error nodes + parse diagnostics
    │
    │ semantic analysis mutates the Expr
    ├── desugar eligible MemberCall nodes
    ├── infer expression types and resolve builtin calls
    └── validate calls
    ▼
root Ty + SemanticMap + semantic diagnostics
```

The diagram shows the internal artifacts and processing order, not one public return type. For
example, `analyze_syntax` exposes the tokens and recovered `Expr`, while the convenience function
`analyze` exposes tokens, combined diagnostics, and the root output type. Callers that need both a
mutated expression and its `SemanticMap` compose the syntax and semantic entry points explicitly.

All source spans inside this crate are half-open UTF-8 byte ranges. The complete coordinate and
token-range rules live in [Cross-crate Contracts](contracts.md).

## Follow one formula through the pipeline

Suppose the semantic `Context` contains the standard builtin catalog, declares `Scores` as a list
of numbers, and receives this source:

```text
prop("Scores").map(current + 1)
```

The stages give that expression progressively more meaning:

1. The lexer emits source-ordered tokens, including an explicit `Eof`. The formula contains no
   trivia, but comments and newlines would remain in the same token stream.
2. The Pratt parser builds a `MemberCall`. Its receiver is the `prop("Scores")` call, and its
   argument is the still-unbound expression `current + 1`.
3. Before inference, `desugar_member_calls` recognizes `map` as postfix-capable and rewrites the
   node to the equivalent prefix form `map(prop("Scores"), current + 1)`.
4. The shared builtin resolver identifies the mapper parameter as a function from the list element
   type. Inference temporarily binds `current` to `Number`, infers the mapper body as `Number`, and
   wraps that body in an `ImplicitLambda` node. The resolved call and expression types are retained
   in `SemanticMap`; the root type is `List(Number)`.
5. Validation consumes the same resolved call record. It does not reconstruct the `map` signature
   independently. The `prop` call is checked separately against `Context.properties` because
   `prop` is not represented by a builtin `FunctionSig`.

This example also exposes an important ownership rule: `builtin_fn` owns the signature model and
call resolution, while the analyzer owns the expression walk, lexical scopes, AST rewrites, and
diagnostics that apply those shared contracts to source.

## Syntax recovery keeps local structure usable

### The token stream and expression tree preserve different information

The lexer gives every token a source span, retains line and block comments plus newlines as trivia,
and appends an empty `Eof` token at the end of the source. Ordinary spaces, tabs, and carriage
returns are skipped. The parser normally ignores trivia when making grammar decisions but returns
the original token stream beside the expression tree.

Trivia is therefore not stored in the AST. The AST represents expression structure; source fidelity
lives in the source string, token stream, and spans. Consumers such as the formatter combine all
three rather than expecting comments or newlines to appear as `ExprKind` variants.

### The parser repairs shape, not meaning

The parser uses Pratt binding powers to encode operator precedence and associativity while building
unary, binary, ternary, call, list, and member-call expressions. When an expected expression is
missing, it inserts `ExprKind::Error` and scans toward a safe boundary such as a comma, colon, or
closing delimiter. The surrounding expression can then remain available to later tooling.

Delimiter and separator recovery may attach `CodeAction` edits to a diagnostic—for example,
inserting a missing `)` or removing a trailing comma. Recovery does not claim that the formula is
valid, and it does not invent semantic values. Every AST consumer must handle `Error`; a diagnostic
never implies that no tree was produced.

Bare member access is outside the grammar. `receiver.member` produces a parse diagnostic and an
error expression; only the call form `receiver.method(...)` reaches semantic analysis as a
`MemberCall`.

## Semantic analysis normalizes before it validates

`analyze_expr_with_semantic_map` performs three ordered operations on a mutable expression:

1. **Desugar supported postfix calls.** A member call is rewritten only when its method appears in
   `postfix_capable_builtin_names()`. That allowlist is derived from the canonical catalog returned
   by `builtins_functions()`, not from arbitrary caller-provided `Context.functions`. Eligible
   prefix and postfix forms then enter the same inference and validation path.
2. **Infer best-effort facts.** Inference visits subexpressions, records their types, and retains the
   final `ResolvedFunctionSig` for each resolved builtin call. When a type cannot be established, it
   records `Ty::Unknown` instead of emitting a diagnostic. Function-typed arguments are wrapped in
   synthetic `ImplicitLambda` nodes while their bodies are inferred in a temporary lexical scope.
3. **Validate calls.** Validation checks `prop`, unknown functions, unsupported postfix calls,
   argument shape, identifier positions, and argument types. A builtin shape error produces one
   diagnostic for that call and suppresses per-argument mismatch diagnostics for the same call.

Any `MemberCall` left after desugaring is invalid but still has its receiver and arguments analyzed.
A known callable—including the special-cased `prop`—reports that it does not support postfix calls;
an unknown method reports an unknown function. Its result type remains `Unknown`.

The mutation is part of the semantic contract. A caller that retains the parsed `Expr` will observe
postfix calls rewritten to `Call` and function-typed arguments wrapped in `ImplicitLambda` after the
full semantic pass.

## Failures keep the artifacts produced before them

Recovery is best-effort rather than unlimited. The important boundary is what each stage preserves:

| Stage | Failure behavior | Artifact that remains useful |
| --- | --- | --- |
| Lexer | Invalid string escapes emit diagnostics and remain in the string token. An unterminated string or block comment, or an unexpected character, stops the lexical scan. | Tokens recognized before the stopping point, an explicit `Eof`, and lex diagnostics |
| Parser | Missing or mismatched syntax emits diagnostics, may add code actions, and inserts `Error` nodes or synchronizes at a delimiter. | The token stream and a best-effort expression tree |
| Inference | Unresolved identifiers, error nodes, and otherwise indeterminate expressions become `Ty::Unknown`; inference itself emits no diagnostics. | Types for visited expressions and resolved builtin-call records where available |
| Validation | Invalid `prop` calls, unknown functions, unsupported postfix calls, and signature mismatches emit semantic diagnostics. | The already-normalized expression and inferred semantic facts |

The one-shot `analyze` function still runs semantic analysis after syntax diagnostics are produced.
This is useful for interactive feedback, but an execution-oriented caller must decide whether any
diagnostic makes the formula unsuitable for its next stage. The evaluator, for example, rejects
semantic diagnostics while preparing an execution plan.

## Choose an entry point by the artifact you need

| Entry point | Produces | Boundary to remember |
| --- | --- | --- |
| `analyze_syntax(text)` | `ParseOutput { expr, diagnostics, tokens }` | Runs lexing and parsing only; the returned `Expr` may contain `Error` nodes. |
| `analyze(text, ctx)` | `AnalyzeResult { diagnostics, tokens, output_type }` | Runs the full pipeline but does not expose the mutated `Expr` or `SemanticMap`. |
| `analyze_expr(expr, ctx)` | Root `Ty` and semantic diagnostics | Mutates an already parsed expression; syntax diagnostics remain the caller's responsibility. |
| `analyze_expr_with_semantic_map(expr, ctx)` | Root `Ty`, `SemanticMap`, and semantic diagnostics | The main seam for consumers such as evaluator planning that need resolved calls. |
| `infer_expr_with_map` / `infer_expr_with_semantic_map` | Best-effort type facts | Inference-only helpers emit no diagnostics and do not run the full semantic preprocessing and validation sequence. |

IDE features deliberately use different seams. Formatting needs the recovered `Expr` together with
the original source and tokens. Completion and signature help also perform fragment-level,
best-effort inference because the cursor often sits inside an incomplete formula. Those policies
belong to [IDE Design](ide.md), not to the analyzer entry points themselves.

## Sibling crates own the behavior beyond analysis

- [`builtin_fn`](builtin-fn.md) owns builtin declarations, parameter shapes, generic binding, and
  shared call-signature resolution. The analyzer re-exports that semantic vocabulary but does not
  maintain a second signature model.
- [`ide`](ide.md) owns cursor interpretation, completion, signature presentation, formatting, and
  edit application. It consumes analyzer tokens, expressions, spans, and best-effort types.
- [`analyzer_wasm`](wasm-boundary.md) owns the JavaScript facade and every UTF-8/UTF-16 conversion.
  The analyzer itself never changes coordinate systems.
- [`evaluator`](evaluator.md) consumes a semantically analyzed expression and `SemanticMap` to build
  an execution plan. Runtime values, row failures, and input contracts are evaluator concerns.

These boundaries let the analyzer remain useful to both permissive editor workflows and stricter
execution workflows without making either policy universal.

## The design trades immutability for one normalized semantic tree

Keeping trivia outside the AST makes the expression model smaller, but source-preserving consumers
must keep the source and its token stream paired with the tree. Mutating the AST during semantic
analysis gives downstream planning one normalized call form and explicit lambda nodes, but callers
cannot treat the parsed tree as immutable across that pass.

`Error` and `Unknown` also move failure handling to consumers. That cost is intentional for editor
features, which need partial structure and types while a formula is incomplete. Consumers that need
an executable formula must enforce a stricter diagnostic boundary before proceeding.

When extending the analyzer, preserve these seams:

- syntax additions belong in the lexer/parser and must define their recovery behavior;
- builtin signature or call-resolution changes belong in `builtin_fn`;
- postfix eligibility must remain shared by analyzer semantics and IDE presentation; and
- source-coordinate changes are cross-crate contract changes, not local parser refactors.

## Continue reading the implementation

- Public composition and result shapes: [`analyzer/src/lib.rs`](../../analyzer/src/lib.rs)
- Tokens and lexical stopping behavior: [`analyzer/src/lexer/mod.rs`](../../analyzer/src/lexer/mod.rs)
  and [`token.rs`](../../analyzer/src/lexer/token.rs)
- Pratt parsing and recovery: [`analyzer/src/parser/expr.rs`](../../analyzer/src/parser/expr.rs) and
  [`ast.rs`](../../analyzer/src/parser/ast.rs)
- Semantic sequencing and diagnostics: [`analyzer/src/analysis/mod.rs`](../../analyzer/src/analysis/mod.rs),
  [`desugar.rs`](../../analyzer/src/analysis/desugar.rs), and
  [`infer.rs`](../../analyzer/src/analysis/infer.rs)
- Representative recovery and semantic coverage:
  [`test_errors.rs`](../../analyzer/src/tests/parser/test_errors.rs),
  [`test_semantic.rs`](../../analyzer/src/tests/analysis/test_semantic.rs), and
  [`test_implicit_lambda.rs`](../../analyzer/src/tests/analysis/test_implicit_lambda.rs)

The complete test inventory and commands remain in [Testing inventory](testing.md).

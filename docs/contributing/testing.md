---
doc_id: contributing.testing
title: "How to test a change"
language: en
source_language: en
counterpart: ./testing.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How to test a change

[简体中文](testing.zh-CN.md)

Run the smallest test that exercises the changed boundary while developing, then widen the scope
before review when the change crosses crates or languages. The repository's [`justfile`](../../justfile)
is the command authority; this guide explains how to choose among those commands and how to review
generated expectations.

## Start with the component that owns the behavior

| Changed area | First command | What it exercises |
| --- | --- | --- |
| Documentation structure, metadata, translations, or links | `just docs-check` | The checker tests and the repository documentation scan |
| Builtin declarations, call shapes, or resolution | `cargo test -p builtin_fn` | Resolver and declaration-DSL behavior, including macro pass/fail compilation |
| Procedural macro parsing or expansion | `cargo test -p builtin_fn_macros` and `cargo test -p builtin_fn` | Macro implementation units plus the consuming crate's declaration-DSL and compile-pass/compile-fail contracts |
| Lexing, parsing, semantic analysis, or diagnostics | `cargo test -p analyzer` | Analyzer unit, integration, and diagnostic golden tests |
| Formula preparation or row evaluation | `cargo test -p evaluator` | Generated contracts, input/runtime invariants, and builtin behavior |
| Completion, signature help, formatting, or text edits | `cargo test -p ide` | IDE unit, integration, and formatting golden tests |
| Rust-to-JavaScript conversion or exported WASM methods | `cargo test -p analyzer_wasm` and `wasm-pack test --node analyzer_wasm` | Native helpers and tests that execute through `wasm-bindgen` |
| Vite example behavior | `just test-example-vite` | WASM build, Vitest unit tests, and Playwright end-to-end tests |

`cargo test -p analyzer_wasm` does not run the `wasm_bindgen_test` integration suite. Run both WASM
commands when a change can affect serialization, UTF-16 offsets, JavaScript-facing errors, or the
exported methods.

The Vite shortcut installs locked Node dependencies and rebuilds the WASM package before running
the frontend suites. If you invoke `pnpm -C examples/vite test` or `test:e2e` directly, prepare those
dependencies and the generated package first. The scripts in
[`examples/vite/package.json`](../../examples/vite/package.json) are the authority for the individual
frontend commands.

For a cross-cutting Rust change, use `just test` after the focused crate test. It runs the
repository-selected Rust suites and the Vite example tests through their `justfile` recipes. It
does not mean `cargo test --workspace`; use that Cargo command when every current workspace member,
including `builtin_fn_macros`, must run. Use `just verify` when the change also needs the complete
dependency, formatting, lint, type-check, documentation, and test workflow. `just docker-test` runs
that same verification recipe in the repository's CI image.

## Match the test shape to the risk

Prefer a test at the narrowest stable public seam that can observe the regression:

- Unit tests isolate algorithms and state transitions inside one crate.
- Integration tests exercise crate boundaries, generated contracts, or a prepared formula with
  caller-supplied inputs.
- Compile-pass and compile-fail tests protect the builtin declaration macro's Rust-facing contract.
- Golden fixtures make structured diagnostics, formatted formulas, and row results reviewable as
  text.
- WASM and frontend tests cover conversions and interaction behavior that native Rust tests cannot
  observe.

Add a focused boundary case when a broad baseline would only prove that the happy path still works.
Conversely, do not duplicate the same assertion at every layer. Keep one test responsible for the
behavior, then add higher-level coverage only when wiring or representation conversion is itself at
risk.

Tests live beside their owning crate and may move as the implementation changes. Find the relevant
runner from the package named in the table rather than treating a copied directory inventory as a
contract.

## Update golden expectations deliberately

Golden tests compare a readable input with a checked-in `.snap` result. A changed snapshot is a
behavior change or a changed representation, not routine generated output. Read the diff before
accepting it and confirm that unrelated cases did not move.

Use the repository recipes for Analyzer diagnostics and IDE formatting:

```bash
just test-analyzer-bless
just test-ide-bless
```

Evaluator builtin fixtures use their focused runner:

```bash
BLESS=1 cargo test -p evaluator --test builtin_golden
```

After blessing, rerun the same test without `BLESS=1`. Commit each changed input and snapshot
together. Do not bless a failure merely to make the suite green; first determine which contract or
implementation change explains the new output.

## Widen verification before review

Before handing off a change:

1. Run the focused command for every boundary the change touches.
2. Run `just docs-check` when Markdown, documentation tooling, or local links changed.
3. Run `just check` when Rust, WASM, or frontend source changed. On a clean checkout, run `just
   deps` first, or use `just verify`, which includes dependency preparation.
4. Run `just test` for a cross-crate or cross-language behavior change; use `just verify` when the
   complete repository workflow is warranted.

Record the commands and results in the PR. Long-lived documentation should explain the testing
strategy; the PR should preserve what was actually run for that change.

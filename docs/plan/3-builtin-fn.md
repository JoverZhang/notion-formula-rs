# Builtin Function Signatures: `builtin_fn` Design Plan

> **Status: Historical.** This file records the migration plan that proposed a runtime parser
> as the production authoring path. The parser interface and its tests remain, while the shipped
> production catalog is authored through the category procedural macro in `builtin_fn_macros`;
> normative Current contracts live in
> [`docs/design/builtin-fn.md`](../design/builtin-fn.md) and
> [`builtin_fn/README.md`](../../builtin_fn/README.md). The unchecked tasks below preserve the
> plan as written and are not a current implementation backlog.

This document describes a staged plan for extracting builtin function signature
infrastructure from `analyzer` into a new `builtin_fn` crate, replacing the
current macro-heavy handwritten definitions with a string-driven signature
parser, and preserving all existing analyzer behaviour.

The initial scope follows **Option A**:

- keep the current inference model based on `GenericParamKind::{Plain, Variant}`
- keep `SigResolver` as the escape hatch for signatures such as `flat`
- introduce a runtime signature parser rather than a procedural macro
- make extensible generic-kind registration part of the parser API

This plan is intentionally test-first. The main delivery is not just a new
crate, but a migration that is provably behaviour-preserving.

## Table of Contents

1. [Goals and Non-Goals](#1-goals-and-non-goals)
2. [Current State](#2-current-state)
3. [Target Architecture](#3-target-architecture)
4. [Public API of `builtin_fn`](#4-public-api-of-builtin_fn)
5. [Signature Language](#5-signature-language)
6. [Parser Design](#6-parser-design)
7. [Generic Kind Registration](#7-generic-kind-registration)
8. [Builtin Definition Model](#8-builtin-definition-model)
9. [Migration Plan](#9-migration-plan)
10. [Testing Strategy](#10-testing-strategy)
11. [Risks and Open Questions](#11-risks-and-open-questions)
12. [Implementation Checklist](#12-implementation-checklist)

---

## 1. Goals and Non-Goals

### Goals

1. Introduce a new leaf crate named `builtin_fn`.
2. Move builtin-signature core types out of `analyzer` into `builtin_fn`.
3. Define builtin signatures from spec-like strings rather than handwritten
   macro DSL calls.
4. Preserve existing analyzer inference, validation, completion, and
   signature-help behaviour.
5. Preserve the current `SigResolver` model for special cases.
6. Support extensible generic-kind names through parser registration.
7. Make spec drift easier to detect by keeping implementation syntax close to
   `docs/builtin_functions/README.md`.

### Non-Goals

1. Do not redesign the generic inference engine in this phase.
2. Do not replace `SigResolver` with a trait-based inference pipeline yet.
3. Do not change user-facing formula syntax.
4. Do not change the canonical builtin spec in
   `docs/builtin_functions/README.md` beyond sync updates required by the
   migration.
5. Do not implement unsupported semantic features such as `DateRange`, rich
   text types, or fully-typed `lets`.

---

## 2. Current State

Today, builtin functions are defined in `analyzer/src/analysis/builtins/` via a
small macro DSL:

- `func!`, `func_g!`, `func_gr!`
- `params!`, `repeat_params!`, `repeat_params_with_tail!`
- `p!`, `opt!`, `g!`, `generics!`

This works, but it has several drawbacks:

1. The implementation is more verbose than the spec.
2. The shape of the code does not match the shape of
   `docs/builtin_functions/README.md`.
3. Adding a new signature requires manual construction of `Ty`, `ParamShape`,
   and generic declarations even for simple cases.
4. The current generic-kind vocabulary is encoded in the `g!` macro, not in a
   reusable parsing or registration layer.
5. The handwritten representation makes large-scale review harder than it needs
   to be.

The existing spec-sync test in `analyzer/tests/builtin_spec_sync.rs` already
proves an important contract: the docs are the source of truth for which
signatures exist, their categories, and their display details.

The new design should strengthen that contract, not weaken it.

---

## 3. Target Architecture

### Dependency Graph

```text
builtin_fn  <-  analyzer  <-  ide
            <-  evaluator
            <-  analyzer_wasm (through analyzer)
```

`builtin_fn` becomes the owner of:

- `Ty`
- `GenericId`
- `LambdaParam`
- `FunctionCategory`
- `GenericParamKind`
- `GenericParam`
- `ParamSig`
- `ParamShape`
- `FunctionSig`
- `SigResolver`
- `normalize_union`
- builtin signature parsing
- builtin signature registration

`analyzer` keeps ownership of:

- lexer
- parser
- AST
- inference algorithm
- diagnostics
- semantic validation
- semantic `Context`

`analyzer` will re-export the moved semantic types from `builtin_fn` so that the
rest of the workspace can migrate with minimal churn.

### High-Level Layout

```text
builtin_fn/
  src/
    lib.rs
    types.rs
    signature.rs
    type_hints.rs
    param_shape.rs
    parser/
      mod.rs
      lexer.rs
      grammar.rs
      error.rs
    builtins/
      mod.rs
      general.rs
      text.rs
      math.rs
      date.rs
      people.rs
      list.rs
      special.rs
```

---

## 4. Public API of `builtin_fn`

The crate should expose a small, stable API surface.

```rust
pub use crate::types::{
    FunctionCategory, GenericId, LambdaParam, Ty,
};
pub use crate::signature::{
    FunctionSig, GenericParam, GenericParamKind, ParamShape, ParamSig, SigResolver,
};
pub use crate::type_hints::normalize_union;
pub use crate::parser::{
    BuiltinSigParser, BuiltinSigParseError, GenericKindRegistry,
};

pub fn builtins_functions() -> Vec<FunctionSig>;
pub fn default_parser() -> BuiltinSigParser;
```

### Parser-Facing API

```rust
pub struct GenericKindRegistry { ... }

impl GenericKindRegistry {
    pub fn new() -> Self;
    pub fn with_builtin_kinds() -> Self;
    pub fn register(&mut self, name: impl Into<String>, kind: GenericParamKind);
    pub fn resolve(&self, name: &str) -> Option<GenericParamKind>;
}

pub struct BuiltinSigParser {
    registry: GenericKindRegistry,
}

impl BuiltinSigParser {
    pub fn new(registry: GenericKindRegistry) -> Self;
    pub fn parse(&self, category: FunctionCategory, text: &str)
        -> Result<FunctionSig, BuiltinSigParseError>;
}
```

This keeps the first version simple:

- custom kind names can be registered
- registered names still map to the existing `GenericParamKind` enum
- resolver attachment remains a separate step

---

## 5. Signature Language

The parser should accept a compact language deliberately aligned with the doc
spec.

### Supported Forms

```text
pi() -> number
abs(value: number) -> number
round(value: number, places?: number) -> number
length<T: Plain>(value: string | T[]) -> number
if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T
map<T: Plain, U: Plain>(list: T[], mapper: (current: T) -> U) -> U[]
let<T: Plain, U: Plain>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U
ifs<T: Variant>(condition1: boolean, value1: () -> T, ..., else: () -> T) -> T
splice<T: Plain>(list: T[], startIndex: number, deleteCount: number, ...items: T[]) -> T[]
```

### Type Vocabulary

The first version supports all types already representable by the analyzer:

- primitives: `number`, `string`, `boolean`, `date`, `null`, `any`
- generic references: `T`, `U`
- list types: `T[]`, `number[]`, `(number | string)[]`
- unions: `A | B`
- lambdas: `() -> T`, `(current: T) -> U`
- identifier binders: `Ident<T>`

### Parameter Shape Vocabulary

The parser must recover the existing `ParamShape` model:

- fixed head parameters
- optional parameters via `name?: type`
- repeat groups via `...,`
- rest args via `...items: T[]`
- tail parameters after a repeat group

### Important Constraint

The parser does not need to invent new semantics. It only needs to build the
same `FunctionSig` values that are currently handwritten.

---

## 6. Parser Design

### Overview

The parser should be a small dedicated parser, not a collection of regexes.

Recommended stages:

1. Tokenise the signature string.
2. Parse into a small internal AST.
3. Lower the AST into `FunctionSig`.
4. Validate the generated signature through existing `FunctionSig::new_builtin`
   invariants.

### Internal AST

The parser can use a crate-private representation such as:

```rust
struct ParsedSig {
    name: String,
    generics: Vec<ParsedGeneric>,
    params: ParsedParamList,
    ret: ParsedTy,
}

struct ParsedGeneric {
    name: String,
    kind_name: Option<String>,
}

struct ParsedParamList {
    items: Vec<ParsedParamItem>,
}

enum ParsedParamItem {
    Param(ParsedParam),
    Ellipsis,
}
```

This keeps parsing concerns separate from semantic lowering concerns.

### Lowering Rules

Lowering should follow deterministic rules:

1. Generic names are assigned `GenericId` values by declaration order.
2. Unqualified generic kinds default to `Plain`.
3. `any` lowers to a fresh generic only when the handwritten implementation used
   a generic before; otherwise it remains a concrete `Ty` choice if the type
   model grows later.
4. `detail` is always generated canonically as `name(arg1, arg2, ...)`.
5. Repeat-group detection is driven by the `...,` marker, matching the doc test.

### Error Reporting

The parser should return structured errors with enough context to debug invalid
signature strings in builtin definitions. At minimum:

- unknown generic kind name
- unexpected token
- missing `->`
- malformed parameter
- malformed type
- invalid repeat-group placement
- duplicate generic name
- unknown generic reference

Friendly error quality matters because the new system moves complexity from Rust
types into strings.

---

## 7. Generic Kind Registration

This phase does not replace `GenericParamKind`. It adds a registration layer on
top of it.

### Why Registration Exists

Today, the builtin declaration site knows the kind vocabulary statically:

- `Plain`
- `Variant`

The new parser should not hardcode that vocabulary in its grammar. Instead, it
should parse the identifier after `:` as a symbolic kind name and look it up in
the registry.

### Initial Model

```rust
let mut registry = GenericKindRegistry::new();
registry.register("Plain", GenericParamKind::Plain);
registry.register("Variant", GenericParamKind::Variant);
```

This already gives two benefits:

1. the parser is data-driven rather than keyword-driven
2. future aliases or experimental names can be introduced without changing the
   parser grammar

### About Future Names Such as `Flat`

In this phase, `Flat` should be treated as a parser-level alias only if needed.
It does **not** add new inference semantics by itself. If a builtin needs custom
behaviour, that behaviour still belongs in `SigResolver`.

That means there are two clean extension paths:

1. register a new name that maps to an existing enum variant, such as
   `registry.register("Flat", GenericParamKind::Plain)`
2. parse the signature normally and attach a resolver afterwards

This keeps the migration low-risk while leaving room for a later trait-based
design.

---

## 8. Builtin Definition Model

Each builtin module in `builtin_fn/src/builtins/` should define signatures with
small helper functions rather than a large macro DSL.

### Recommended Helper Layer

```rust
fn sig(parser: &BuiltinSigParser, category: FunctionCategory, text: &str) -> FunctionSig {
    parser.parse(category, text).unwrap()
}

fn sig_with_resolver(
    parser: &BuiltinSigParser,
    category: FunctionCategory,
    text: &str,
    resolver: SigResolver,
) -> FunctionSig {
    let mut sig = parser.parse(category, text).unwrap();
    sig.resolver = Some(resolver);
    sig
}
```

### Example

```rust
pub(super) fn builtins(parser: &BuiltinSigParser) -> Vec<FunctionSig> {
    vec![
        sig(parser, FunctionCategory::General,
            "if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T"),
        sig(parser, FunctionCategory::General,
            "empty<T: Plain>(value?: T) -> boolean"),
    ]
}
```

For `flat`:

```rust
sig_with_resolver(
    parser,
    FunctionCategory::List,
    "flat<T: Plain>(list: T[]) -> T[]",
    resolve_flat,
)
```

### Why This Is Better Than Replacing Everything With One Macro

1. the string matches the spec closely
2. the parser remains independently testable
3. resolver wiring stays explicit
4. debugging stays simple because the construction path is ordinary Rust

---

## 9. Migration Plan

The migration should be incremental and continuously testable.

### Phase 1: Create the Crate

1. Add `builtin_fn` to the workspace.
2. Add `builtin_fn` as a dependency of `analyzer`.
3. Create minimal `lib.rs` exports.

### Phase 2: Move Core Types

Move the following code from `analyzer/src/analysis/` into `builtin_fn`:

- `Ty`, `GenericId`, `LambdaParam`, `FunctionCategory`
- `FunctionSig`, `ParamSig`, `ParamShape`, `GenericParam`,
  `GenericParamKind`, `SigResolver`
- `normalize_union`
- repeat-tail helper logic used by `ParamShape`

Then re-export them from `analyzer::semantic` to avoid a large immediate API
break.

### Phase 3: Introduce the Parser

1. Implement `GenericKindRegistry`.
2. Implement `BuiltinSigParser`.
3. Add parser-only unit tests.
4. Add equivalence tests against selected handwritten signatures.

At this point, builtin definitions in production code still remain handwritten.

### Phase 4: Migrate Simple Builtins First

Migration order should minimise risk:

1. `math`
2. `date`
3. `people`
4. `special`
5. `text`
6. `general`
7. `list`

This order goes from lowest semantic complexity to highest.

### Phase 5: Replace `analyzer` Builtin Source

Once all categories live in `builtin_fn`, update `analyzer` so that
`builtins_functions()` delegates to `builtin_fn::builtins_functions()`.

### Phase 6: Delete Legacy Macro DSL

After full migration and green tests:

- delete `analyzer/src/analysis/builtins/macros.rs`
- delete legacy handwritten builtin modules from `analyzer`
- simplify `analyzer/src/analysis/builtins/mod.rs`

---

## 10. Testing Strategy

This migration should be judged primarily by test quality.

### 10.1 Test Layers

The full suite should contain five layers:

1. parser unit tests in `builtin_fn`
2. signature-equivalence tests in `builtin_fn`
3. builtin-registry tests in `builtin_fn`
4. existing spec-sync tests in `analyzer`
5. existing analyzer semantic/inference tests across the workspace

### 10.2 Parser Unit Tests

Parser unit tests should cover every grammar feature independently.

#### Basic signatures

- `pi() -> number`
- `abs(value: number) -> number`
- `add(a: number, b: number) -> number`

#### Optional parameters

- `round(value: number, places?: number) -> number`
- `empty<T: Plain>(value?: T) -> boolean`

#### Generic declarations

- `format<T: Plain>(value: T) -> string`
- `if<T: Variant>(condition: boolean, then: () -> T, else: () -> T) -> T`
- `map<T: Plain, U: Plain>(list: T[], mapper: (current: T) -> U) -> U[]`

#### Lists and unions

- `string[]`
- `number[][]`
- `string | number`
- `string | T[]`
- `(number | string)[]`

#### Lambda types

- `() -> T`
- `(current: T) -> boolean`
- `(ident: T) -> U`

#### Identifier binders

- `Ident<T>` in parameter position
- `let<T: Plain, U: Plain>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U`

#### Repeat and variadic shapes

- `ifs<T: Variant>(condition1: boolean, value1: () -> T, ..., else: () -> T) -> T`
- `splice<T: Plain>(list: T[], startIndex: number, deleteCount: number, ...items: T[]) -> T[]`

#### Registry-driven generic kinds

- register `Plain`, parse successfully
- register `Variant`, parse successfully
- register alias `Flat -> Plain`, parse successfully
- fail on unknown kind name

#### Invalid input

- missing arrow
- missing closing paren
- duplicate generic name
- unknown generic reference in type position
- invalid ellipsis placement
- malformed optional parameter syntax

### 10.3 Signature Equivalence Tests

These are the most important migration tests.

For a representative sample of signatures, construct the legacy handwritten
`FunctionSig` and compare it to the parsed result with `assert_eq!`.

Required cases:

- `if`
- `ifs`
- `empty`
- `length`
- `let`
- `substring`
- `concat`
- `round`
- `min`
- `at`
- `splice`
- `map`
- `filter`
- `flat` without resolver

Once confidence is high, expand this into a category-wide equivalence test for
every builtin that exists today.

### 10.4 Builtin Registry Tests

`builtin_fn::builtins_functions()` should have its own tests:

- no duplicate builtin names
- category order is deterministic
- resolver is attached only where expected
- all declared builtin signatures validate successfully

### 10.5 Analyzer Compatibility Tests

Keep the following tests green throughout the migration:

- `analyzer/tests/builtin_spec_sync.rs`
- `analyzer/src/tests/analysis/test_semantic_infer_builtins.rs`
- `analyzer/src/tests/analysis/test_generic_infer.rs`
- `analyzer/src/tests/analysis/test_sig_resolver.rs`
- all parser and diagnostics tests that transitively rely on semantic types

### 10.6 Suggested Test Execution Order

During implementation, use this loop:

1. `cargo test -p builtin_fn`
2. `cargo test -p analyzer builtin_spec_sync`
3. `cargo test -p analyzer`
4. `cargo test`

This keeps failures local first, then verifies workspace integration.

---

## 11. Risks and Open Questions

### Risk 1: String Parser Bugs Hide Type Construction Errors

Mitigation:

- keep parser unit tests granular
- add equivalence tests against handwritten signatures
- continue using `FunctionSig::new_builtin` validation

### Risk 2: Detail String Drift

The spec-sync test compares `sig.detail` exactly.

Mitigation:

- generate `detail` canonically in one place only
- add dedicated parser tests for detail rendering

### Risk 3: Ambiguity Between Repeat Groups and Rest Args

Mitigation:

- document a narrow grammar
- add focused tests for `ifs` and `splice`
- reject ambiguous or unsupported forms rather than guessing

### Risk 4: Moving Types Breaks Downstream Imports

Mitigation:

- re-export from `analyzer::semantic`
- migrate internal imports first
- defer removal of analyzer-local aliases until all workspace crates are green

### Open Question: Should `any` Parse to a Concrete `Ty` Variant?

The docs use `any` as a readability device, while the implementation often uses a
generic `T: Plain` instead.

For this migration, the parser should support whichever lowering is required to
reproduce existing behaviour exactly. That means some signature strings in code
may intentionally use generics instead of the literal `any` spelling from the
docs.

Behavioural compatibility is more important than textual purity in phase one.

---

## 12. Implementation Checklist

### Phase 1: Workspace and Crate Setup

- [ ] Add `builtin_fn` to the workspace in `Cargo.toml`
- [ ] Create `builtin_fn/Cargo.toml`
- [ ] Create `builtin_fn/src/lib.rs`
- [ ] Add `builtin_fn` as a dependency of `analyzer`
- [ ] Verify `cargo check` still passes

### Phase 2: Type Extraction

- [ ] Move semantic type definitions from `analyzer` into `builtin_fn`
- [ ] Move signature model definitions into `builtin_fn`
- [ ] Move `normalize_union` into `builtin_fn`
- [ ] Move repeat-tail helper logic into `builtin_fn`
- [ ] Re-export moved types from `analyzer::semantic`
- [ ] Update analyzer imports to use the new source of truth
- [ ] Verify `cargo test -p analyzer` still passes before parser work begins

### Phase 3: Parser Foundation

- [ ] Implement parser token model
- [ ] Implement parser error model
- [ ] Implement parsed-signature internal AST
- [ ] Implement type parsing
- [ ] Implement generic declaration parsing
- [ ] Implement parameter parsing
- [ ] Implement repeat-group and variadic parsing
- [ ] Implement canonical detail generation
- [ ] Implement lowering to `FunctionSig`
- [ ] Add parser unit tests for each grammar feature

### Phase 4: Generic Kind Registration

- [ ] Implement `GenericKindRegistry`
- [ ] Pre-register `Plain` and `Variant`
- [ ] Support custom parser-level aliases
- [ ] Add tests for registered and unregistered kind names

### Phase 5: Equivalence Harness

- [ ] Add handwritten-vs-parsed equivalence tests for representative signatures
- [ ] Expand equivalence tests to all builtin categories
- [ ] Add deterministic-order tests for `builtin_fn::builtins_functions()`
- [ ] Add duplicate-name tests for `builtin_fn::builtins_functions()`

### Phase 6: Migrate Builtin Definitions

- [ ] Migrate `math`
- [ ] Migrate `date`
- [ ] Migrate `people`
- [ ] Migrate `special`
- [ ] Migrate `text`
- [ ] Migrate `general`
- [ ] Migrate `list`
- [ ] Re-attach `resolve_flat` via `SigResolver`
- [ ] Keep `builtin_spec_sync` passing after each migrated category

### Phase 7: Analyzer Integration

- [ ] Replace analyzer builtin registry construction with `builtin_fn::builtins_functions()`
- [ ] Remove legacy builtin macro DSL from `analyzer`
- [ ] Remove legacy handwritten builtin modules from `analyzer`
- [ ] Update `analyzer/README.md` to describe the new ownership boundary

### Phase 8: Final Verification

- [ ] Run `cargo test -p builtin_fn`
- [ ] Run `cargo test -p analyzer`
- [ ] Run `cargo test`
- [ ] Review `docs/builtin_functions/README.md` for any terminology drift
- [ ] Confirm no public API regressions for downstream crates

---

## Recommended First Slice

The safest first implementation slice is:

1. create `builtin_fn`
2. move the core types only
3. implement the parser with tests
4. migrate the `math` category first
5. prove equivalence with existing `math` signatures

If that slice stays green, the rest of the migration is mostly repetition plus
special-case coverage for `general` and `list`.

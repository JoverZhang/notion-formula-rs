# Implicit Lambdas: Design Plan

This document captures the design for adding implicit lambda (thunk) support to the
analyzer. Functions like `if`, `ifs`, `let`, `lets`, `and`, `or`, `map`, `filter`, etc.
have arguments that must not be eagerly evaluated. Rather than requiring users to write
arrow syntax, the analyzer wraps designated argument positions into lambda AST nodes
automatically during type inference.

**Prerequisite for**: [Evaluator Builtins](./evaluator-builtins.md) control-flow and
list-builtin sections.

## Table of Contents

1. [Motivation and Examples](#1-motivation-and-examples)
2. [Type System Changes](#2-type-system-changes)
3. [AST Changes](#3-ast-changes)
4. [Inference Pass Changes](#4-inference-pass-changes)
5. [Signature Updates](#5-signature-updates)
6. [Evaluator / IR / Runtime](#6-evaluator--ir--runtime)
7. [Implementation Checklist](#7-implementation-checklist)

---

## 1. Motivation and Examples

### The Problem

Notion formulas contain functions whose arguments must not be eagerly evaluated:

| Category | Functions | Why lazy? |
|----------|-----------|-----------|
| Branching | `if`, `ifs` | Only the taken branch should execute |
| Short-circuit | `and`, `or` | Stop evaluating after a falsy/truthy operand |
| Binders | `let`, `lets` | The body expression references a name that does not exist until runtime |
| List transforms | `map`, `filter`, `find`, `findIndex`, `some`, `every`, `count` | The mapper/predicate runs once per element with `current` bound |

Users never write lambda/arrow syntax. The formula `if(true, 1, "2")` looks like three
ordinary arguments, but `1` and `"2"` must be wrapped in nullary thunks (`() -> T`) so
the evaluator can choose which to execute.

### User-Facing Syntax (unchanged)

```
# Branching — args 2,3 are nullary thunks
if(true, 1, "2")

# Multi-branch — each (condition, value) pair and the else are thunks
ifs(x > 0, "positive", x == 0, "zero", "negative")

# Binder — arg 1 is a bare identifier, arg 3 is a lambda (a) -> T
let(a, 123, "hello" + format(a))

# Nested binders
lets(a, 1, b, a + 2, a + b)

# List transform — arg 2 is a lambda (current) -> T
[1, 2, 3].map(current + 1)

# Chained
[1, 2, 3].filter(current > 1).map(current * 10)
```

None of these examples contain arrow tokens. The analyzer sees ordinary expressions and
implicitly wraps them based on the function signature's type annotations.

### Current State

- **`Ty` enum** (`analyzer/src/analysis/mod.rs:81-91`): No function/lambda type.
- **`ExprKind` enum** (`analyzer/src/parser/ast.rs:175-208`): No lambda variant.
- **`infer_expr_inner`** (`analyzer/src/analysis/infer.rs:181`): No scope mechanism — all
  `ExprKind::Ident` resolve to `Ty::Unknown`.
- **Builtin signatures** (`analyzer/src/analysis/builtins/general.rs:75-76`,
  `list.rs:138-146`): `let`/`lets` and all lambda-taking list builtins are TODO-commented.
- **Lexer** (`analyzer/src/lexer/token.rs`): No arrow token. This feature adds no new
  tokens — the wrapping is purely analyzer-internal.

---

## 2. Type System Changes

### 2.1 New `Ty` Variants

Two new variants are added to the `Ty` enum in `analyzer/src/analysis/mod.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,
    Generic(GenericId),
    List(Box<Ty>),
    Union(Vec<Ty>),
    // ── NEW ──────────────────────────────────────────
    /// A function type representing an implicit lambda.
    ///
    /// `params` describes the bindings the lambda introduces (empty for nullary thunks).
    /// `ret` is the return type of the lambda body.
    Fn {
        params: Vec<(LambdaParam, Ty)>,
        ret: Box<Ty>,
    },
    /// A bare-identifier parameter position (used by `let`/`lets`).
    ///
    /// The inner `Ty` is the type of the value that will be bound to this identifier.
    /// During inference the analyzer reads the identifier's text from the AST and uses it
    /// as the binding name in the lambda scope.
    Ident(Box<Ty>),
}
```

### 2.2 `LambdaParam` Enum

Describes how a lambda parameter is sourced. Lives alongside `Ty`:

```rust
/// Describes the origin of an implicit lambda parameter binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LambdaParam {
    /// The implicit `current` binding injected by list builtins (`map`, `filter`, etc.).
    Current,
    /// A binding whose name comes from the value of another argument position.
    ///
    /// The string is the `name` field of the `ParamSig` whose runtime value supplies the
    /// identifier text. For example, in `let(ident, value, body)` the body's lambda param
    /// is `ParamRef("ident")` — the analyzer reads the bare identifier from the `ident`
    /// argument to determine the binding name.
    ParamRef(std::string::String),
}
```

### 2.3 How the Three Lambda Families Map to Types

| Family | Signature sketch | `Ty::Fn` params | Example |
|--------|-----------------|-----------------|---------|
| **Nullary thunk** | `if<T>(cond: Boolean, then: Fn([] -> T), else: Fn([] -> T)) -> T` | `vec![]` | `if(true, 1, "2")` — `1` becomes `ImplicitLambda { params: [], body: 1 }` |
| **Binder** | `let<T0,T1>(ident: Ident<T0>, value: T0, body: Fn([(ParamRef("ident"), T0)] -> T1)) -> T1` | `vec![(ParamRef("ident"), T0)]` | `let(a, 123, a + 1)` — body becomes `ImplicitLambda { params: ["a"], body: a + 1 }` |
| **List transform** | `map<T,U>(list: T[], mapper: Fn([(Current, T)] -> U)) -> U[]` | `vec![(Current, T)]` | `[1,2,3].map(current + 1)` — mapper becomes `ImplicitLambda { params: ["current"], body: current + 1 }` |

### 2.4 Display, Precedence, and Serde

- **Display**: `Fn` renders as `(param: type, ...) -> ret` for non-empty params, `() -> ret`
  for thunks. `Ident` renders as `ident<inner>`.
- **Precedence**: `Fn` gets the lowest precedence (0) so it always parenthesises inside
  unions. `Ident` gets the same precedence as scalars (3).
- **Serde**: Both variants serialise naturally via the existing `#[serde(rename_all = "PascalCase")]`.
  `LambdaParam` gets its own `Serialize`/`Deserialize`.

### 2.5 `ty_accepts` Rules

The acceptance function (`analyzer/src/analysis/mod.rs`) gains two new arms:

```rust
// A Fn-typed param accepts any expression — the inference pass handles wrapping.
// During validation the wrapped ImplicitLambda's body type is checked against ret.
(Ty::Fn { ret: expected_ret, .. }, actual) => ty_accepts(expected_ret, actual),

// Ident-typed positions are internal annotations — accept any type.
// validate_call short-circuits before reaching ty_accepts for Ident params,
// but IDE callers may hit this path.
(Ty::Ident(_), _) => true,
```

---

## 3. AST Changes

### 3.1 New `ExprKind::ImplicitLambda`

A single new variant is added to `ExprKind` in `analyzer/src/parser/ast.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Ident(Symbol),
    Group { inner: Box<Expr> },
    List { items: Vec<Expr> },
    Call { callee: Symbol, args: Vec<Expr> },
    MemberCall { receiver: Box<Expr>, method: Symbol, args: Vec<Expr> },
    Lit(Lit),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Ternary { cond: Box<Expr>, then: Box<Expr>, otherwise: Box<Expr> },
    // ── NEW ──────────────────────────────────────────
    /// An implicit lambda inserted by the inference pass.
    ///
    /// The parser never produces this variant. It is created in-place by `infer_call`
    /// when a call argument occupies a `Ty::Fn`-typed parameter position.
    ///
    /// `params` contains the resolved binding names (e.g. `["current"]` for list builtins,
    /// `["a"]` for `let(a, ...)`). Empty for nullary thunks.
    /// `body` is the original expression that was at this argument position.
    ImplicitLambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    Error,
}
```

### 3.2 Key Design Decisions

1. **Not produced by the parser** — The parser has no knowledge of lambdas. The
   `ImplicitLambda` node is inserted during inference, which requires type information
   to know which argument positions need wrapping.

2. **`params` are resolved names, not `LambdaParam`** — The `LambdaParam::Current` and
   `LambdaParam::ParamRef(...)` variants live only in `Ty::Fn`. The inference pass
   resolves them to concrete `String` names before constructing the AST node:
   - `LambdaParam::Current` → `"current"`
   - `LambdaParam::ParamRef("ident")` → reads the bare identifier text from the
     referenced argument position (e.g. `"a"` from `let(a, ...)`)

3. **In-place mutation** — The inference pass takes `&mut Expr` and replaces the argument
   expression with an `ImplicitLambda` wrapping the original. This avoids a separate
   rewrite pass and keeps type information available during the transformation.

4. **`body` retains its original `ExprId`** — The wrapping `ImplicitLambda` node gets a
   fresh `ExprId`. The inner body expression keeps its original id and span, so
   diagnostics and IDE tooling can still reference the user's source positions.

### 3.3 Example AST Transformation

**Input**: `if(x > 0, "yes", "no")`

**Before inference** (parser output):
```
Call {
  callee: "if",
  args: [
    Binary { op: Gt, left: Ident("x"), right: Lit(0) },
    Lit("yes"),
    Lit("no"),
  ]
}
```

**After inference** (mutated in-place):
```
Call {
  callee: "if",
  args: [
    Binary { op: Gt, left: Ident("x"), right: Lit(0) },   // unchanged
    ImplicitLambda { params: [], body: Lit("yes") },       // nullary thunk
    ImplicitLambda { params: [], body: Lit("no") },        // nullary thunk
  ]
}
```

**Input**: `let(a, 123, a + 1)`

**After inference**:
```
Call {
  callee: "let",
  args: [
    Ident("a"),                                            // Ident<Number> — bare identifier
    Lit(123),                                              // Number — the value
    ImplicitLambda { params: ["a"], body: Binary(+, Ident("a"), Lit(1)) },
  ]
}
```

**Input**: `[1,2,3].map(current + 1)`

**After inference** (desugared to prefix form `map([1,2,3], current + 1)`):
```
Call {
  callee: "map",
  args: [
    List { items: [Lit(1), Lit(2), Lit(3)] },              // T[] — the list
    ImplicitLambda { params: ["current"], body: Binary(+, Ident("current"), Lit(1)) },
  ]
}
```

---

## 4. Inference Pass Changes

This is the core of the feature. The inference pass (`analyzer/src/analysis/infer.rs`)
changes from a read-only traversal to a mutating one that can wrap arguments in
`ImplicitLambda` nodes.

### 4.1 Signature Change: `&Expr` → `&mut Expr`

The public entry point and all recursive helpers change to take mutable references:

```rust
// Before:
pub fn infer_expr_with_map(expr: &Expr, ctx: &Context, map: &mut TypeMap) -> Ty

// After:
pub fn infer_expr_with_map(expr: &mut Expr, ctx: &Context, map: &mut TypeMap) -> Ty
```

This propagates through `infer_expr_inner`, `infer_call`, `infer_prop`, and all match
arms that recurse into child expressions.

**Impact**: All call sites in `analyze_expr` (`analyzer/src/analysis/mod.rs`) and
validation must pass `&mut expr` instead of `&expr`. The parser's output is consumed
mutably by analysis.

### 4.2 Scope Stack

A scope stack is added to the inference state to resolve identifiers introduced by
implicit lambdas:

```rust
/// Lexical scope stack for implicit lambda bindings.
///
/// Each frame is a map from binding name to its type. Frames are pushed when
/// entering a lambda body and popped when leaving.
struct InferState {
    scopes: Vec<HashMap<String, Ty>>,
}

impl InferState {
    fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    fn push_scope(&mut self, bindings: HashMap<String, Ty>) {
        self.scopes.push(bindings);
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Look up a name in the scope stack (innermost first).
    fn resolve(&self, name: &str) -> Option<&Ty> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }
}
```

The `InferState` is threaded through all inference functions (or stored in a context
struct). The key integration point is the `ExprKind::Ident` arm:

```rust
// Before:
ExprKind::Ident(_) => Ty::Unknown,

// After:
ExprKind::Ident(sym) => {
    state.resolve(&sym.text)
        .cloned()
        .unwrap_or(Ty::Unknown)
}
```

This is the only change needed for identifier resolution — `current`, `a`, etc. all
resolve through the same scope mechanism.

### 4.3 Enhanced `infer_call`

The `infer_call` function is the heart of the lambda wrapping logic. The new flow:

```rust
fn infer_call(
    name: &str,
    sig: Option<&FunctionSig>,
    args: &mut [Expr],           // now &mut
    ctx: &Context,
    map: &mut TypeMap,
    state: &mut InferState,      // new
) -> Ty {
    let Some(sig) = sig else {
        for arg in args.iter_mut() {
            let _ = infer_expr_with_map(arg, ctx, map, state);
        }
        return Ty::Unknown;
    };

    // Phase 1: Infer non-lambda arguments first to populate generic substitutions.
    //
    // This is necessary because lambda body types may depend on generics bound by
    // earlier arguments. For example, in `let(a, 123, a + 1)`:
    //   - arg 0 ("a") is Ident<T0> — does not bind T0
    //   - arg 1 (123) is T0 — binds T0 = Number
    //   - arg 2 is Fn([(ParamRef("ident"), T0)] -> T1) — needs T0 = Number to type the body
    //
    // Non-lambda, non-ident arguments are inferred eagerly. Ident arguments are skipped
    // (they contribute no type information beyond Unknown). Lambda arguments are deferred.

    let resolved_params = sig.shape.resolve_params(args.len());
    let mut subst = Subst::new();
    let mut arg_tys = vec![Ty::Unknown; args.len()];

    // First pass: infer non-lambda, non-ident args and build substitution.
    for (i, arg) in args.iter_mut().enumerate() {
        let Some(param) = resolved_params.get(i) else { continue };
        match &param.ty {
            Ty::Fn { .. } => continue,   // defer
            Ty::Ident(_) => continue,    // skip — bare ident, no type info
            _ => {
                arg_tys[i] = infer_expr_with_map(arg, ctx, map, state);
                unify_single(param, &arg_tys[i], &mut subst);
            }
        }
    }

    // Second pass: process Ident and Fn-typed arguments with substitutions available.
    for (i, arg) in args.iter_mut().enumerate() {
        let Some(param) = resolved_params.get(i) else { continue };
        match &param.ty {
            Ty::Ident(inner_ty) => {
                // Record the identifier's bound type (after substitution) for
                // downstream ParamRef resolution, but do not infer the arg itself.
                let bound_ty = apply(&subst, inner_ty);
                arg_tys[i] = Ty::Ident(Box::new(bound_ty));
            }
            Ty::Fn { params: fn_params, ret } => {
                // Resolve lambda parameter names and types.
                let mut bindings = HashMap::new();
                let mut param_names = Vec::new();

                for (lp, lp_ty) in fn_params {
                    let (name, ty) = match lp {
                        LambdaParam::Current => {
                            ("current".to_string(), apply(&subst, lp_ty))
                        }
                        LambdaParam::ParamRef(ref_name) => {
                            // Find the argument at the position named `ref_name` and
                            // extract the bare identifier text.
                            let ident_text = resolve_param_ref(
                                ref_name, &resolved_params, args
                            );
                            let ty = apply(&subst, lp_ty);
                            (ident_text, ty)
                        }
                    };
                    bindings.insert(name.clone(), ty);
                    param_names.push(name);
                }

                // Push scope, infer body, pop scope.
                state.push_scope(bindings);
                let body_ty = infer_expr_with_map(arg, ctx, map, state);
                state.pop_scope();

                // Wrap the argument in-place.
                let original = std::mem::replace(&mut arg.kind, ExprKind::Error);
                arg.kind = ExprKind::ImplicitLambda {
                    params: param_names,
                    body: Box::new(Expr {
                        id: arg.id,     // keep original id for diagnostics
                        span: arg.span,
                        kind: original,
                    }),
                };
                // Assign a fresh ExprId to the wrapper node.
                arg.id = ExprId::next();

                arg_tys[i] = body_ty;
                let expected_ret = apply(&subst, ret);
                // Unify body type with expected return type.
                unify_ty(&expected_ret, &arg_tys[i], &mut subst);
            }
            _ => {} // already inferred in first pass
        }
    }

    if let Some(resolver) = sig.resolver {
        let resolved = resolver(sig, &arg_tys);
        return resolved.ret;
    }

    apply(&subst, &sig.ret)
}
```

### 4.4 `resolve_param_ref` Helper

Resolves a `LambdaParam::ParamRef` by finding the referenced argument and extracting its
bare identifier text:

```rust
/// Given a param name (e.g. "ident"), find the argument at that position and extract
/// the bare identifier text from it.
///
/// Returns the identifier string, or a fallback `"_"` if the argument is not a bare
/// identifier (which would be a validation error caught elsewhere).
fn resolve_param_ref(
    ref_name: &str,
    resolved_params: &[&ParamSig],
    args: &[Expr],
) -> String {
    let pos = resolved_params.iter().position(|p| p.name == ref_name);
    match pos {
        Some(i) if i < args.len() => match &args[i].kind {
            ExprKind::Ident(sym) => sym.text.clone(),
            _ => "_".to_string(), // fallback for malformed input
        },
        _ => "_".to_string(),
    }
}
```

### 4.5 `MemberCall` Handling

The existing `MemberCall` arm desugars to prefix form before calling `infer_call`. This
continues to work but the `all_args` construction must build `&mut` references. Since
member calls clone the receiver today (`(**receiver).clone()`), the lambda wrapping
applies to the cloned args vector, which is then discarded.

**Decision**: For the initial implementation, member-call lambda wrapping uses the same
clone-and-infer approach. The cloned `ImplicitLambda` nodes are sufficient for type
inference and `TypeMap` population. The planner (evaluator side) independently reconstructs
lambda structure from the original AST + type information.

> **Future optimisation**: If the planner consumes the mutated AST directly, member-call
> desugaring should be refactored to avoid the clone. This is deferred.

### 4.6 Two-Pass Ordering Rationale

The two-pass approach (non-lambda args first, then lambda args) is necessary because:

1. **Generic binding order matters**: In `let(a, 123, a + 1)`, the value argument `123`
   binds `T0 = Number`. The body lambda `(a: T0) -> T1` needs `T0` to be resolved before
   it can type-check `a + 1` as `Number + Number = Number`.

2. **List builtins similarly**: In `map(list, current + 1)`, the `list` argument binds
   `T = Number` (if it's `List<Number>`), and the mapper needs `current: T = Number`.

3. **Ident args contribute no bindings**: `Ident<T0>` at argument position cannot be
   inferred (it's a bare name) — `T0` gets bound by the value argument instead.

This matches the left-to-right evaluation intuition: value-producing arguments are
analysed before body-consuming arguments.

---

## 5. Signature Updates

This section shows the concrete signature changes for each function family. All
signatures use the new `Ty::Fn`, `Ty::Ident`, and `LambdaParam` types.

### 5.1 Branching: `if`

```rust
// if<T>(condition: Boolean, then: () -> T, else: () -> T) -> T
func_g!(
    FunctionCategory::General,
    "if(condition, then, else)",
    generics!(g!(0, Variant)),
    "if",
    params!(
        p!("condition", Ty::Boolean),
        p!("then", Ty::Fn { params: vec![], ret: Box::new(Ty::Generic(t0)) }),
        p!("else", Ty::Fn { params: vec![], ret: Box::new(Ty::Generic(t0)) })
    ),
    Ty::Generic(t0),
)
```

### 5.2 Branching: `ifs`

```rust
// ifs<T>(condition1: Boolean, value1: () -> T, ..., else: () -> T) -> T
func_g!(
    FunctionCategory::General,
    "ifs(condition1, value1, ..., else)",
    generics!(g!(0, Variant)),
    "ifs",
    repeat_params!(
        head!(),
        repeat!(
            p!("condition1", Ty::Boolean),
            p!("value1", Ty::Fn { params: vec![], ret: Box::new(Ty::Generic(t0)) })
        ),
        tail!(p!("else", Ty::Fn { params: vec![], ret: Box::new(Ty::Generic(t0)) })),
    ),
    Ty::Generic(t0),
)
```

**Note**: The `condition` arguments are *not* thunks. Only the `value` and `else`
arguments are wrapped. The condition is eagerly evaluated to determine which branch thunk
to execute.

### 5.3 Binders: `let`

```rust
// let<T0, T1>(ident: Ident<T0>, value: T0, body: (ident: T0) -> T1) -> T1
func_g!(
    FunctionCategory::General,
    "let(ident, value, body)",
    generics!(g!(0, Plain), g!(1, Plain)),
    "let",
    params!(
        p!("ident", Ty::Ident(Box::new(Ty::Generic(t0)))),
        p!("value", Ty::Generic(t0)),
        p!("body", Ty::Fn {
            params: vec![(LambdaParam::ParamRef("ident".into()), Ty::Generic(t0))],
            ret: Box::new(Ty::Generic(t1)),
        })
    ),
    Ty::Generic(t1),
)
```

### 5.4 Binders: `lets`

```rust
// lets<T0, T1>(ident1: Ident<T0>, value1: T0, ..., body: (ident1: T0, ...) -> T1) -> T1
//
// NOTE: `lets` is more complex because each binding can reference previous bindings.
// The repeat group introduces one (ident, value) pair per cycle, and the body lambda
// accumulates all bindings. This requires special handling in infer_call:
//
// For the initial implementation, `lets` can be desugared to nested `let` calls during
// inference or handled as a special case. The signature below models the simplified
// single-repeat form:
func_g!(
    FunctionCategory::General,
    "lets(ident1, value1, ..., body)",
    generics!(g!(0, Plain), g!(1, Plain)),
    "lets",
    repeat_params!(
        head!(),
        repeat!(
            p!("ident1", Ty::Ident(Box::new(Ty::Generic(t0)))),
            p!("value1", Ty::Generic(t0))
        ),
        tail!(p!("body", Ty::Fn {
            params: vec![(LambdaParam::ParamRef("ident1".into()), Ty::Generic(t0))],
            ret: Box::new(Ty::Generic(t1)),
        })),
    ),
    Ty::Generic(t1),
)
// DESIGN NOTE: The single-generic `lets` signature above is a simplification. In
// practice, `lets(a, 1, b, "hello", a + b)` has bindings of different types (Number
// and String). Full multi-generic support for variadic binders may require a custom
// SigResolver rather than the generic unification path. This is tracked as a follow-up.
```

### 5.5 List Transforms: `map`

```rust
// map<T, U>(list: T[], mapper: (current: T) -> U) -> U[]
func_g!(
    FunctionCategory::List,
    "map(list, mapper)",
    generics!(g!(0, Plain), g!(1, Plain)),
    "map",
    params!(
        p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
        p!("mapper", Ty::Fn {
            params: vec![(LambdaParam::Current, Ty::Generic(t0))],
            ret: Box::new(Ty::Generic(t1)),
        })
    ),
    Ty::List(Box::new(Ty::Generic(t1))),
)
```

### 5.6 List Transforms: `filter`, `find`, `some`, `every`, `count`

All predicate-taking list builtins follow the same pattern with `(current: T) -> Boolean`:

```rust
// filter<T>(list: T[], predicate: (current: T) -> Boolean) -> T[]
// find<T>(list: T[], predicate: (current: T) -> Boolean) -> T
// findIndex<T>(list: T[], predicate: (current: T) -> Boolean) -> Number
// some<T>(list: T[], predicate: (current: T) -> Boolean) -> Boolean
// every<T>(list: T[], predicate: (current: T) -> Boolean) -> Boolean
// count<T>(list: T[], predicate: (current: T) -> Boolean) -> Number

// Example: filter
func_g!(
    FunctionCategory::List,
    "filter(list, predicate)",
    generics!(g!(0, Plain)),
    "filter",
    params!(
        p!("list", Ty::List(Box::new(Ty::Generic(t0)))),
        p!("predicate", Ty::Fn {
            params: vec![(LambdaParam::Current, Ty::Generic(t0))],
            ret: Box::new(Ty::Boolean),
        })
    ),
    Ty::List(Box::new(Ty::Generic(t0))),
)
```

### 5.7 Summary Table

| Function | Param types | Lambda params | Return |
|----------|------------|---------------|--------|
| `if` | `Boolean, () -> T, () -> T` | `[], []` | `T` |
| `ifs` | `Boolean, () -> T, ..., () -> T` | `[], ..., []` | `T` |
| `let` | `Ident<T0>, T0, (ident: T0) -> T1` | `[ParamRef("ident")]` | `T1` |
| `lets` | `Ident<T0>, T0, ..., (ident1: T0, ...) -> T1` | `[ParamRef("ident1"), ...]` | `T1` |
| `map` | `T[], (current: T) -> U` | `[Current]` | `U[]` |
| `filter` | `T[], (current: T) -> Boolean` | `[Current]` | `T[]` |
| `find` | `T[], (current: T) -> Boolean` | `[Current]` | `T` |
| `findIndex` | `T[], (current: T) -> Boolean` | `[Current]` | `Number` |
| `some` | `T[], (current: T) -> Boolean` | `[Current]` | `Boolean` |
| `every` | `T[], (current: T) -> Boolean` | `[Current]` | `Boolean` |
| `count` | `T[], (current: T) -> Boolean` | `[Current]` | `Number` |

---

## 6. Evaluator / IR / Runtime

> **Status: TODO** — This section is intentionally deferred. The evaluator-side design for
> how implicit lambdas are lowered to IR and executed at runtime will be specified in the
> [Evaluator Builtins plan](./evaluator-builtins.md) once the analyzer-side implementation
> is complete.

### 6.1 Known Decisions (from evaluator-builtins design interview)

The following decisions have been made and will be incorporated into the evaluator plan:

1. **`ExecNode::Ternary { cond, then_branch, else_branch }`** — Both the `?:` operator
   and `if()` function lower to this IR node. `ifs()` desugars to nested Ternary via the
   rewrite table.

2. **`ExecNode::ShortCircuit { op: ShortCircuitOp, left, right }`** — Binary `&&`/`||`
   operators produce this. Future variadic `and()`/`or()` functions desugar to nested
   ShortCircuit via the rewrite table.

3. **`ExecNode::Lambda { params, body }`** — An explicit IR node for unevaluated
   subtrees. Parent Call nodes' arg lists contain Lambda nodes at lambda-typed positions.

4. **Rewrite table entries**:
   ```rust
   "if"  => Transform(rewrite_if_to_ternary)
   "ifs" => Transform(rewrite_ifs_to_nested_ternary)
   ```

5. **`&&`/`||` lowering** — Early check in the `ExprKind::Binary` match arm: if
   `AndAnd`/`OrOr`, emit `ShortCircuit` instead of `Binary`.

### 6.2 Open Questions (to be resolved in evaluator plan)

- **Lambda execution model**: Dedicated `ExecNode` eval arm vs. Call dispatch with eval
  access? The evaluator needs to "call into" a lambda body with bindings in scope. Options:
  - (A) `eval_node` matches `Lambda` and evaluates `body` with a pushed scope frame.
  - (B) Lambda bodies are inlined at the call site (no separate ExecNode evaluation).
  - (C) Lambdas are closures carrying an environment, evaluated by a `call_lambda` helper.

- **Scope representation at runtime**: The evaluator currently has no scope/environment
  mechanism. Does it use a `Vec<HashMap<String, Value>>` mirroring the analyzer, or
  something more efficient (flat array with indices)?

- **List builtin iteration**: How does `map` invoke the lambda per-element? Row-at-a-time
  vs. column-at-a-time? The existing evaluator uses columnar `EvalBlock`/`Column`, so
  list lambdas may need a different execution path.

These questions are deferred to the evaluator-builtins plan update.

---

## 7. Implementation Checklist

### Phase 1: Type System Foundation

- [x] Add `LambdaParam` enum to `analyzer/src/analysis/mod.rs`
- [x] Add `Ty::Fn { params, ret }` variant to `Ty` enum
- [x] Add `Ty::Ident(Box<Ty>)` variant to `Ty` enum
- [x] Update `Ty::precedence()` — `Fn` gets 0, `Ident` gets 3
- [x] Update `Ty::fmt_with_prec()` — display for `Fn` and `Ident`
- [x] Add `Serialize`/`Deserialize` for `LambdaParam`
- [x] Update `ty_accepts` with `Fn` and `Ident` arms
- [ ] Update snapshot tests for `Ty` display

### Phase 2: AST Extension

- [x] Add `ExprKind::ImplicitLambda { params, body }` variant to `ast.rs`
- [x] Update all `ExprKind` match arms across the codebase (exhaustiveness)
- [x] Add `ExprId::next()` method if not already present (for fresh wrapper ids)

### Phase 3: Inference Pass Rewrite

- [x] Change `infer_expr_with_map` signature from `&Expr` to `&mut Expr`
- [x] Propagate `&mut` through all recursive inference functions
- [x] Update all call sites in `analyze_expr` and validation
- [x] Add `InferState` struct with scope stack
- [x] Thread `InferState` through all inference functions
- [x] Update `ExprKind::Ident` arm to check scope stack
- [x] Implement two-pass `infer_call` (non-lambda first, then lambda)
- [x] Implement `resolve_param_ref` helper
- [x] Implement in-place `ImplicitLambda` wrapping in `infer_call`
- [x] Handle `MemberCall` lambda wrapping (clone-and-infer approach)

### Phase 4: Signature Updates

- [x] Update `if` signature to use `Ty::Fn` thunks
- [x] Update `ifs` signature to use `Ty::Fn` thunks
- [x] Add `let` signature with `Ty::Ident` and `Ty::Fn`
- [ ] Add `lets` signature (simplified or with custom resolver)
- [x] Add `map` signature with `Ty::Fn` + `LambdaParam::Current`
- [x] Add `filter` signature
- [x] Add `find` signature
- [x] Add `findIndex` signature
- [x] Add `some` signature
- [x] Add `every` signature
- [x] Add `count` signature
- [x] Remove TODO comments from `general.rs` and `list.rs`
- [ ] Update `docs/builtin_functions/README.md` spec entries

### Phase 5: Testing

- [x] Unit tests: `Ty::Fn` display and equality
- [x] Unit tests: `ty_accepts` with `Fn` and `Ident` types
- [x] Inference tests: `if(true, 1, "2")` infers to `Number | String`
- [x] Inference tests: `if(true, 1, 2)` infers to `Number`
- [x] Inference tests: `let(a, 123, a + 1)` infers to `Number`
- [x] Inference tests: `[1,2,3].map(current + 1)` infers to `List<Number>`
- [x] Inference tests: `[1,2,3].filter(current > 1)` infers to `List<Number>`
- [x] Inference tests: nested `let(a, 1, let(b, a + 1, b))` infers to `Number`
- [x] AST mutation tests: verify `ImplicitLambda` nodes are inserted at correct positions
- [x] Scope isolation tests: `current` does not leak outside lambda body
- [x] Validation tests: wrong-type lambda body produces diagnostic

### Phase 6: Evaluator Integration (deferred)

- [ ] Update evaluator-builtins plan with lambda IR design
- [ ] Implement `ExecNode::Lambda` in planner
- [ ] Implement `ExecNode::Ternary` (replaces/extends current `If`)
- [ ] Implement `ExecNode::ShortCircuit`
- [ ] Add rewrite table entries for `if`, `ifs`
- [ ] Implement runtime scope/environment for lambda execution
- [ ] Implement list-builtin iteration with lambda invocation

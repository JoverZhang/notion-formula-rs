# Evaluator Builtins: Design Plan

> **Status:** Superseded as a design source. The normative contracts now live in
> [`docs/design/builtin-fn.md`](../design/builtin-fn.md) and
> [`docs/design/evaluator.md`](../design/evaluator.md). Keep this file only as historical
> implementation-planning context; where it conflicts with those documents, the design
> documents take precedence.

This document captures the full design for implementing builtin function evaluation,
including the new `builtins` crate, codegen pipeline, IR extensions, and runtime execution model.

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [The `builtins` Crate](#2-the-builtins-crate)
3. [Codegen Pipeline](#3-codegen-pipeline)
4. [IR Extensions](#4-ir-extensions)
5. [Planner Enhancements](#5-planner-enhancements)
6. [Runtime Execution](#6-runtime-execution)
7. [Generated Code Examples](#7-generated-code-examples)
8. [Compilation and Runtime Flow](#8-compilation-and-runtime-flow)
9. [Phase 1 Deliverables](#9-phase-1-deliverables)

---

## 1. Architecture Overview

### Current State

```
analyzer (complete)
  └── 77 builtin signatures defined via macro DSL
  └── Full type inference with generics, unions, variadic params
  └── Spec-sync test against docs/builtin_functions/README.md

evaluator (skeletal)
  └── Only literals + 5 binary arithmetic kernels (+, -, *, /)
  └── No function call dispatch
  └── Planner passes functions: vec![] -- function inference always yields Unknown
```

### Target State

```
builtins (new leaf crate, no workspace dependencies)
  └── Ty, FunctionSig, ParamShape, GenericParam, FunctionCategory, SigResolver
  └── All 77 builtin function signatures
  └── normalize_union, resolve_flat, collect_leaf_types

analyzer (depends on: builtins)
  └── Lexer, parser, AST, diagnostics
  └── Inference engine, validation
  └── Imports Ty, FunctionSig, etc. from builtins

evaluator (depends on: builtins, analyzer)
  └── IR with typed ExecNode (Call, If, Prop, Unary, Binary, Cast)
  └── Planner with rewrite rules + structured arg resolution
  └── Codegen'd dispatch table + per-function traits
  └── Hand-written implementations in builtins/impls/
  └── Row-iteration helpers (map_f64_unary, map_f64_binary, etc.)

ide (depends on: analyzer, which re-exports from builtins)
analyzer_wasm (depends on: analyzer, ide)
```

### Dependency Graph

```
builtins ←── analyzer ←── evaluator
   ↑                        │
   └────────────────────────┘
                analyzer ←── ide
                analyzer ←── analyzer_wasm ──→ ide
```

`builtins` is a leaf with zero workspace dependencies. Both `analyzer` and `evaluator`
depend on it. The evaluator also depends on analyzer for AST types and inference.

---

## 2. The `builtins` Crate

### What Moves from `analyzer`

The following types and modules move from `analyzer/src/analysis/` into the new `builtins/` crate:

**Types** (from `analyzer/src/analysis/mod.rs`):
- `Ty` enum (`Number`, `String`, `Boolean`, `Date`, `Null`, `Unknown`, `Generic`, `List`, `Union`)
- `GenericId`
- `FunctionCategory` enum
- `Property` struct
- `ty_accepts()` function
- `is_postfix_capable()` function

**Signature model** (from `analyzer/src/analysis/signature.rs`):
- `FunctionSig`
- `ParamSig`
- `ParamShape`
- `GenericParam`
- `GenericParamKind`
- `SigResolver` type alias

**Type utilities** (from `analyzer/src/analysis/type_hints.rs`):
- `normalize_union()`
- `ty_sort_key()` (internal to normalize_union)

**Builtin definitions** (from `analyzer/src/analysis/builtins/`):
- `macros.rs` -- `p!`, `opt!`, `params!`, `func!`, `func_g!`, `func_gr!`, etc.
- `general.rs` -- `if`, `ifs`, `empty`, `length`, `format`, `equal`, `unequal`
- `text.rs` -- `substring`, `contains`, `test`, `match`, `replace`, etc.
- `math.rs` -- `add`, `abs`, `ceil`, `floor`, `sqrt`, `pi`, `e`, etc.
- `date.rs` -- `now`, `today`, `minute`, `dateAdd`, `formatDate`, etc.
- `people.rs` -- `name`, `email`
- `list.rs` -- `at`, `first`, `last`, `sort`, `flat` + `resolve_flat` + `collect_leaf_types`
- `special.rs` -- `id`
- `mod.rs` -- `builtins_functions() -> Vec<FunctionSig>`

**Param shape utilities** (from `analyzer/src/analysis/param_shape.rs`):
- `resolve_repeat_tail_used()`

### What Stays in `analyzer`

- Lexer, parser, AST (`Expr`, `ExprKind`, `BinOpKind`, `UnOp`, etc.)
- Inference engine (`infer.rs`: `TypeMap`, `infer_expr_with_map`, `Subst`, `unify`, `apply`)
- Validation (`validate_expr`, `validate_call`, `validate_arity`)
- Diagnostics model
- `Context` struct (it uses `FunctionSig` and `Property` from `builtins`)
- `postfix_capable_builtin_names()` lazy static
- The spec-sync test (`builtin_spec_sync.rs`)

### Crate Structure

```
builtins/
  Cargo.toml          # dependencies: serde (for Serialize/Deserialize on Ty, etc.)
  src/
    lib.rs            # re-exports everything
    ty.rs             # Ty, GenericId, FunctionCategory, Property, ty_accepts
    signature.rs      # FunctionSig, ParamSig, ParamShape, GenericParam, SigResolver
    type_hints.rs     # normalize_union
    param_shape.rs    # resolve_repeat_tail_used
    registry.rs       # builtins_functions(), is_postfix_capable
    macros.rs         # p!, opt!, params!, func!, func_g!, func_gr!
    defs/
      mod.rs
      general.rs
      text.rs
      math.rs
      date.rs
      people.rs
      list.rs         # includes resolve_flat, collect_leaf_types
      special.rs
```

---

## 3. Codegen Pipeline

### Overview

A Rust binary (`evaluator/src/bin/gen_builtins.rs`) reads all 77 signatures from
`builtins::builtins_functions()` and generates `evaluator/src/builtins/generated.rs`.

Invoked via `just gen-builtins`. Output is checked in to version control.

### What Gets Generated

For each of the 77 builtin functions, the codegen produces:

1. **A `BuiltinKey` enum** -- `#[repr(u8)]` with one variant per function (phase 1: `_Any` suffix only).
2. **A trait per function variant** -- typed parameters and typed return, derived from the `FunctionSig`.
3. **A dispatch table** -- static array mapping `BuiltinKey` -> function pointer.
4. **A `builtin_key_from_name()` lookup** -- `&str -> Option<BuiltinKey>`.
5. **A signature hash** -- in the file header comment, for drift detection.

### Ty-to-ColumnType Mapping

The codegen maps `builtins::Ty` to evaluator-level column types:

```rust
// In the evaluator, not in builtins crate:
enum ColumnType {
    F64,       // Ty::Number
    Bool,      // Ty::Boolean
    Str,       // Ty::String
    Date,      // Ty::Date (i64 epoch millis)
    List,      // Ty::List(_)
    Any,       // Ty::Union(_), Ty::Generic(_), Ty::Unknown
}
```

Mapping rules:
- `Ty::Number` -> `ColumnType::F64`
- `Ty::Boolean` -> `ColumnType::Bool`
- `Ty::String` -> `ColumnType::Str`
- `Ty::Date` -> `ColumnType::Date`
- `Ty::List(_)` -> `ColumnType::List`
- `Ty::Union(_)` -> `ColumnType::Any`
- `Ty::Generic(_)` -> `ColumnType::Any`
- `Ty::Unknown` -> `ColumnType::Any`
- `Ty::Null` -> `ColumnType::Any`

### Ty-to-Rust-Type Mapping for Trait Parameters

For the `_Any` variant (phase 1), parameters and returns map as follows:

| ColumnType | `Column` variant | Kernel input type (per-row) | Kernel output type (column) |
|---|---|---|---|
| `F64` | `Column::F64(Vec<f64>)` | `f64` | `Vec<f64>` |
| `Bool` | `Column::Bool(Vec<bool>)` | `bool` | `Vec<bool>` |
| `Str` | `Column::Str(Vec<String>)` | `&str` | `Vec<String>` |
| `Date` | `Column::Date(Vec<i64>)` | `i64` | `Vec<i64>` |
| `List` | `Column::List(Vec<Vec<Value>>)` | `&[Value]` | `Vec<Vec<Value>>` |
| `Any` | `Column::Any(Vec<Value>)` | `&Value` | `Vec<Value>` |

Each `ColumnType` has a 1:1 corresponding `Column` variant. The planner sets the
`output_type` on every `ExecNode::Call`, guaranteeing the `Column` variant at runtime.
Generated dispatch wrappers use `unsafe` unchecked accessors (see Section 4) to extract
typed slices without branching in release builds.

### BuiltinKey Enum (Generated)

```rust
// evaluator/src/builtins/generated.rs (generated by gen_builtins)

/// Auto-generated from builtins::builtins_functions().
/// Signature hash: <sha256 of serialized signatures>
/// Re-generate with: just gen-builtins

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuiltinKey {
    // General
    IfAny = 0,
    IfsAny = 1,
    EmptyAny = 2,
    LengthAny = 3,
    FormatAny = 4,
    EqualAny = 5,
    UnequalAny = 6,
    // Text
    SubstringAny = 7,
    ContainsAny = 8,
    // ... (all 77 builtins)
}

pub(crate) const BUILTIN_COUNT: usize = 77;

pub(crate) fn builtin_key_from_name(name: &str) -> Option<BuiltinKey> {
    match name {
        "if" => Some(BuiltinKey::IfAny),
        "ifs" => Some(BuiltinKey::IfsAny),
        "empty" => Some(BuiltinKey::EmptyAny),
        // ... all 77 entries
        _ => None,
    }
}
```

### Per-Function Trait (Generated)

Each builtin gets a trait whose method signature is derived from the `FunctionSig`.
The codegen maps each parameter's `Ty` to the corresponding Rust type.

**Example: `abs(value: Number) -> Number`**

```rust
// generated.rs
pub(crate) trait AbsAnyKernel {
    fn exec(
        value: &[f64],    // Number -> &[f64]
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
    //    ^^^^^^^^  return type: Number -> Vec<f64>
}
```

**Example: `lower(text: String) -> String`**

```rust
pub(crate) trait LowerAnyKernel {
    fn exec(
        text: &[String],    // String -> &[String]
        mask: &Mask,
    ) -> (Vec<String>, Vec<(usize, EvalError)>);
}
```

**Example: `contains(text: String, search: String) -> Boolean`**

```rust
pub(crate) trait ContainsAnyKernel {
    fn exec(
        text: &[String],
        search: &[String],
        mask: &Mask,
    ) -> (Vec<bool>, Vec<(usize, EvalError)>);
}
```

**Example: `length(value: String | List(T0)) -> Number` (union param)**

```rust
pub(crate) trait LengthAnyKernel {
    fn exec(
        value: &[Value],    // Union -> &[Value]
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
}
```

**Example: `at(list: List(T0), index: Number) -> T0` (generic return)**

```rust
pub(crate) trait AtAnyKernel {
    fn exec(
        list: &[Value],     // List(Generic) -> &[Value]
        index: &[f64],      // Number -> &[f64]
        mask: &Mask,
    ) -> (Vec<Value>, Vec<(usize, EvalError)>);
    //    ^^^^^^^^^^  Generic return -> Vec<Value>
}
```

**Example: `min(values...)` (variadic with repeat group)**

For variadic functions, the trait receives structured args:

```rust
pub(crate) trait MinAnyKernel {
    fn exec(
        // head: (none)
        repeat_groups: &[&[Value]],  // repeat: [values: Number|Number[]]... -> &[&[Value]]
        // tail: (none)
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
}
```

**Example: `splice(list, startIndex, deleteCount, ...items)` (head + repeat)**

```rust
pub(crate) trait SpliceAnyKernel {
    fn exec(
        // head params
        list: &[Value],            // List(T0) -> &[Value]
        start_index: &[f64],       // Number -> &[f64]
        delete_count: &[f64],      // Number -> &[f64]
        // repeat groups
        repeat_groups: &[&[Value]], // repeat: [items: T0]... -> &[&[Value]]
        // tail: (none)
        mask: &Mask,
    ) -> (Vec<Value>, Vec<(usize, EvalError)>);
}
```

### Dispatch Table (Generated)

```rust
// generated.rs

pub(crate) type BuiltinExecFn = fn(&BuiltinArgs, &Mask) -> BuiltinResult;

pub(crate) struct BuiltinEntry {
    pub name: &'static str,
    pub exec: BuiltinExecFn,
    pub output_type: ColumnType,
}

pub(crate) static BUILTIN_REGISTRY: [BuiltinEntry; BUILTIN_COUNT] = [
    BuiltinEntry {
        name: "if",
        exec: exec_if_any,        // wired to impl
        output_type: ColumnType::Any,
    },
    // ... all 77 entries
];
```

### Drift Detection

The codegen binary computes a SHA-256 hash of the serialised signature data
(names, param types, return types, categories, in order) and writes it into the
generated file header.

A test in `evaluator/tests/builtin_drift.rs` recomputes the same hash from
`builtins::builtins_functions()` and compares it against the header. If they differ,
the test fails:

```
FAILED: builtin signature hash mismatch.
  expected: a1b2c3d4...
  got:      e5f6g7h8...
Run `just gen-builtins` to regenerate evaluator/src/builtins/generated.rs.
```

---

## 4. IR Extensions

### Current IR (`ExecNode`)

```rust
// evaluator/src/ir/nodes.rs (current)
enum ExecNode {
    LiteralF64(f64),
    LiteralAny(Value),
    CastToF64 { input: Box<ExecNode> },
    Binary { key: BinaryExecKey, left: Box<ExecNode>, right: Box<ExecNode> },
}
```

### Target IR

```rust
// evaluator/src/ir/nodes.rs (target)

/// Column-level type tag. Set by the planner based on analyzer type inference.
/// Used for kernel selection and cast insertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColumnType {
    F64,
    Bool,
    Str,
    Date,
    List,
    Any,
}

/// Columnar storage — one variant per ColumnType.
/// Replaces the existing Column { F64, Any } with a full set of typed variants.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Column {
    F64(Vec<f64>),
    Bool(Vec<bool>),
    Str(Vec<String>),
    Date(Vec<i64>),
    List(Vec<Vec<Value>>),
    Any(Vec<Value>),
}

impl Column {
    // -- Safe accessors (return Option) --

    pub(crate) fn as_f64_slice(&self) -> Option<&[f64]> {
        match self { Column::F64(v) => Some(v), _ => None }
    }
    pub(crate) fn as_bool_slice(&self) -> Option<&[bool]> {
        match self { Column::Bool(v) => Some(v), _ => None }
    }
    pub(crate) fn as_str_slice(&self) -> Option<&[String]> {
        match self { Column::Str(v) => Some(v), _ => None }
    }
    pub(crate) fn as_date_slice(&self) -> Option<&[i64]> {
        match self { Column::Date(v) => Some(v), _ => None }
    }
    pub(crate) fn as_value_slice(&self) -> Option<&[Value]> {
        match self { Column::Any(v) => Some(v), _ => None }
    }

    // -- Unchecked accessors (used by generated dispatch wrappers) --
    //
    // Safety: The planner guarantees each ExecNode's output_type matches
    // the Column variant produced at runtime. These accessors rely on that
    // invariant. In debug builds the assert catches planner bugs; in
    // release builds the branch is elided entirely.

    /// # Safety: caller must guarantee this is Column::F64.
    pub(crate) unsafe fn as_f64_unchecked(&self) -> &[f64] {
        debug_assert!(matches!(self, Column::F64(_)), "expected Column::F64");
        match self { Column::F64(v) => v, _ => std::hint::unreachable_unchecked() }
    }
    /// # Safety: caller must guarantee this is Column::Bool.
    pub(crate) unsafe fn as_bool_unchecked(&self) -> &[bool] {
        debug_assert!(matches!(self, Column::Bool(_)), "expected Column::Bool");
        match self { Column::Bool(v) => v, _ => std::hint::unreachable_unchecked() }
    }
    /// # Safety: caller must guarantee this is Column::Str.
    pub(crate) unsafe fn as_str_unchecked(&self) -> &[String] {
        debug_assert!(matches!(self, Column::Str(_)), "expected Column::Str");
        match self { Column::Str(v) => v, _ => std::hint::unreachable_unchecked() }
    }
    /// # Safety: caller must guarantee this is Column::Date.
    pub(crate) unsafe fn as_date_unchecked(&self) -> &[i64] {
        debug_assert!(matches!(self, Column::Date(_)), "expected Column::Date");
        match self { Column::Date(v) => v, _ => std::hint::unreachable_unchecked() }
    }
    /// # Safety: caller must guarantee this is Column::Any.
    pub(crate) unsafe fn as_value_unchecked(&self) -> &[Value] {
        debug_assert!(matches!(self, Column::Any(_)), "expected Column::Any");
        match self { Column::Any(v) => v, _ => std::hint::unreachable_unchecked() }
    }
}

/// Unary operation key, analogous to BinaryExecKey.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryExecKey {
    NegF64 = 0,
    NotBool = 1,
}

/// Extended binary operation key (existing + new comparison/logical ops).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryExecKey {
    // Arithmetic (existing)
    AddF64 = 0,
    AddAny = 1,
    SubF64 = 2,
    MulF64 = 3,
    DivF64 = 4,
    // Arithmetic (new)
    ModF64 = 5,
    PowF64 = 6,
    // Comparison
    EqAny = 7,
    NeAny = 8,
    LtF64 = 9,
    LeF64 = 10,
    GtF64 = 11,
    GeF64 = 12,
    // Logical
    AndBool = 13,
    OrBool = 14,
}

/// Generalised cast (replaces CastToF64).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CastPlan {
    None,
    ToF64,
    ToBool,
    ToAny,
}

enum ExecNode {
    // --- Existing (unchanged) ---
    LiteralF64(f64),
    LiteralAny(Value),

    // --- Generalised cast (replaces CastToF64) ---
    Cast {
        input: Box<ExecNode>,
        from: ColumnType,
        to: ColumnType,
    },

    // --- Binary (extended key set) ---
    Binary {
        key: BinaryExecKey,
        left: Box<ExecNode>,
        right: Box<ExecNode>,
    },

    // --- New: Unary ---
    Unary {
        key: UnaryExecKey,
        operand: Box<ExecNode>,
    },

    // --- New: Conditional (mask-driven branching) ---
    If {
        cond: Box<ExecNode>,        // must produce ColumnType::Bool
        then_branch: Box<ExecNode>,
        else_branch: Box<ExecNode>,
    },

    // --- New: Property access ---
    Prop {
        name: String,
    },

    // --- New: Builtin function call ---
    Call {
        key: BuiltinKey,
        head: Vec<ExecNode>,
        repeat_groups: Vec<Vec<ExecNode>>,
        tail: Vec<ExecNode>,
        output_type: ColumnType,
    },
}
```

### Key Design Decisions for the IR

**`ExecNode::If`**: Used for both the `if()` function and the `?:` ternary operator.
The planner lowers both to the same node. `ifs()` is desugared into nested `If` nodes:

```
ifs(c1, v1, c2, v2, else)
  --> If { cond: c1, then: v1, else: If { cond: c2, then: v2, else: else } }
```

**`ExecNode::Prop`**: Dedicated node for `prop("Name")`. Not routed through `Call` dispatch.
The evaluator calls `Provider::get_prop(name, batch, mask)` at runtime.

**`ExecNode::Call`**: Structured arguments. The planner resolves `ParamShape` (head/repeat/tail)
at plan time using `resolve_repeat_tail_used()`, so the function implementation receives
pre-structured argument groups. `output_type` is derived from the function's return `Ty`.

**`ColumnType` on `Call`**: Enables zero-cost forwarding when composing functions.
For example, `sum(1, sum(2, 3))`: the inner `sum` outputs `ColumnType::F64`, and the outer
`sum` expects `ColumnType::F64` input -- no conversion node needed. The planner only inserts
`Cast` nodes when output/input types mismatch.

**`Column` expansion and unchecked accessors**: `Column` has one variant per `ColumnType`
(`F64`, `Bool`, `Str`, `Date`, `List`, `Any`). Generated dispatch wrappers extract typed
slices from `Column` via `unsafe` unchecked accessors (`as_f64_unchecked()`, etc.) that
compile to zero branches in release builds. A `debug_assert!` guard on each accessor
catches planner type-guarantee violations during development. This avoids the cost of
enum-matching per argument while keeping `EvalBlock` as the uniform intermediate type
between `eval_node` calls.

---

## 5. Planner Enhancements

### Fix: Pass Real Builtins to Inference

The planner currently passes `functions: vec![]` to the semantic context (planner.rs:18).
This must be changed to pass `builtins::builtins_functions()` so the type inference
engine produces accurate `Ty` for every subexpression -- especially function return types.

```rust
// Before (broken):
let sema_ctx = SemaContext {
    properties: ctx.properties.clone(),
    functions: vec![],
};

// After (correct):
let sema_ctx = SemaContext {
    properties: ctx.properties.clone(),
    functions: builtins::builtins_functions(),
};
```

### Rewrite Rules

Location: `evaluator/src/planner/rewrites.rs`

Some builtin functions are expressible as existing IR constructs. The planner
checks the rewrite table before generating `ExecNode::Call`.

```rust
/// Alias rewrites: function call -> existing IR node.
pub(crate) enum AliasRewrite {
    BinaryOp(BinOpKind),     // add(a,b) -> Binary(+), subtract(a,b) -> Binary(-), etc.
    Comparison(BinOpKind),   // equal(a,b) -> Binary(==), unequal(a,b) -> Binary(!=)
}

/// Complex rewrites: function call -> custom IR transformation.
/// Receives AST args + TypeMap, produces ExecNode.
pub(crate) type TransformRewriteFn = fn(
    args: &[Expr],
    planner: &Planner,
    map: &TypeMap,
) -> Result<ExecNode, PlanError>;

pub(crate) enum RewriteRule {
    Alias(AliasRewrite),
    Transform(TransformRewriteFn),
}

/// Returns the rewrite rule for a builtin function, if any.
pub(crate) fn get_rewrite(name: &str) -> Option<RewriteRule> {
    match name {
        // Arithmetic aliases
        "add"      => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Plus))),
        "subtract" => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Minus))),
        "multiply" => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Star))),
        "divide"   => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Slash))),
        "mod"      => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Percent))),
        "pow"      => Some(RewriteRule::Alias(AliasRewrite::BinaryOp(BinOpKind::Caret))),
        // Comparison aliases
        "equal"    => Some(RewriteRule::Alias(AliasRewrite::Comparison(BinOpKind::EqEq))),
        "unequal"  => Some(RewriteRule::Alias(AliasRewrite::Comparison(BinOpKind::Ne))),
        // Complex transforms
        "ifs"      => Some(RewriteRule::Transform(rewrite_ifs_to_nested_if)),
        _ => None,
    }
}
```

### Planner Lowering Logic (Pseudocode)

```rust
fn lower(&self, expr: &Expr, map: &TypeMap) -> Result<ExecNode, PlanError> {
    match &expr.kind {
        ExprKind::Group { inner } => self.lower(inner, map),
        ExprKind::Lit(lit) => lower_lit(lit),
        ExprKind::List { items } => lower_const_list(items),

        // Ternary -> ExecNode::If
        ExprKind::Ternary { cond, then, otherwise } => {
            Ok(ExecNode::If {
                cond: Box::new(self.lower(cond, map)?),
                then_branch: Box::new(self.lower(then, map)?),
                else_branch: Box::new(self.lower(otherwise, map)?),
            })
        }

        // Unary -> ExecNode::Unary
        ExprKind::Unary { op, expr } => {
            let operand = self.lower(expr, map)?;
            let key = select_unary_key(op, &inferred_ty_for_expr(map, expr)?)?;
            Ok(ExecNode::Unary { key, operand: Box::new(operand) })
        }

        // Binary -> ExecNode::Binary (extended to cover all BinOpKind)
        ExprKind::Binary { op, left, right } => {
            let left_node = self.lower(left, map)?;
            let right_node = self.lower(right, map)?;
            let left_ty = inferred_ty_for_expr(map, left)?;
            let right_ty = inferred_ty_for_expr(map, right)?;
            let plan = select_binary_plan(op.node, &left_ty, &right_ty)?;
            Ok(ExecNode::Binary {
                key: plan.key,
                left: Box::new(apply_cast(left_node, plan.left_cast)),
                right: Box::new(apply_cast(right_node, plan.right_cast)),
            })
        }

        // Call -> check rewrite, then dispatch
        ExprKind::Call { callee, args } => {
            match callee.text.as_str() {
                "prop" => lower_prop(args),
                name => {
                    // Check rewrite rules first
                    if let Some(rule) = get_rewrite(name) {
                        return apply_rewrite(rule, args, self, map);
                    }
                    // Standard call dispatch
                    lower_call(name, args, self, map)
                }
            }
        }

        // MemberCall -> flatten receiver into args, then same as Call
        ExprKind::MemberCall { receiver, method, args } => {
            let mut all_args = vec![(**receiver).clone()];
            all_args.extend(args.iter().cloned());
            // Then same logic as Call with method.text
            let name = method.text.as_str();
            if let Some(rule) = get_rewrite(name) {
                return apply_rewrite(rule, &all_args, self, map);
            }
            lower_call(name, &all_args, self, map)
        }

        _ => Err(PlanError::InvalidArgument),
    }
}

fn lower_call(
    name: &str,
    args: &[Expr],
    planner: &Planner,
    map: &TypeMap,
) -> Result<ExecNode, PlanError> {
    let key = builtin_key_from_name(name)
        .ok_or(PlanError::UnknownFunction)?;

    let sig = builtins::builtins_functions()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap();

    // Resolve structured args using ParamShape
    let (head_args, repeat_groups, tail_args) =
        resolve_structured_args(&sig.params, args, planner, map)?;

    let output_type = ty_to_column_type(&sig.ret, map, args);

    Ok(ExecNode::Call {
        key,
        head: head_args,
        repeat_groups,
        tail: tail_args,
        output_type,
    })
}
```

### Structured Arg Resolution

The planner splits a flat argument list into head/repeat_groups/tail
using `resolve_repeat_tail_used()`:

```
splice([1,2,3], 0, 1, 10, 20)
                                    
sig.params: head=[list, startIndex, deleteCount], repeat=[items], tail=[]

resolve_repeat_tail_used(params, 5) -> tail_used = 0
  head_count = 3, tail_used = 0, middle = 5 - 3 - 0 = 2
  repeat_group_size = 1, groups = 2 / 1 = 2

Result:
  head:          [lower([1,2,3]), lower(0), lower(1)]
  repeat_groups: [[lower(10)], [lower(20)]]
  tail:          []
```

---

## 6. Runtime Execution

### eval_node Extension

```rust
fn eval_node(&self, node: &ExecNode, len: usize, mask: &Mask) -> EvalBlock {
    match node {
        ExecNode::LiteralF64(v) => literal_f64(*v, len, mask),
        ExecNode::LiteralAny(v) => literal_any(v.clone(), len, mask),
        ExecNode::Cast { input, from, to } => {
            let input = self.eval_node(input, len, mask);
            cast_block(input, mask, *from, *to)
        }
        ExecNode::Binary { key, left, right } => {
            let left = self.eval_node(left, len, mask);
            let right = self.eval_node(right, len, mask);
            dispatch_binary(*key, left, right, mask)
        }
        ExecNode::Unary { key, operand } => {
            let operand = self.eval_node(operand, len, mask);
            dispatch_unary(*key, operand, mask)
        }
        ExecNode::If { cond, then_branch, else_branch } => {
            self.eval_if(cond, then_branch, else_branch, len, mask)
        }
        ExecNode::Prop { name } => {
            self.eval_prop(name, len, mask)
        }
        ExecNode::Call { key, head, repeat_groups, tail, output_type } => {
            self.eval_call(*key, head, repeat_groups, tail, *output_type, len, mask)
        }
    }
}
```

### If Evaluation (Mask-Driven Branching)

```rust
fn eval_if(
    &self,
    cond: &ExecNode,
    then_branch: &ExecNode,
    else_branch: &ExecNode,
    len: usize,
    mask: &Mask,
) -> EvalBlock {
    // 1. Evaluate condition for all active rows
    let cond_block = self.eval_node(cond, len, mask);

    // 2. Split mask: then_mask[i] = mask[i] && cond[i] == true
    //               else_mask[i] = mask[i] && cond[i] != true
    let (then_mask, else_mask) = split_mask_on_bool(&cond_block, mask);

    // 3. Evaluate branches only for their respective rows
    let then_result = self.eval_node(then_branch, len, &then_mask);
    let else_result = self.eval_node(else_branch, len, &else_mask);

    // 4. Merge: for each row, pick the result from the branch that was active
    merge_blocks(then_result, else_result, &then_mask, &else_mask)
}
```

### Call Evaluation

```rust
fn eval_call(
    &self,
    key: BuiltinKey,
    head: &[ExecNode],
    repeat_groups: &[Vec<ExecNode>],
    tail: &[ExecNode],
    output_type: ColumnType,
    len: usize,
    mask: &Mask,
) -> EvalBlock {
    // 1. Evaluate all argument nodes -> EvalBlock (typed Column inside)
    let head_blocks: Vec<EvalBlock> = head.iter()
        .map(|n| self.eval_node(n, len, mask))
        .collect();
    let repeat_blocks: Vec<Vec<EvalBlock>> = repeat_groups.iter()
        .map(|group| group.iter().map(|n| self.eval_node(n, len, mask)).collect())
        .collect();
    let tail_blocks: Vec<EvalBlock> = tail.iter()
        .map(|n| self.eval_node(n, len, mask))
        .collect();

    // 2. Dispatch to the generated wrapper function.
    //    Inside the wrapper, each argument's Column is accessed via unsafe
    //    unchecked accessors (e.g., column.as_f64_unchecked()). The planner
    //    guarantees the Column variant matches the expected ColumnType, so
    //    there is no branch cost in release builds. See Section 7.5 for
    //    concrete wrapper examples.
    let entry = &BUILTIN_REGISTRY[key as usize];
    let args = BuiltinArgs { head: head_blocks, repeat_groups: repeat_blocks, tail: tail_blocks };
    (entry.exec)(&args, mask)
}
```

**Why `EvalBlock` is still the intermediate type**: Each `eval_node` call produces an
`EvalBlock` containing a *typed* `Column` variant (e.g., `Column::F64(Vec<f64>)`). The
`EvalBlock` envelope carries error and null metadata alongside the column data. The
generated dispatch wrappers extract the inner typed slice via `unsafe` unchecked accessors
-- this is a pointer cast, not a copy. The kernel then operates on `&[f64]` / `&[String]`
/ etc. directly. The overhead versus a fully monomorphised `eval_node<T>` approach is
zero branches in release (the `debug_assert!` is compiled out) and zero allocations
(the `Vec<f64>` is borrowed, not cloned).

### Prop Evaluation

```rust
fn eval_prop(&self, name: &str, len: usize, mask: &Mask) -> EvalBlock {
    // Delegate to Provider trait
    match self.provider.get_prop(name, len, mask) {
        Ok(column_block) => EvalBlock::from_column(column_block),
        Err(e) => EvalBlock::fail_mask(mask, EvalError::from(e)),
    }
}
```

### Row-Iteration Helpers

These utilities handle the per-row mask/null/ok/error bookkeeping
so that kernel implementations only provide the scalar logic.

```rust
// evaluator/src/builtins/helpers.rs

/// Map a unary f64 operation across all active rows.
pub(crate) fn map_f64_unary(
    input: &[f64],
    mask: &Mask,
    f: impl Fn(f64) -> Result<f64, EvalError>,
) -> (Vec<f64>, Vec<(usize, EvalError)>) {
    let len = input.len();
    let mut output = vec![0.0; len];
    let mut errors = Vec::new();

    for i in 0..len {
        if !mask[i] { continue; }
        match f(input[i]) {
            Ok(v) => output[i] = v,
            Err(e) => {
                errors.push((i, e));
                // output[i] remains sentinel 0.0
            }
        }
    }
    (output, errors)
}

/// Map a binary f64 operation across all active rows.
pub(crate) fn map_f64_binary(
    left: &[f64],
    right: &[f64],
    mask: &Mask,
    f: impl Fn(f64, f64) -> Result<f64, EvalError>,
) -> (Vec<f64>, Vec<(usize, EvalError)>) {
    let len = left.len();
    let mut output = vec![0.0; len];
    let mut errors = Vec::new();

    for i in 0..len {
        if !mask[i] { continue; }
        match f(left[i], right[i]) {
            Ok(v) => output[i] = v,
            Err(e) => errors.push((i, e)),
        }
    }
    (output, errors)
}

/// Map a unary Value->Value operation across all active rows.
pub(crate) fn map_any_unary(
    input: &[Value],
    mask: &Mask,
    f: impl Fn(&Value) -> Result<Value, EvalError>,
) -> (Vec<Value>, Vec<(usize, EvalError)>) {
    let len = input.len();
    let mut output = Vec::with_capacity(len);
    let mut errors = Vec::new();

    for i in 0..len {
        if !mask[i] {
            output.push(Value::Number(0.0)); // sentinel
            continue;
        }
        match f(&input[i]) {
            Ok(v) => output.push(v),
            Err(e) => {
                errors.push((i, e));
                output.push(Value::Number(0.0));
            }
        }
    }
    (output, errors)
}

// Additional variants: map_str_unary, map_bool_unary, etc.
```

### Example Kernel Implementations

```rust
// evaluator/src/builtins/impls/math.rs

struct AbsAny;
impl AbsAnyKernel for AbsAny {
    fn exec(value: &[f64], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_f64_unary(value, mask, |v| Ok(v.abs()))
    }
}

struct CeilAny;
impl CeilAnyKernel for CeilAny {
    fn exec(value: &[f64], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_f64_unary(value, mask, |v| Ok(v.ceil()))
    }
}

struct SqrtAny;
impl SqrtAnyKernel for SqrtAny {
    fn exec(value: &[f64], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_f64_unary(value, mask, |v| {
            if v < 0.0 { Err(EvalError::InvalidArgument) }
            else { Ok(v.sqrt()) }
        })
    }
}
```

```rust
// evaluator/src/builtins/impls/text.rs

struct LowerAny;
impl LowerAnyKernel for LowerAny {
    fn exec(text: &[String], mask: &Mask) -> (Vec<String>, Vec<(usize, EvalError)>) {
        map_str_unary(text, mask, |s| Ok(s.to_lowercase()))
    }
}

struct ContainsAny;
impl ContainsAnyKernel for ContainsAny {
    fn exec(
        text: &[String],
        search: &[String],
        mask: &Mask,
    ) -> (Vec<bool>, Vec<(usize, EvalError)>) {
        map_str_binary(text, search, mask, |t, s| Ok(t.contains(s)))
    }
}
```

```rust
// evaluator/src/builtins/impls/general.rs

struct LengthAny;
impl LengthAnyKernel for LengthAny {
    fn exec(value: &[Value], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_any_unary(value, mask, |v| match v {
            Value::Text(s) => Ok(Value::Number(s.len() as f64)),
            Value::List(items) => Ok(Value::Number(items.len() as f64)),
            _ => Err(EvalError::TypeMismatch),
        })
        // Note: the return is (Vec<Value>, ...) from map_any_unary,
        // but the trait expects (Vec<f64>, ...).
        // A wrapper or specialised map function handles the conversion.
    }
}
```

### Return Type Validation (Test-Only)

For `_Any` variants where the return type cannot be verified at compile time
(e.g., `length` returns `Number` but the trait returns `Vec<Value>` since the
*input* is polymorphic), a test-time assertion validates correctness:

```rust
// evaluator/tests/return_type_check.rs
#[test]
fn builtin_return_types_match_signatures() {
    for sig in builtins::builtins_functions() {
        // For each builtin, run a set of well-typed sample inputs
        // and verify the output column type matches the expected Ty.
        let samples = sample_inputs_for(&sig);
        for (args, expected_ret_ty) in samples {
            let result = dispatch_builtin(sig.name, args);
            assert_column_type_matches(result, expected_ret_ty);
        }
    }
}
```

---

## 7. Generated Code Examples

This section shows the **complete output** of `gen_builtins` for representative builtins,
so that the shape of generated code is unambiguous to the implementor.

### 7.1 File Structure

Running `just gen-builtins` produces a single file:

```
evaluator/src/builtins/generated.rs
```

The file is divided into four regions:

1. **Header** -- generation metadata and drift-detection hash
2. **BuiltinKey enum** -- one variant per (function x specialization) pair
3. **Per-function traits** -- one trait per BuiltinKey variant
4. **Dispatch table** -- static array mapping BuiltinKey -> exec fn + metadata

### 7.2 Header

```rust
// @generated by gen_builtins -- do not edit by hand.
// Source: builtins::builtins_functions()
// Signature hash: sha256:a3f1c8e907b4d2...
// Re-generate with: just gen-builtins

use crate::builtins::helpers::BuiltinArgs;
use crate::core::errors::EvalError;
use crate::core::types::{Mask, Value};

/// Total number of registered builtin variants.
pub(crate) const BUILTIN_COUNT: usize = 77;
```

### 7.3 BuiltinKey Enum (Full _Any Excerpt)

```rust
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BuiltinKey {
    // -- General --
    IfAny        = 0,
    IfsAny       = 1,
    EmptyAny     = 2,
    LengthAny    = 3,
    FormatAny    = 4,
    EqualAny     = 5,
    UnequalAny   = 6,
    NotAny       = 7,
    AndAny       = 8,
    OrAny        = 9,
    // -- Text --
    SubstringAny = 10,
    ContainsAny  = 11,
    TestAny      = 12,
    MatchAny     = 13,
    ReplaceAny   = 14,
    ReplaceAllAny = 15,
    LowerAny     = 16,
    UpperAny     = 17,
    RepeatAny    = 18,
    LinkAny      = 19,
    StyleAny     = 20,
    UnstyleAny   = 21,
    PadAny       = 22,
    TrimAny      = 23,
    JoinAny      = 24,
    SplitAny     = 25,
    // -- Math --
    AbsAny       = 26,
    CeilAny      = 27,
    FloorAny     = 28,
    SqrtAny      = 29,
    CbrtAny      = 30,
    ExpAny       = 31,
    Exp2Any      = 32,
    Exp10Any     = 33,
    LnAny        = 34,
    Log2Any      = 35,
    Log10Any     = 36,
    SignAny       = 37,
    RoundAny     = 38,
    MinAny       = 39,
    MaxAny       = 40,
    SumAny       = 41,
    // ... remaining math, date, people, list, special builtins ...
    // (all 77 entries, one per builtin)
}
```

When future specializations are added, new variants appear alongside the `_Any` ones:

```rust
    // Phase 2+ additions (not generated in Phase 1):
    // AbsF64       = 77,
    // SqrtF64      = 78,
    // ...
```

The `BuiltinKey -> name` lookup:

```rust
pub(crate) fn builtin_key_from_name(name: &str) -> Option<BuiltinKey> {
    match name {
        "if"         => Some(BuiltinKey::IfAny),
        "ifs"        => Some(BuiltinKey::IfsAny),
        "empty"      => Some(BuiltinKey::EmptyAny),
        "length"     => Some(BuiltinKey::LengthAny),
        "format"     => Some(BuiltinKey::FormatAny),
        "equal"      => Some(BuiltinKey::EqualAny),
        "unequal"    => Some(BuiltinKey::UnequalAny),
        "not"        => Some(BuiltinKey::NotAny),
        "and"        => Some(BuiltinKey::AndAny),
        "or"         => Some(BuiltinKey::OrAny),
        "substring"  => Some(BuiltinKey::SubstringAny),
        "contains"   => Some(BuiltinKey::ContainsAny),
        // ... all 77 entries ...
        _ => None,
    }
}
```

### 7.4 Per-Function Traits (Concrete Examples)

Each trait is generated from the function's `FunctionSig`. The codegen walks the
`params` list and maps each `Ty` to a Rust type according to the type-mapping table
from Section 3.

**`abs(value: Number) -> Number` -- simple unary, single concrete type**

```rust
/// Kernel trait for `abs` (_Any variant).
/// Signature: abs(value: Number) -> Number
pub(crate) trait AbsAnyKernel {
    fn exec(
        value: &[f64],
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
}
```

**`contains(text: String, search: String) -> Boolean` -- binary, mixed types**

```rust
/// Kernel trait for `contains` (_Any variant).
/// Signature: contains(text: String, search: String) -> Boolean
pub(crate) trait ContainsAnyKernel {
    fn exec(
        text: &[String],
        search: &[String],
        mask: &Mask,
    ) -> (Vec<bool>, Vec<(usize, EvalError)>);
}
```

**`substring(text: String, start?: Number, end?: Number) -> String` -- optional params**

Optional parameters are lowered to the same column type as non-optional.
The planner provides a sentinel column (e.g., `NaN` for Number, empty string for String)
for absent optionals, with a companion presence-mask column.

```rust
/// Kernel trait for `substring` (_Any variant).
/// Signature: substring(text: String, start?: Number, end?: Number) -> String
pub(crate) trait SubstringAnyKernel {
    fn exec(
        text: &[String],
        start: &[f64],
        start_present: &Mask,
        end_val: &[f64],
        end_present: &Mask,
        mask: &Mask,
    ) -> (Vec<String>, Vec<(usize, EvalError)>);
}
```

**`min(values: Number | Number[]...) -> Number` -- variadic repeat-only**

```rust
/// Kernel trait for `min` (_Any variant).
/// Signature: min(values: Number | Number[]...) -> Number
pub(crate) trait MinAnyKernel {
    fn exec(
        repeat_groups: &[&[Value]],
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
}
```

**`splice(list: List(T0), start: Number, deleteCount: Number, items: T0...) -> List(T0)` -- head + repeat**

```rust
/// Kernel trait for `splice` (_Any variant).
/// Signature: splice(list: List(T0), start: Number, deleteCount: Number, items: T0...) -> List(T0)
pub(crate) trait SpliceAnyKernel {
    fn exec(
        list: &[Value],
        start_index: &[f64],
        delete_count: &[f64],
        repeat_groups: &[&[Value]],
        mask: &Mask,
    ) -> (Vec<Value>, Vec<(usize, EvalError)>);
}
```

**`flat(list: List(T0 | List(T0)), depth?: Number) -> List(T0)` -- custom SigResolver**

The `flat` function has a custom `SigResolver` in the analyzer. For codegen purposes
the signature is treated the same as any other -- the resolver is an analyzer concern.
The generated trait follows the standard pattern:

```rust
/// Kernel trait for `flat` (_Any variant).
/// Signature: flat(list: List(T0 | List(T0)), depth?: Number) -> List(T0)
pub(crate) trait FlatAnyKernel {
    fn exec(
        list: &[Value],
        depth: &[f64],
        depth_present: &Mask,
        mask: &Mask,
    ) -> (Vec<Value>, Vec<(usize, EvalError)>);
}
```

### 7.5 Dispatch Wiring

Each trait gets a static wrapper function that unpacks `BuiltinArgs` into the
trait's typed parameters. The codegen generates one wrapper per trait. Each wrapper
uses `unsafe` unchecked accessors on `Column` (see Section 4) to extract typed slices
without branching in release builds. The planner guarantees the `Column` variant is
correct; `debug_assert!` catches violations in debug builds.

```rust
/// Dispatch wrapper for AbsAny.
/// Generated code: unpacks BuiltinArgs -> calls AbsAny impl.
fn exec_abs_any(args: &BuiltinArgs, mask: &Mask) -> BuiltinResult {
    // Safety: planner guarantees head[0] is Column::F64 (sig: value: Number)
    let value = unsafe { args.head[0].values.column.as_f64_unchecked() };
    let (data, errors) = <AbsAny as AbsAnyKernel>::exec(value, mask);
    BuiltinResult::f64(data, errors)
}

fn exec_contains_any(args: &BuiltinArgs, mask: &Mask) -> BuiltinResult {
    // Safety: planner guarantees head[0] is Column::Str, head[1] is Column::Str
    let text = unsafe { args.head[0].values.column.as_str_unchecked() };
    let search = unsafe { args.head[1].values.column.as_str_unchecked() };
    let (data, errors) = <ContainsAny as ContainsAnyKernel>::exec(text, search, mask);
    BuiltinResult::bool(data, errors)
}

fn exec_min_any(args: &BuiltinArgs, mask: &Mask) -> BuiltinResult {
    // Safety: planner guarantees each repeat group column is Column::Any
    //         (sig: values: Number | Number[]... -> union maps to Any)
    let repeat: Vec<&[Value]> = args.repeat_groups.iter()
        .map(|g| unsafe { g[0].values.column.as_value_unchecked() })
        .collect();
    let (data, errors) = <MinAny as MinAnyKernel>::exec(&repeat, mask);
    BuiltinResult::f64(data, errors)
}

fn exec_splice_any(args: &BuiltinArgs, mask: &Mask) -> BuiltinResult {
    // Safety: planner guarantees head types match signature
    let list = unsafe { args.head[0].values.column.as_value_unchecked() };
    let start_index = unsafe { args.head[1].values.column.as_f64_unchecked() };
    let delete_count = unsafe { args.head[2].values.column.as_f64_unchecked() };
    let repeat: Vec<&[Value]> = args.repeat_groups.iter()
        .map(|g| unsafe { g[0].values.column.as_value_unchecked() })
        .collect();
    let (data, errors) = <SpliceAny as SpliceAnyKernel>::exec(
        list, start_index, delete_count, &repeat, mask,
    );
    BuiltinResult::value(data, errors)
}
```

The static dispatch table ties everything together:

```rust
pub(crate) type BuiltinExecFn = fn(&BuiltinArgs, &Mask) -> BuiltinResult;

#[derive(Clone, Copy)]
pub(crate) struct BuiltinEntry {
    pub name: &'static str,
    pub exec: BuiltinExecFn,
    pub output_type: ColumnType,
}

pub(crate) static BUILTIN_REGISTRY: [BuiltinEntry; BUILTIN_COUNT] = [
    // -- General --
    BuiltinEntry { name: "if",        exec: exec_if_any,        output_type: ColumnType::Any  },
    BuiltinEntry { name: "ifs",       exec: exec_ifs_any,       output_type: ColumnType::Any  },
    BuiltinEntry { name: "empty",     exec: exec_empty_any,     output_type: ColumnType::Bool },
    BuiltinEntry { name: "length",    exec: exec_length_any,    output_type: ColumnType::F64  },
    BuiltinEntry { name: "format",    exec: exec_format_any,    output_type: ColumnType::Str  },
    BuiltinEntry { name: "equal",     exec: exec_equal_any,     output_type: ColumnType::Bool },
    BuiltinEntry { name: "unequal",   exec: exec_unequal_any,   output_type: ColumnType::Bool },
    // -- Text --
    BuiltinEntry { name: "substring", exec: exec_substring_any, output_type: ColumnType::Str  },
    BuiltinEntry { name: "contains",  exec: exec_contains_any,  output_type: ColumnType::Bool },
    // -- Math --
    BuiltinEntry { name: "abs",       exec: exec_abs_any,       output_type: ColumnType::F64  },
    BuiltinEntry { name: "ceil",      exec: exec_ceil_any,      output_type: ColumnType::F64  },
    BuiltinEntry { name: "floor",     exec: exec_floor_any,     output_type: ColumnType::F64  },
    BuiltinEntry { name: "sqrt",      exec: exec_sqrt_any,      output_type: ColumnType::F64  },
    BuiltinEntry { name: "min",       exec: exec_min_any,       output_type: ColumnType::F64  },
    BuiltinEntry { name: "max",       exec: exec_max_any,       output_type: ColumnType::F64  },
    // ... all 77 entries ...
];
```

### 7.6 Future Specialised Variant (Illustration)

In Phase 2+, the codegen adds specialised variants alongside `_Any`. Here is what
`abs_f64` would look like -- included now for clarity, but **not generated in Phase 1**:

```rust
// Added to BuiltinKey enum:
//   AbsF64 = 77,

/// Kernel trait for `abs` (_F64 specialization).
/// Selected by planner when input is known to be Number.
pub(crate) trait AbsF64Kernel {
    fn exec(
        value: &[f64],
        mask: &Mask,
    ) -> (Vec<f64>, Vec<(usize, EvalError)>);
}

// Note: the trait signature is identical to AbsAnyKernel in this case,
// because abs already takes f64. The difference is that the _F64 dispatch
// wrapper skips the Value->f64 extraction step in BuiltinArgs, since the
// planner guarantees the input column is already ColumnType::F64.
```

### 7.7 Impl File Structure

Hand-written implementations live alongside the generated code:

```
evaluator/src/builtins/
    mod.rs            -- pub(crate) re-exports
    generated.rs      -- generated by gen_builtins (do not edit)
    helpers.rs        -- map_f64_unary, map_str_binary, BuiltinArgs, BuiltinResult
    impls/
        mod.rs        -- re-exports all impl modules
        math.rs       -- AbsAny, CeilAny, FloorAny, SqrtAny, CbrtAny, ...
        text.rs       -- LowerAny, UpperAny, ContainsAny, SubstringAny, ...
        general.rs    -- EmptyAny, LengthAny, FormatAny, EqualAny, ...
        date.rs       -- NowAny, DateBetweenAny, ...
        list.rs       -- AtAny, SliceAny, SpliceAny, ConcatAny, FlatAny, ...
        people.rs     -- IdAny, NameAny, EmailAny
        special.rs    -- LetAny, LetsAny
```

Each impl file contains zero-boilerplate struct + trait impl pairs:

```rust
// evaluator/src/builtins/impls/math.rs

use crate::builtins::generated::{AbsAnyKernel, CeilAnyKernel, FloorAnyKernel, SqrtAnyKernel};
use crate::builtins::helpers::map_f64_unary;
use crate::core::errors::EvalError;
use crate::core::types::Mask;

pub(crate) struct AbsAny;
impl AbsAnyKernel for AbsAny {
    fn exec(value: &[f64], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_f64_unary(value, mask, |v| Ok(v.abs()))
    }
}

pub(crate) struct CeilAny;
impl CeilAnyKernel for CeilAny {
    fn exec(value: &[f64], mask: &Mask) -> (Vec<f64>, Vec<(usize, EvalError)>) {
        map_f64_unary(value, mask, |v| Ok(v.ceil()))
    }
}
// ... etc.
```

If a trait has no impl, the Rust compiler emits an error like:

```
error[E0277]: the trait bound `PadAny: PadAnyKernel` is not satisfied
  --> evaluator/src/builtins/generated.rs:412:5
```

This is the core safety property: the codegen and the type system together guarantee
that every builtin has exactly one implementation with the correct signature.

---

## 8. Compilation and Runtime Flow

### 8.1 Build-Time / Codegen Flow

```
                                   BUILD TIME
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                                                                         │
  │  builtins crate                                                         │
  │  ┌─────────────────────────────────────────────┐                        │
  │  │ Ty, FunctionSig, ParamShape, GenericParam,  │                        │
  │  │ FunctionCategory, SigResolver               │                        │
  │  │ normalize_union, resolve_flat                │                        │
  │  │ macro DSL: def_builtins!                     │                        │
  │  │ builtins_functions() -> Vec<FunctionSig>     │ ◄─── 77 signatures    │
  │  └────────────────────┬────────────────────────┘                        │
  │                       │                                                 │
  │            ┌──────────┴──────────┐                                      │
  │            ▼                     ▼                                       │
  │  ┌─────────────────┐   ┌────────────────────────────────┐               │
  │  │ analyzer crate   │   │ gen_builtins binary             │              │
  │  │ (imports Ty etc) │   │ evaluator/src/bin/gen_builtins  │              │
  │  └─────────────────┘   └──────────────┬─────────────────┘               │
  │                                       │                                 │
  │                                       │ reads builtins_functions()       │
  │                                       │ for each sig:                   │
  │                                       │   1. emit BuiltinKey variant    │
  │                                       │   2. emit per-function trait    │
  │                                       │   3. emit dispatch wrapper fn   │
  │                                       │   4. emit registry entry        │
  │                                       │ compute SHA-256 of all sigs     │
  │                                       │ write header + hash             │
  │                                       ▼                                 │
  │                        ┌───────────────────────────────┐                │
  │                        │ evaluator/src/builtins/        │                │
  │                        │   generated.rs  (written)      │                │
  │                        └────────────────┬──────────────┘                │
  │                                         │                               │
  │                 cargo build compiles     │ references                    │
  │                 generated.rs which       │                               │
  │                 imports from:            ▼                               │
  │                        ┌───────────────────────────────┐                │
  │                        │ evaluator/src/builtins/        │                │
  │                        │   impls/math.rs   (hand)       │                │
  │                        │   impls/text.rs   (hand)       │                │
  │                        │   impls/general.rs (hand)      │                │
  │                        │   impls/date.rs   (hand)       │                │
  │                        │   impls/list.rs   (hand)       │                │
  │                        │   impls/people.rs (hand)       │                │
  │                        │   impls/special.rs (hand)      │                │
  │                        │   helpers.rs       (hand)      │                │
  │                        └───────────────────────────────┘                │
  │                                                                         │
  │  If any impl is missing or has wrong signature -> compile error         │
  │  If sig hash drifts from builtins crate -> test failure                 │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
```

Step-by-step:

1. **`just gen-builtins`** invokes `cargo run --bin gen_builtins` in the evaluator crate.
2. The binary calls `builtins::builtins_functions()` to get all 77 `FunctionSig` values.
3. For each signature, it generates a `BuiltinKey` variant, a trait, a dispatch wrapper,
   and a registry entry. The type-mapping rules from the table in Section 3 drive
   parameter and return type selection.
4. It computes `SHA-256(serialized signatures)` and writes it into the file header.
5. The complete file is written to `evaluator/src/builtins/generated.rs`.
6. On the next `cargo build`, the Rust compiler verifies that every trait referenced
   in the dispatch wrappers has a concrete impl. Missing impls produce compile errors.
7. The `builtin_drift` test recomputes the hash and compares -- if someone adds a builtin
   to the `builtins` crate but forgets to re-run codegen, CI catches it.

### 8.2 Runtime Flow

```
                                  RUNTIME
  ┌─────────────────────────────────────────────────────────────────────────┐
  │                                                                         │
  │  Source text                                                            │
  │  "abs(prop(\"Score\")) + 1"                                            │
  │         │                                                               │
  │         ▼                                                               │
  │  ┌──────────────┐                                                       │
  │  │ analyzer      │                                                      │
  │  │  lexer        │ ──► tokens                                           │
  │  │  parser       │ ──► AST (Expr tree)                                  │
  │  │  inference    │ ──► TypeMap { expr_id -> Ty }                        │
  │  │  validation   │ ──► diagnostics (errors/warnings)                    │
  │  └──────┬───────┘                                                       │
  │         │ AST + TypeMap                                                  │
  │         ▼                                                               │
  │  ┌──────────────────────────────────────────────────────────┐           │
  │  │ planner                                                   │          │
  │  │                                                           │          │
  │  │  1. Walk AST, consult TypeMap for each subexpression       │          │
  │  │                                                           │          │
  │  │  2. For Call("abs", [Call("prop", ["Score"])]):            │          │
  │  │     a. "prop" -> ExecNode::Prop { name: "Score" }         │          │
  │  │     b. "abs"  -> check rewrite table: not found           │          │
  │  │     c. builtin_key_from_name("abs") -> AbsAny             │          │
  │  │     d. resolve_structured_args: head=[Prop], repeat=[], tail=[]     │
  │  │     e. output_type: F64 (from sig.ret = Number)           │          │
  │  │     f. -> ExecNode::Call { key: AbsAny, head: [Prop],     │          │
  │  │            repeat_groups: [], tail: [], output_type: F64 }│          │
  │  │                                                           │          │
  │  │  3. For Binary(+, Call(...), Lit(1)):                      │          │
  │  │     a. left = Call node (output_type: F64)                │          │
  │  │     b. right = LiteralF64(1.0)                            │          │
  │  │     c. select_binary_plan(+, F64, F64) -> AddF64, no cast │          │
  │  │     d. -> ExecNode::Binary { key: AddF64, left, right }   │          │
  │  │                                                           │          │
  │  └──────┬───────────────────────────────────────────────────┘           │
  │         │ ExecNode tree                                                 │
  │         ▼                                                               │
  │                                                                         │
  │  ExecNode::Binary {                                                     │
  │      key: AddF64,                                                       │
  │      left: ExecNode::Call {                                             │
  │          key: AbsAny,                                                   │
  │          head: [ExecNode::Prop { name: "Score" }],                      │
  │          repeat_groups: [],                                             │
  │          tail: [],                                                      │
  │          output_type: F64,                                              │
  │      },                                                                 │
  │      right: ExecNode::LiteralF64(1.0),                                 │
  │  }                                                                      │
  │         │                                                               │
  │         ▼                                                               │
  │  ┌──────────────────────────────────────────────────────────┐           │
  │  │ evaluator (eval_node, recursive dispatch)                 │          │
  │  │                                                           │          │
  │  │  Given: len=4 rows, mask=[true, true, true, true]         │          │
  │  │                                                           │          │
  │  │  eval Binary(AddF64):                                     │          │
  │  │    ├─ eval left: Call(AbsAny)                             │          │
  │  │    │    ├─ eval head[0]: Prop("Score")                    │          │
  │  │    │    │    └─ provider.get_prop("Score", 4, mask)       │          │
  │  │    │    │       => EvalBlock::F64([-3.0, 5.0, -2.0, 0.0])│          │
  │  │    │    ├─ args = BuiltinArgs { head: [that block], ... } │          │
  │  │    │    ├─ BUILTIN_REGISTRY[AbsAny].exec(&args, &mask)   │          │
  │  │    │    │    └─ exec_abs_any -> AbsAny::exec              │          │
  │  │    │    │       => (vec![3.0, 5.0, 2.0, 0.0], vec![])    │          │
  │  │    │    └─ EvalBlock::F64([3.0, 5.0, 2.0, 0.0])          │          │
  │  │    ├─ eval right: LiteralF64(1.0)                         │          │
  │  │    │    └─ EvalBlock::F64([1.0, 1.0, 1.0, 1.0])          │          │
  │  │    └─ dispatch_binary(AddF64, left, right, mask)          │          │
  │  │       => EvalBlock::F64([4.0, 6.0, 3.0, 1.0])            │          │
  │  │                                                           │          │
  │  └──────┬───────────────────────────────────────────────────┘           │
  │         │                                                               │
  │         ▼                                                               │
  │  EvalBlock::F64([4.0, 6.0, 3.0, 1.0])                                  │
  │  (returned to caller)                                                   │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘
```

### 8.3 If/Ternary Flow (Mask Splitting)

A more complex example: `if(prop("Active"), abs(prop("Score")), 0)`

```
  eval If node, len=4, mask=[T, T, T, T]
    │
    ├─ eval cond: Prop("Active")
    │    => EvalBlock::Bool([true, false, true, false])
    │
    ├─ split_mask_on_bool:
    │    then_mask = [T, F, T, F]   (mask AND cond)
    │    else_mask = [F, T, F, T]   (mask AND NOT cond)
    │
    ├─ eval then_branch with then_mask: Call(AbsAny, Prop("Score"))
    │    ├─ Prop("Score") with then_mask => [-3.0, ?, -2.0, ?]
    │    │  (rows 1,3 are masked out -- sentinel values, never read)
    │    └─ AbsAny => [3.0, ?, 2.0, ?]
    │
    ├─ eval else_branch with else_mask: LiteralF64(0.0)
    │    => [?, 0.0, ?, 0.0]
    │
    └─ merge_blocks(then, else, then_mask, else_mask)
       => EvalBlock::Any([3.0, 0.0, 2.0, 0.0])
       (each row picks from the branch whose mask was active)
```

### 8.4 Rewrite Flow

Example: `add(prop("A"), prop("B"))` is rewritten, never reaching dispatch.

```
  planner encounters: Call("add", [Prop("A"), Prop("B")])
    │
    ├─ get_rewrite("add") => Some(Alias(BinaryOp(Plus)))
    │
    ├─ apply_rewrite:
    │    left  = lower(Prop("A"))  => ExecNode::Prop { name: "A" }
    │    right = lower(Prop("B"))  => ExecNode::Prop { name: "B" }
    │    op    = BinOpKind::Plus
    │    plan  = select_binary_plan(Plus, inferred_ty_A, inferred_ty_B)
    │
    └─ ExecNode::Binary { key: AddF64, left: Prop("A"), right: Prop("B") }
       (no ExecNode::Call emitted -- uses existing binary kernel directly)
```

### 8.5 ifs Desugaring Flow

Example: `ifs(prop("X") > 0, "positive", prop("X") == 0, "zero", "negative")`

```
  planner encounters: Call("ifs", [Gt(Prop("X"), 0), "positive",
                                   Eq(Prop("X"), 0), "zero",
                                   "negative"])
    │
    ├─ get_rewrite("ifs") => Some(Transform(rewrite_ifs_to_nested_if))
    │
    └─ rewrite_ifs_to_nested_if:
         ExecNode::If {
             cond: Binary(GtF64, Prop("X"), LiteralF64(0)),
             then_branch: LiteralAny("positive"),
             else_branch: ExecNode::If {
                 cond: Binary(EqAny, Prop("X"), LiteralF64(0)),
                 then_branch: LiteralAny("zero"),
                 else_branch: LiteralAny("negative"),
             },
         }
```

---

## 9. Phase 1 Deliverables

Phase 1 delivers end-to-end function call execution for all 77 builtins using
`_Any` variants only. Specialised `_F64`/`_Str`/etc. variants are deferred to Phase 2.

### 9.1 Implementation Checklist

Each step is a single PR-sized unit. Steps within a group may be combined into one PR
when they touch the same files, but should remain separate commits.

#### Group A: `builtins` Crate Extraction

- [ ] **A1.** Create `builtins/` crate with `Cargo.toml` (no workspace deps).
- [ ] **A2.** Move `Ty`, `FunctionCategory`, `FunctionSig`, `ParamSig`, `ParamShape`,
      `GenericParam`, `SigResolver` from `analyzer/src/analysis/` into `builtins/src/`.
- [ ] **A3.** Move `normalize_union`, `resolve_flat`, `collect_leaf_types`,
      helper functions into `builtins/src/`.
- [ ] **A4.** Move the macro DSL (`def_builtins!`) and all 7 category modules
      (`general.rs`, `text.rs`, `math.rs`, `date.rs`, `people.rs`, `list.rs`, `special.rs`)
      into `builtins/src/builtins/`.
- [ ] **A5.** Expose `builtins_functions() -> Vec<FunctionSig>` as the public API.
- [ ] **A6.** Update `analyzer` to depend on `builtins` and re-export types for
      downstream consumers (`ide`, `analyzer_wasm`). Verify all existing analyzer tests pass.
- [ ] **A7.** Update `evaluator` `Cargo.toml` to depend on `builtins`.

#### Group B: Codegen Pipeline

- [ ] **B1.** Implement `evaluator/src/bin/gen_builtins.rs`: reads `builtins_functions()`,
      emits `BuiltinKey` enum, per-function traits, dispatch wrappers, and `BUILTIN_REGISTRY`.
- [ ] **B2.** Implement the Ty-to-Rust type mapping logic (the table from Section 3):
      `Number -> f64`, `String -> String`, `Boolean -> bool`, `Date -> i64`,
      union/generic/list -> `Value`, optional params -> `(column, presence_mask)`.
- [ ] **B3.** Implement SHA-256 drift detection: hash computation + header comment.
- [ ] **B4.** Add `just gen-builtins` recipe to the Justfile.
- [ ] **B5.** Run codegen, commit `generated.rs`. Verify it compiles (it will not link
      until impls exist -- that is expected; use `#[cfg(feature = "...")]` or stub impls
      to keep CI green).
- [ ] **B6.** Add `evaluator/tests/builtin_drift.rs` test that compares live hash
      against generated hash.

#### Group C: IR Extensions

- [ ] **C1.** Add `ColumnType` enum to `evaluator/src/ir/nodes.rs`.
- [ ] **C2.** Expand `Column` enum from `{ F64, Any }` to `{ F64, Bool, Str, Date, List, Any }`
      with 1:1 correspondence to `ColumnType`. Update `Column::len()` and any existing
      match arms.
- [ ] **C3.** Add safe accessors (`as_f64_slice() -> Option<&[f64]>`, etc.) and `unsafe`
      unchecked accessors (`as_f64_unchecked()`, etc.) with `debug_assert!` guards on
      `Column`. The unchecked variants use `std::hint::unreachable_unchecked()` in
      release builds for zero-branch extraction.
- [ ] **C4.** Add `UnaryExecKey` enum.
- [ ] **C5.** Extend `BinaryExecKey` with `ModF64`, `PowF64`, comparison keys (`EqAny`,
      `NeAny`, `LtF64`, `LeF64`, `GtF64`, `GeF64`), and logical keys (`AndBool`, `OrBool`).
- [ ] **C6.** Replace `CastToF64` with generalised `Cast { input, from, to }` node.
      Update all existing tests that reference `CastToF64`.
- [ ] **C7.** Add `ExecNode::Unary { key, operand }`.
- [ ] **C8.** Add `ExecNode::If { cond, then_branch, else_branch }`.
- [ ] **C9.** Add `ExecNode::Prop { name }`.
- [ ] **C10.** Add `ExecNode::Call { key, head, repeat_groups, tail, output_type }`.

#### Group D: Planner Enhancements

- [ ] **D1.** Fix `functions: vec![]` bug: pass `builtins::builtins_functions()` to
      `SemaContext` so type inference works for function calls.
- [ ] **D2.** Add `evaluator/src/planner/rewrites.rs` with `AliasRewrite`,
      `TransformRewriteFn`, `RewriteRule`, and the `get_rewrite()` match table.
- [ ] **D3.** Implement `rewrite_ifs_to_nested_if` transform function.
- [ ] **D4.** Implement `resolve_structured_args()` using `resolve_repeat_tail_used()`
      from builtins to split flat arg list into head/repeat_groups/tail.
- [ ] **D5.** Implement `ty_to_column_type()` mapping from analyzer `Ty` to `ColumnType`.
- [ ] **D6.** Extend `lower()` to handle `ExprKind::Ternary` -> `ExecNode::If`.
- [ ] **D7.** Extend `lower()` to handle `ExprKind::Unary` -> `ExecNode::Unary`.
- [ ] **D8.** Extend `lower()` to handle `ExprKind::Call` with the full pipeline:
      prop check -> rewrite check -> `lower_call()` with structured args.
- [ ] **D9.** Extend `lower()` to handle `ExprKind::MemberCall` by flattening
      receiver into args and routing through the same Call pipeline.
- [ ] **D10.** Extend `select_binary_plan()` for new `BinaryExecKey` variants
      (mod, pow, comparisons, logical).

#### Group E: Runtime Execution

- [ ] **E1.** Create `evaluator/src/builtins/helpers.rs` with `BuiltinArgs`,
      `BuiltinResult`, and row-iteration helpers (`map_f64_unary`, `map_f64_binary`,
      `map_str_unary`, `map_str_binary`, `map_any_unary`).
- [ ] **E2.** Extend `eval_node()` with arms for `Unary`, `If`, `Prop`, `Call`.
- [ ] **E3.** Implement `eval_if()` with mask splitting (`split_mask_on_bool`)
      and `merge_blocks`.
- [ ] **E4.** Implement `eval_call()` that evaluates arg nodes and dispatches
      through `BUILTIN_REGISTRY`.
- [ ] **E5.** Implement `eval_prop()` delegating to the `Provider` trait.
- [ ] **E6.** Implement unary kernels (`NegF64`, `NotBool`) and wire into
      `dispatch_unary`.
- [ ] **E7.** Implement new binary kernels (`ModF64`, `PowF64`, comparisons, logical)
      and wire into `dispatch_binary`.
- [ ] **E8.** Implement generalised `cast_block()` replacing the old `CastToF64` kernel.

#### Group F: Builtin Implementations (by category)

Each sub-step implements all builtins in a category. The compiler enforces
completeness -- until all traits in `generated.rs` have impls, the build fails.

- [ ] **F1.** `impls/math.rs` -- abs, ceil, floor, sqrt, cbrt, exp, exp2, exp10,
      ln, log2, log10, sign, round, min, max, sum.
- [ ] **F2.** `impls/text.rs` -- substring, contains, test, match, replace, replaceAll,
      lower, upper, repeat, link, style, unstyle, pad, trim, join, split.
- [ ] **F3.** `impls/general.rs` -- empty, length, format, equal, unequal, not, and, or.
      Note: `if`/`ifs` are handled by `ExecNode::If`, not dispatch. The generated
      `exec_if_any` / `exec_ifs_any` wrappers delegate to `eval_if` or are unreachable
      (since the planner rewrites them).
- [ ] **F4.** `impls/date.rs` -- now, timestamp, fromTimestamp, minute, hour, day,
      date, month, year, dateBetween, dateAdd, dateSubtract, dateRange, dateStart,
      dateEnd, formatDate, parseDate.
- [ ] **F5.** `impls/list.rs` -- at, first, last, slice, concat, sort, reverse, unique,
      includes, find, findIndex, filter, every, some, map, flat, splice, reduce.
- [ ] **F6.** `impls/people.rs` -- id, name, email.
- [ ] **F7.** `impls/special.rs` -- let, lets.

#### Group G: Testing and Validation

- [ ] **G1.** Drift detection test (`builtin_drift.rs`) passes.
- [ ] **G2.** Return type validation test (`return_type_check.rs`) passes for all 77 builtins.
- [ ] **G3.** End-to-end evaluator tests for representative formulas:
      - Simple calls: `abs(-5)`, `lower("HELLO")`, `length("abc")`
      - Composed calls: `abs(prop("X")) + 1`, `contains(lower(prop("Name")), "foo")`
      - Ternary/if: `if(prop("Active"), prop("Score"), 0)`
      - ifs desugaring: `ifs(prop("X") > 0, "pos", prop("X") == 0, "zero", "neg")`
      - Rewritten calls: `add(1, 2)`, `equal(prop("A"), prop("B"))`
      - Variadic: `min(1, 2, 3)`, `sum(prop("A"), prop("B"), prop("C"))`
      - Member calls: `"hello".contains("ell")`, `[1,2,3].length()`
      - Optional params: `substring("hello", 1)`, `round(3.14159, 2)`
      - Prop: `prop("Name")` with a mock Provider
- [ ] **G4.** All existing evaluator tests continue to pass (binary arithmetic, literals, casts).

### 9.2 Suggested PR Order

```
  A1-A7  builtins crate extraction        (foundation -- everything depends on this)
    │
    ├─► B1-B6  codegen pipeline            (can start once A7 lands)
    │     │
    ├─► C1-C10 IR extensions               (independent of codegen)
    │     │
    │     └──► D1-D10  planner             (needs both IR extensions and builtins crate)
    │            │
    │            └──► E1-E8  runtime        (needs planner + IR)
    │                  │
    │                  └──► F1-F7  impls    (needs runtime helpers + generated traits)
    │                        │
    └────────────────────────┴──► G1-G4  testing (needs everything)
```

Groups B and C are independent and can be developed in parallel.
Groups D and E are sequential (E depends on D).
Group F can be parallelised across contributors (one person per category file).

### 9.3 What Phase 1 Does NOT Include

These are explicitly deferred to Phase 2+:

- **Specialised variants** (`_F64`, `_Str`, `_Bool`): Phase 1 only generates `_Any`.
  The planner always selects the `_Any` variant. Specialised selection based on
  inferred types comes later.
- **SIMD / vectorised kernels**: Row-iteration helpers use scalar loops. Vectorisation
  is a future optimisation pass.
- **Lazy/short-circuit logical operators**: `&&`/`||` use eager evaluation in Phase 1.
  Mask-split short-circuiting is a Phase 2 optimisation.
- **Async / streaming evaluation**: All evaluation is synchronous and batch-oriented.
- **Error recovery / partial results**: Errors are collected per-row but do not
  halt evaluation. No "error-as-value" propagation -- a row either succeeds or
  appears in the error list.
- **Provider implementation**: Only the `Provider` trait and its integration point
  (`eval_prop`) are implemented. Concrete providers (Notion API, mock data) are
  separate work.

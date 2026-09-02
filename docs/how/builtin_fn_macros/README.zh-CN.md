---
doc_id: how.builtin-fn-macros
title: "Builtin 声明 macro 如何报告局部错误"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Builtin 声明 macro 如何报告局部错误

[English](README.md)

本文面向修改 DSL parser、validation 或 expansion 的维护者，说明
`builtin_fn_macros` 如何把一次 category DSL invocation 变成 `BuiltinCategory`
expression，同时保留有用的编译期 diagnostic。

这个 crate 负责 declaration-local parsing 与 validation，不负责生产 catalog、
cross-category invariant、signature resolution 或 evaluator code generation。生成后的
semantic model 由 [`builtin_fn` guide](../builtin_fn/README.zh-CN.md) 说明。

## 一次 invocation 只走一条 parse-to-expression 流程

[`builtin_fn_macros/src/lib.rs`](../../../builtin_fn_macros/src/lib.rs) 中的
`builtin_functions!` entry point 先解析 `CategoryDecl`，再交给 expansion，最终返回
`BuiltinCategory` 类型的 expression：

```text
token stream
    |
    v
CategoryDecl parse -- recover malformed functions
    |
    v
local validation -- collect independent errors
    |
    +-- errors --> compile_error! block, no category
    |
    v
BuiltinCategory expression
```

macro 不生成 surrounding function，也不决定 item visibility。普通 Rust 代码负责
category function，并决定返回值如何进入完整 catalog。

## Parser 只在稳定边界恢复

[`builtin_fn_macros/src/ast.rs`](../../../builtin_fn_macros/src/ast.rs) 先解析 category
header，再逐个尝试 function。每次尝试都在 forked `syn` parse stream 上执行；成功时才
推进原始 stream，失败时原始 cursor 仍停在 declaration 起点。

Function syntax error 出现后，recovery 会消费 token，直到下一个 top-level `;`。Nested
delimiter group 在这里是单个 token tree，其中的 punctuation 不会被误当成恢复边界。
Recovery 刻意不在已损坏的 function 内继续解析。这样既能让后续 declaration 报告独立
错误，也无需猜测错误声明原本的嵌套方式。

Parse error 与成功解析的 function 会一起保存在 `CategoryDecl` 中。因此，validation
既能看到 syntax failure，也能处理 recovery 安全保留下来的全部 declaration。

如果 category header 本身无法解析，`syn::parse_macro_input!` 会立即返回错误；此时没有
可供 expansion 继续工作的 category boundary。

## 校验完成后才执行 lowering

[`builtin_fn_macros/src/expand.rs`](../../../builtin_fn_macros/src/expand.rs) 先校验完整的
local AST，成功后才开始 lowering。Local validation 覆盖：

- supported declaration 中已知的 category、generic kind 与 type；
- invocation 内重复的 function、generic 与 parameter name；
- optional-parameter ordering 和 repeat shape；
- attribute syntax，以及 `#[resolver]` 与 `#[unsupported]` 的冲突组合；
- `#[unsupported]` declaration 必须提供的说明；
- snake-case 与 keyword normalization 之后仍须唯一的 Rust field name。

校验成功后，expansion 使用绝对 `::builtin_fn` path 构造 `BuiltinCatalogEntry`、
`FunctionSig`、`ParamShape`、generic ID、type node 和 canonical presentation string。
Unsupported declaration 只得到 catalog metadata，不生成 `FunctionSig`。

macro 可以解析 resolver path，但 path 是否存在、类型是否为 `SigResolver`，由 Rust
compiler 校验。Proc macro 不重复实现 name resolution 或 Rust type checking。

## Diagnostic 保持局部，并指向可修改的 token

独立的 `syn::Error` 会组合成多个 `compile_error!` expansion。一次 invocation 最多发出
32 条 diagnostic；发现更多错误时，最后一条保留的 diagnostic 会报告还有多少错误被
suppressed。

Primary span 指向作者能够直接修正的 construct：

| 错误 | Primary span |
| --- | --- |
| 未知 generic kind 或 type | 未知 identifier |
| duplicate function name | 后一个 declaration，并在第一个 declaration 处补充一条错误 |
| duplicate generic 或 parameter name | 后一个 declaration |
| 非法 repeat minimum | integer literal |
| 非法或重复的 repeat layout | `repeat` keyword 或有问题的 member name |
| unsupported declaration 缺少说明 | `#[unsupported]` |
| attribute 冲突或格式错误 | 有问题的 attribute |
| function syntax error | semicolon recovery 之前由 `syn` 报告的 token |

只要存在任何 parse 或 validation error，expansion 就不会返回 partial
`BuiltinCategory`。由于这个 macro 出现在 expression position，proc-macro entry point
会在一个 block 内发出组合后的 diagnostic。

[`builtin_fn/tests/ui/fail/`](../../../builtin_fn/tests/ui/fail/) 下的 compile-fail snapshot
固定了 recovery、validation、resolver type checking、diagnostic span 和 error limit。
这些 fixture 放在 `builtin_fn`，因为展开后的代码需要引用该 crate 的 support type。

## 全局不变量留在 macro 之外

一次 invocation 只能比较同一 category 内的 declaration。它看不到其他 macro expansion、
category function 的组合顺序，也看不到最终 supported registry。因此，cross-category
name uniqueness、category order、support status 与 whole-catalog inclusion 由遍历
`builtin_categories()` 的
[`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs) 校验。

这种分工让 diagnostic 靠近 declaration token，同时不假设 proc macro 拥有 repository-wide
visibility。macro 不读取 repository file，不渲染 Markdown catalog，不遍历 formula AST，
也不生成 evaluator runtime behavior。

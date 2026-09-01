---
doc_id: specs.builtin-functions
title: "Builtin function 规格"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Builtin function 规格

[English](README.md)

Builtin function 为公式语言提供 text、number、date、list、control flow 和 row context 等操作。本文说明公式作者和集成方可以依赖的共同行为，但不取代定义可用函数的源码声明。

## Rust 声明是唯一 catalog

[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs) 是 builtin 名称与 signature 的唯一完整 catalog。声明顺序保持稳定，构造受支持的 callable set 时也沿用这一顺序。文档不得复制或渲染第二份完整清单。

带有 `#[unsupported]` 的声明不会进入 callable set。调用这类名称与调用其他未知函数的可观察结果相同：analysis 报告 unknown function。公式接口中没有单独的 unsupported-function 类别。

受支持的声明共同决定 analysis、editor assistance 和 evaluation 中公式能够看到的 callable set。只有受支持的声明才属于当前公式接口；Rust 中出现一个声明，并不表示这里承诺提供所有语法相似的上游函数。

## Call 必须符合声明的 argument shape

受支持的函数可以使用 `length(value)` 这样的普通 call，其中一部分也支持 `"hello".substring(1)` 这样的 postfix syntax。Postfix call 等价于把 receiver 放入第一个 argument slot。只有 signature 能确定第一个 slot，并且 receiver 之后仍有可写入的 argument position 时，才支持这种形式。不符合条件的 member call 即使能够被 parser 接受，也不会被静默转换为普通 call。

Signature 可以组合 fixed parameter、optional parameter、重复 parameter group，以及重复组之后的 tail。Call 必须先满足整体 shape；argument 太少、太多或重复组不完整时，会先产生 argument-shape diagnostic，之后才会检查各 argument 的 type。

部分 parameter 会绑定 generic type、接受 union type，或表示 list 与 binding 操作使用的 implicit function。同一次 call 会复用相同的 type binding，因此已经观察到的 argument type 可以进一步约束后续 parameter 和 result type。[`builtin_fn/src/resolution.rs`](../../../builtin_fn/src/resolution.rs) 是这些共用 shape 与 type 规则的实现入口。

## Analysis 先于 evaluation，但不能预判 row outcome

Semantic analysis 会按照解析后的 signature 检查已知 argument type。Best-effort analysis 可以为未完成的 source 返回有用的 type 和 diagnostic，但受支持的 evaluation 只从通过 syntax 与 semantic validation 的公式开始。在 call 内部，未知 value 或仍包含 `unknown` 的 type 会保持 indeterminate，而不是立刻成为 type mismatch。即使 call 通过 analysis，个别 row 仍可能在 runtime 失败。

Shape error 的优先级高于 argument type error。Shape 有效后，已知且不兼容的 argument 才会产生 type diagnostic。Diagnostic 文本用于说明问题，不是 machine-readable compatibility key。

Analyzer 在 [`analyzer/src/analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs) 中应用这些规则。公式 syntax 和通用 operator behavior 由 formula-language spec 维护，不属于本文。

## Evaluation 可以是 eager，也可以是 controlled

普通 value function 会先求值并 materialize 当前 active arguments，再执行函数。Controlled function 接收尚未求值的 plan，并按 active row 选择 conditional branch、binding body 或 callback application。

List expression 会先为每个 active row 完整构造，然后 controlled list function 才应用 callback，因此构造 list 时的 error 不会被隐藏。Callback 只应用于 active 的 row-element pair。`find`、`findIndex`、`some` 和 `every` 可以在结果确定后停止处理后续 element；其他 callback-based function 不承诺这种提前停止。真正被跳过的 branch 或 callback application 不会产生可观察 error。

Null 与 failure behavior 由各 function family 分别定义，不能假定所有 builtin 都遵循统一的 null propagation。例如，有些函数把 null 解释为空输入，而普通 typed operation 在必需 value 为 null 时通常返回 null。无效 regex、date、numeric domain 或其他 argument 可能让对应 row 失败。

Runtime failure 以 row 为边界。一个 active row 失败不会使其他成功 row 一起失败；controlled evaluation 跳过的表达式也不会产生 error。Value 与 controlled execution path 的实现入口分别是 [`evaluator/src/kernels/value.rs`](../../../evaluator/src/kernels/value.rs) 和 [`evaluator/src/kernels/controlled.rs`](../../../evaluator/src/kernels/controlled.rs)。

## 时间与 row identity 来自同一次 evaluation

`now()` 读取本次 evaluation 冻结的 timestamp。`today()` 使用同一 timestamp 和配置的 timezone offset 计算当地午夜。同一次 evaluation 中的重复调用因此共享同一 clock snapshot。

`id()` 以 text 返回当前 row identifier。它不是 formula identifier，也不建立 formula identity、persistence 或 rename behavior。

## Contract 边界

本文维护的是公式能够观察到的 call 与 evaluation behavior，不包括：

- generated Markdown catalog 或重复的名称与 signature 清单；
- declaration DSL、procedural macro、signature projection structure 和 generated evaluator ABI；
- Rust `pub` API 的稳定性保证；
- 与 Notion 或其他上游系统逐项一致的承诺；
- 仅仅因为存在 catalog fixture 而推导出的穷尽性 failure 保证。

代表性的 catalog invariant 位于 [`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs)。各函数的 runtime example 位于 [`evaluator/tests/builtins/`](../../../evaluator/tests/builtins/)，但这些 fixture 不会把未写入规格的 failure case 自动变成兼容性承诺。

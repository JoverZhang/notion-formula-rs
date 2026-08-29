---
doc_id: architecture.evaluator
title: "公式求值如何跨越输入预准备边界"
language: zh-CN
source_language: en
counterpart: ./evaluator.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-29
---

# Evaluator 设计

[English](evaluator.md)

本文描述 Current 实现，回答一个核心问题：经过语法分析的公式如何完成语义分析与准备，并变成同步的行批次
结果，以及数据加载、输入契约、null 和行级错误的职责在哪里分开。主要读者是需要理解准备、执行和失败
行为的 evaluator 维护者与 Rust 集成方。

本文范围从已完成语法分析的表达式和调用方持有的 schema 开始，到 `EvalBlock` 结束。它不定义外部数据
获取，也不逐一解释内置函数语义。当前实现参见 [`evaluator/README.md`](../../evaluator/README.md)，由内置函数
声明生成的 trait、类型化 ABI 和输入清单参见 [`builtin-fn.md`](builtin-fn.md)。

## 目标

准备阶段执行语义分析，并把最终 `SemanticMap` 降级为自包含的执行计划。runtime 随后接收调用方预先准备的
类型化列和行掩码，同步执行 IR，并返回逐行值、null 状态和错误。

## 流水线

```text
parsed expression + schema
          |
          v
 Semantic Analysis --> SemanticMap
          |
          v
      Planner
          |
          +--> PreparedFormula
          |      +-- ExecPlan
          |      +-- RequiredColumn[]
          |
          +-----------------------------+
                                        |
caller loads complete columns from      |
RequiredColumn                          |
          |                             |
          v                             |
 EvalInputsBuilder --validate----------+
          |
          v
      EvalInputs
          |
          v
PreparedFormula::evaluate (synchronous)
          |
          +--> IR walker
          +--> generated builtin dispatch
          +--> handwritten kernels
          |
          v
       EvalBlock
          +-- values
          +-- validity
          +-- ok
          +-- row errors
```

阅读时从顶部的已解析表达式开始，经过调用方准备阶段，再进入同步执行。该图强调所有权分界和结果状态，
有意省略具体 IR 节点、内置函数算法和外部 I/O 调度。

## 核心类型

| 类型 | 作用 |
| --- | --- |
| `PreparedFormula` | 已解析的执行计划和完整输入依赖 |
| `RequiredColumn` / `InputSlot` | 调用方必须准备的属性列及其计划局部槽位 |
| `EvalInputsBuilder` | 收集输入并根据已准备布局执行校验 |
| `EvalInputs` | 已完成定型、可用于同步求值的完整输入列 |
| `InputContractError` | 缺列、槽位重复，或 kind、长度、布局错误 |
| `KernelColumn<K>` | 具有共享所有权的类型化运行时列 |
| `Validity` | `AllValid`、`AllNull` 或共享位图 |
| `Mask` | 当前控制流步骤必须执行的行 |
| `EvalBlock` | 列、有效性、`ok` 和逐行错误 |
| `BuiltinEvalContext` | Controlled 内置函数在掩码下执行类型化计划的同步接口 |

## 契约

- Planner 使用 Analyzer 保存的 `ResolvedFunctionSig`，不会重新绑定泛型，也不会根据批次值执行签名 resolver。
- `PreparedFormula::required_columns()` 返回公式静态引用属性的完整去重清单，包括只出现在最终未选中分支里的引用。
- 调用方在求值前准备所有列。外部数据源可以异步加载，但 evaluator 内没有 Provider、Future 或 `block_on`。
- 任何 kernel 开始前，`EvalInputsBuilder` 会校验槽位、ABI kind、批次长度和布局。失败时返回
  `InputContractError`，不会产生部分结果。
- 执行掩码、`ok` 和 `Validity` 是相互独立的状态。null 是成功值；未激活行也不等同于 null。
- `ok[i] = false` 表示该行的物理值只是占位符，下游 kernel 不得读取它。
- `if`、`ifs`、`&&`、`||` 和 lambda 内置函数保持基于执行掩码的惰性。预加载完整列不允许系统提前执行未选中的表达式。
- 每个受支持的内置函数都使用生成的 trait、marker 和 dispatch binding。缺少实现或 ABI 不匹配会导致编译失败。

## 为什么使用 IR

- IR 把语义分析已经确定的内置函数、类型和参数形状固定为可执行节点。
- Planner 可以选择类型化列的特化实现，kernel 无需再对动态 `Value` 做顶层匹配。
- 执行掩码可以自然地沿控制流分支传播。
- `PlanId` 和命名的 Args/Plans 只向 Controlled 内置函数暴露受限的求值接口。
- 属性输入节点直接引用 `InputSlot`，避免运行时字符串查找。

## 为什么由调用方准备列

外部数据获取和公式计算具有不同的调度与错误边界。调用方可以根据 `RequiredColumn` 清单并发加载数据库
或接口数据，然后一次性构造 `EvalInputs`；evaluator 只负责同步、确定性的列计算。

这个边界意味着：只在未选中分支中引用的列仍可能被加载，但该分支表达式本身不会执行。作为交换，kernel
ABI 可以保持同步，异步 trait 不会扩散到 evaluator 内部，输入契约也保持显式。

## 错误边界

| 类别 | 表示方式 | 作用范围 |
| --- | --- | --- |
| 准备错误 | `PrepareError` | 在可执行计划产生前返回 |
| 输入结构错误 | `InputContractError` | 求值开始前作用于整个批次 |
| 行求值错误 | `EvalError` + `ok` | 单行 |
| Null | `Validity` | 有效的行值 |
| Kernel 契约错误 | Debug assertion | 开发阶段的实现错误 |

遇到 `PrepareError` 时，需要先修正表达式、schema 或不受支持的结构，再构造输入。`InputContractError`
只能通过修正或重新构造调用方持有的输入来恢复。此时尚未运行任何 kernel，因此没有需要合并或回滚的部分
结果。行级 `EvalError` 不会中止整个批次：不受影响的行继续执行，失败行通过 `ok` 和对应错误条目标记。
null 仍然是成功值。kernel debug assertion 表示实现违反契约，evaluator 不保证能从这种错误中恢复。

## 实现入口

- 当前 Planner：`evaluator/src/planner/`
- 当前 IR：`evaluator/src/ir/`
- 当前 runtime：`evaluator/src/runtime/`
- 当前 kernel：`evaluator/src/kernels/`
- 当前实现状态和已知差异：`evaluator/README.md`

## 运行时行为验证

受支持的内置函数行为由 `evaluator/tests/builtins/` 下覆盖完整目录的 golden fixture 验证。每个 fixture
都会跨越公开 evaluator 接口：使用生产环境的语法和语义分析、`prepare_formula`、必需列构造、
`EvalInputsBuilder` 和带执行掩码的行批次求值。fixture metadata 只负责提供调用方持有的属性列、row ID、
执行掩码和冻结的运行时上下文。

目录中的每个受支持声明都必须具有一个基线 fixture。只有在需要清楚表达重要边界或保护回归时才增加额外
场景。snapshot 展示准确源码和可观察的逐行结果——值、null、错误或未激活——而不暴露物理占位符和
内部列存储。

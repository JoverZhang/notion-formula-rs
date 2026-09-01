---
doc_id: how.evaluator
title: "Evaluator 如何准备并执行公式"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Evaluator 如何准备并执行公式

[English](README.md)

`evaluator` crate 接收已解析的表达式和 schema，生成一份自有的执行计划，再用调用方准备好的列执行这份
计划。本文面向需要扩展或调试 evaluator 的维护者，重点解释同步求值边界、基于掩码的惰性控制流，以及
内置函数声明如何变成类型化 kernel 约束。

本文只描述 Current Rust 实现。公式语言行为和集成契约由 `docs/specs/` 维护；外部数据加载、语法解析和
完整内置函数清单不在本文范围内。

## 准备阶段把公式含义与行数据分开

求值分为两个阶段。准备阶段完成依赖语法和 schema 的工作，执行阶段只接收行数据和冻结的运行时快照。

```text
parsed Expr + EvalContext
          |
          v
 prepare_formula -- semantic analysis --> SemanticMap
          |
          v
       Planner -------------------------> PreparedFormula
          |                                  |
          |                                  +--> ExecPlan
          |                                  +--> RequiredColumn[]
          |                                           |
          |                                caller prepares columns
          |                                           |
          +-------------------------------------------v
                           EvalInputsBuilder::finish
                                      |
                                      v
              RowBatch + Mask + EvalInputs --> Runtime --> EvalBlock
```

阅读时从顶部的已解析表达式向下，直到最终逐行结果。左侧完成语义工作并冻结计划，右侧是由调用方负责的
输入边界。图中有意省略语法解析、数据获取和单个运算符算法。

[`prepare_formula`](../../../evaluator/src/planner/prepared.rs) 使用 `EvalContext` 中的属性和受支持的 Rust
内置函数注册表执行语义分析。只要产生语义诊断，准备过程就会在创建 `PreparedFormula` 之前终止。成功
时，最终 `SemanticMap` 已保存表达式类型和每个内置调用的解析后签名。Planner 不会根据批次值再次推断
泛型，也不会重新调用自定义签名 resolver。

[`Planner`](../../../evaluator/src/planner/planner.rs) 遍历表达式，将它降级为自有的 `ExecPlan`。每个
`PlanId` 都指向不可变节点，节点可以表示字面量、输入、运算符、cast、变量、列表、分支或内置函数调用。
lambda body 和 thunk 也会变成计划；runtime 不持有 analyzer AST 引用。Planner 在降级时插入必要的 ABI
cast，让运行时 dispatch 直接处理类型化列。

### 必需列属于本次准备产生的布局

Planner 遇到 `prop("Name")` 时，会通过 `EvalContext` 找到属性，分配 `InputSlot`，并记录带有属性名和
语义类型的 `RequiredColumn`。同名引用复用同一 slot。因此，必需列清单完整、去重，并按首次出现顺序
排列；即使某个属性只出现在特定行不会执行的分支里，它也会进入清单。

`InputSlot` 同时保存 index 和不透明的布局身份，只能用于创建它的 `PreparedFormula`。调用方可以并发
加载这些列，也可以自行选择其他调度方式，但必须在进入 evaluator 的逐行执行之前完成。这个 crate
内部没有 provider，也没有异步执行边界。

[`EvalInputsBuilder`](../../../evaluator/src/core/inputs.rs) 收集已准备列和 `BuiltinRuntimeContext`。
`finish` 会确认每个 slot 恰好出现一次、属于同一份准备布局、物理 ABI kind 正确，并检查批次长度和
validity 长度。通过校验后，列被固定为 `EvalInputs`。之后调用 `evaluate` 时，还会再次确认 inputs 属于
当前准备布局，并确认 inputs、`RowBatch` 和可选执行掩码的长度一致。

## Runtime 在执行掩码下遍历计划

`PreparedFormula::evaluate` 使用覆盖全部行的掩码；`evaluate_with_mask` 则接收调用方指定的掩码。
[`Runtime`](../../../evaluator/src/runtime/evaluator.rs) 在该掩码下递归执行根节点。普通运算符先物化输入，
再调用类型化 operator helper。`&&`、`||`、三元表达式和 Controlled 内置函数会先计算更窄的掩码，
然后才访问后续 operand 或分支。

虽然所有属性列已经加载，这个区别仍然重要：准备阶段会完整收集依赖，但执行阶段仍按需计算。未选中分支
可以贡献 `RequiredColumn`，它的节点却不会运行，也不会产生行级错误。

Lambda 执行使用一组按栈管理的“名称到列”scope。`LambdaPlan` 只保存计划 owner、节点 ID、参数名和
debug contract。`Runtime::apply_lambda` 先校验 handle，再压入绑定，在元素掩码下执行 body，最后移除
scope。来自另一份计划的 handle 会被拒绝，不会被继续解析。

运行时快照同样是显式输入。`EvalInputs` 持有一份 `BuiltinRuntimeContext`，其中冻结了求值时间和时区
offset；`RowBatch` 则持有 row ID。Value kernel 通过 `BuiltinKernelContext` 读取这些数据，因此
`now()` 和 `today()` 在同一次求值中看到同一时钟，`id()` 读取对应行的身份。

## 列同时维护三种彼此独立的行状态

Evaluator 分开记录一行是否执行、是否成功，以及是否为 null：

| 表示 | 回答的问题 | `false` 的含义 |
| --- | --- | --- |
| 执行 `Mask` | 当前节点是否应在这一行运行？ | 物理 slot 未激活 |
| `EvalBlock.ok` | 这一行是否求值成功？ | slot 只是占位符，不得读取 |
| `Validity` | 成功行是否含有值？ | 该行为 null |

Null 不是错误，未激活行也不是 null。未激活的物理 slot 会被规范化为成功、非 null 的占位符，避免存储
状态泄露控制流信息；只有执行掩码决定这些占位符是否可观察。

[`Column`](../../../evaluator/src/core/columns.rs) 为 number、boolean、text、date、list 和动态 `Value`
数据提供不同物理 variant。每种 variant 都持有类型化 `KernelColumn<K>`，其中包含 `SharedStorage` 和
独立的 `Validity`（`AllValid`、`AllNull` 或共享 bitmap）。克隆 kernel column 时只会克隆由 `Arc`
支持的存储 handle，不会复制行数据。如果 kernel 独占存储，可通过 `try_into_unique` 取回 buffer；
`abs()` 会用这条路径原地计算，存储已共享时才分配新结果。

`EvalBlock` 把物理 `Column`、`ok` 掩码和带行 index 的 `EvalError` 组合在一起。Null、失败或未激活 slot
中的物理值都是实现占位符，kernel 必须先检查相应状态，才能读取它。

## 内置函数声明会生成 evaluator ABI

完整的内置函数清单只维护在
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs)。编译期间，
[`evaluator/build.rs`](../../../evaluator/build.rs) 读取受支持声明，并把确定性的 Rust contract 写入
`OUT_DIR`。不受支持的声明会被过滤，不会给 evaluator 生成实现义务。

对每个受支持声明，[`build_support.rs`](../../../evaluator/build_support.rs) 中的生成器会产生：

- `BuiltinKey` 条目，以及对应的求值模式和返回 ABI；
- marker 类型和该函数专属的 kernel trait；
- 命名的类型化 `Args` 或 `Plans`，包括重复参数组结构；
- 解码参数并通过 trait 调用 marker 的 dispatch arm。

生成代码不包含实现 body。手写 trait 实现集中在
[`builtins/implementations.rs`](../../../evaluator/src/builtins/implementations.rs)，再委托给可复用 kernel。
由于生成 dispatch 会引用每个 marker 和 trait，缺少实现或方法签名不兼容都会导致编译失败。

Planner 复用 `SemanticMap` 中每个调用的最终解析签名，为参数分配逻辑 `ParamRef` 和 repeat-group number。
[`builtins/support/arguments.rs`](../../../evaluator/src/builtins/support/arguments.rs) 中共享的
`ArgumentPool` 随后按这两个字段执行破坏性读取，并报告参数缺失或重复。Dispatch 不会再次解析参数形状
或泛型。

### Value 模式在 dispatch 前物化参数

如果声明的顶层参数不包含 `Fn` 或 `Ident`，生成器会产生 Value contract。Runtime 在当前掩码下执行
所有参数计划，保留上游行错误，并将各参数的 `ok` 掩码相交，得到 kernel 可以处理的行。这个过程不会
与 `Validity` 相交：`empty()` 之类理解 null 的函数仍须看到 null。

生成 decoder 会把类型化 `KernelColumn<K>` handle 移入函数专属的命名 `Args`，不会复制行 buffer。
手写 kernel 根据自身语义选择计算路径：纯且不会失败的操作可以计算所有物理 slot；可能失败的操作只处理
eligible 且非 null 的行；理解 null 的操作则显式接收 null 行。函数签名无法推导这种选择，因此它仍由
[`kernels/`](../../../evaluator/src/kernels/) 下的实现决定。

### Controlled 模式把所有参数保留为计划

只要有一个顶层参数是函数或 identifier，生成器就会选择 Controlled 模式，并让全部参数保持未求值。
只延迟 lambda 参数并不够：以 `ifs()` 为例，如果某行已经匹配前一个条件，后续普通 boolean 条件也不能
再执行。

生成字段分别成为类型化 `ValuePlan`、`ThunkPlan`、`LambdaPlan` 或 `BinderHandle`。Kernel 除了这些
handle，还会收到受限的 `BuiltinEvalContext`，可以在指定掩码下执行计划、带绑定调用 lambda，或拆分
条件掩码，但不能访问 analyzer AST。
[`kernels/controlled.rs`](../../../evaluator/src/kernels/controlled.rs) 通过这套接口处理分支选择、binder 和
逐元素列表求值；短路过程会随着工作完成不断收窄 active mask。

Plan handle 带有 owner token，因此 Controlled kernel 不会误执行另一份 `ExecPlan` 的节点。生成的计划
结构也不携带 AST 借用 lifetime；dispatch 通过泛型 context 静态完成，而不是使用 `dyn` trait object。

## 失败会停在负责该问题的边界

| 失败 | 表示方式 | 影响 |
| --- | --- | --- |
| 语义分析或降级失败 | `PrepareError` | 不返回 `PreparedFormula` |
| 输入缺失、重复、kind 错误、长度错误或布局不匹配 | `InputContractError` | kernel 开始前拒绝整个操作 |
| 单行公式或数据失败 | `EvalError`，同时 `ok[row] = false` | 其他 eligible 行继续执行 |
| Null 结果 | `Validity` | 该行成功，但没有值 |
| 生成 ABI 不匹配 | Rust 编译错误 | evaluator 实现无法构建 |
| 运行时实现契约被破坏 | debug assertion | 表示 evaluator bug；不承诺恢复方式 |

整项操作级失败不会产生部分 `EvalBlock`，因此没有需要合并或回滚的结果。行级失败保留带 index 的错误，
物理存储只放占位符；下游通过 `ok` 排除这些行。Controlled 执行不会合并未被掩码执行的分支或后续
predicate 的错误。

生成 trait 和 Rust 类型保护静态 ABI。Debug build 还会在参数物化和 dispatch 边界，检查 active、成功、
非 null 行的解析后参数类型和返回类型。这些 assertion 用于定位实现错误；release 模式下的数据错误仍须
转换成普通 `EvalError`，不能依赖 assertion。

## 继续阅读源码

- 准备过程和输入布局：[`planner/prepared.rs`](../../../evaluator/src/planner/prepared.rs)、
  [`planner/planner.rs`](../../../evaluator/src/planner/planner.rs) 和
  [`core/inputs.rs`](../../../evaluator/src/core/inputs.rs)。
- 带掩码的 IR 执行：[`runtime/evaluator.rs`](../../../evaluator/src/runtime/evaluator.rs) 和
  [`runtime/operators.rs`](../../../evaluator/src/runtime/operators.rs)。
- 类型化存储和行状态：[`core/columns.rs`](../../../evaluator/src/core/columns.rs) 与
  [`core/types.rs`](../../../evaluator/src/core/types.rs)。
- 生成和手写内置函数的交界：[`build_support.rs`](../../../evaluator/build_support.rs)、
  [`builtins/support/arguments.rs`](../../../evaluator/src/builtins/support/arguments.rs) 和
  [`builtins/implementations.rs`](../../../evaluator/src/builtins/implementations.rs)。
- 代表性不变量测试：[`runtime_structure.rs`](../../../evaluator/tests/runtime_structure.rs) 和
  [`generated_contract.rs`](../../../evaluator/tests/generated_contract.rs)。

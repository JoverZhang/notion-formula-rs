---
doc_id: architecture.contracts
title: "跨 crate 调用方可以依赖哪些不变量？"
language: zh-CN
source_language: en
counterpart: ./contracts.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# 跨 crate 契约

[English](contracts.md)

本文是一份 Current 状态的文档，回答公式数据跨越 `analyzer`、`ide`、`analyzer_wasm` 和
`evaluator` 时，Rust 与 JavaScript 调用方可以依赖哪些不变量。它面向需要修改或调试这些边界、
同时又不能意外改变可观察行为的维护者和集成方。

本文从源码坐标、token 和错误恢复后的语法树开始，继续说明诊断、编辑操作、签名帮助、WASM 门面和
evaluator 的已准备输入。单个 builtin 的语义和模块内部 API 不在范围内。这里的每条规则都是兼容性
接口：若要改变，必须同步修改测试、双语文档和 changelog；只移动内部 helper、且不改变规则，则不算
契约变化。

## 边界地图

| 边界 | 调用方提供 | 调用方获得 | 失败范围 |
| --- | --- | --- | --- |
| `analyzer` | UTF-8 公式源码 | token、可恢复的 AST、诊断和字节 span | 问题表示为诊断；可恢复的语法错误仍会产生 AST |
| `ide` 编辑操作 | 源码以及字节坐标的 cursor 或编辑 | 更新后的源码和重定位后的字节 cursor | 非法输入或格式化失败会拒绝整个操作 |
| `analyzer_wasm` | JavaScript 配置、源码和 UTF-16 编辑器坐标 | span、编辑和 cursor 均使用 UTF-16 的 DTO | 构造器和编辑操作错误以错误形式跨越 WASM 边界 |
| `evaluator` | 已分析表达式、schema、行批次和完整的类型化输入 | 包含值、有效性、行成功状态和行错误的 `EvalBlock` | 准备与输入错误拒绝整个操作；求值错误只影响对应行 |

这张表描述的是职责归属，不是另一份流水线说明。下文会定义让这些交接保持安全的坐标转换、错误恢复保证和
失败边界。

## 只在 WASM 边界转换坐标

在 Rust 的 analyzer 和 IDE 层中，所有源码 `Span`、`TextEdit.range` 和 cursor 均以 UTF-8 字节
计量。区间采用半开形式 `[start, end)`，两个端点都必须是同一份源码中的有效字符边界。因此，有效
`Span` 可以直接用 `&source[start..end]` 切片对应源码。

`SourceMap::line_col` 是展示位置，不是第二套源码 offset。它返回从 1 开始的行号和列号，其中列按
Unicode scalar value（Rust `char`）计数，而不是按字节或 UTF-16 code unit 计数。诊断转换到
WASM 时，`Diagnostic.line` 和 `Diagnostic.col` 正是通过这种方式得到。

到了 JavaScript 边界，DTO span、编辑区间和 cursor 改用半开的 UTF-16 code unit 坐标。
`analyzer_wasm` 独占 UTF-16 坐标与 Rust 字节 offset 之间的转换；`analyzer` 和 `ide` 都不执行
UTF-16 转换。调用方不得用 DTO span 直接切片 UTF-8 字符串，也不得把 UTF-16 位置直接传给 Rust
API。

坐标转换的边界行为是确定的：

- 落在多单元 Unicode scalar 内部的位置会向下取整到该 scalar 的起点。
- 通用转换 helper（包括 `help` 使用的转换）会把超出源码末尾的位置截断到末尾。
- `format` 和 `apply_edits` 使用带校验的 cursor 转换，并拒绝超过 UTF-16 文档长度的 cursor。编辑转换
  还会拒绝反向区间以及 end 超出文档的区间；落在 surrogate pair 内部的端点仍会向下取整到 scalar
  起点。
- 核心 IDE 层会拒绝越界或不在 UTF-8 字符边界上的字节 cursor 和编辑端点。

这些规则保证转换过程不会 panic，但不意味着两套坐标可以混用。

## 局部错误之后，token 与语法结构仍然可用

lexer 按源码顺序产生 token，保留 `DocComment` 和 `Newline` 作为非语义 token，并在源码末尾追加一个
span 为空的显式 `Eof` token。普通空格不会成为 token。token range 同样采用半开形式，但它的单位是
token index，而不是字节。

`tokens_in_span` 把非空字节 span 映射到所有与其相交的 token。由于 `Eof` 的 span 为空，非空源码
span 不会包含它。空 span 或反向 span 会映射到一个稳定插入点：第一个起点不早于 `span.start` 的
token 之前；这个插入点可以位于 `Eof` index。`TokenQuery` 是 span 映射和感知非语义 token 的相邻
扫描的规范 API，因此 parser 和 formatter 代码不应重复实现 token index 运算。

parser 遇到局部语法错误时会插入 `ExprKind::Error` 节点并继续解析。因此，AST 消费方必须能够处理
`Error` 节点，不能因为存在诊断就假定不会返回语法树。消费方还必须处理 `ExprKind::ImplicitLambda`：
parser 从不创建这个节点，但语义分析可能针对函数类型参数插入它。成员语法只支持
`receiver.method(...)` 这一种形式；`receiver.member` 之类的裸成员访问会产生诊断，并恢复为错误
表达式。

## 诊断与编辑操作保持确定性

`Diagnostics` 对完全相同的 span 最多保留一条诊断。新诊断优先级更高时会替换已有诊断；优先级相同且
消息相同时，会合并并去重 label、note 和代码操作；优先级相同但消息不同时，则保留已有诊断。

`format_diagnostics` 依次按起点、终点、优先级降序和消息排序诊断；label 也会按起点、终点和 label
消息排序，note 则保留去重后的发出顺序。代码操作继续以 `Diagnostic.actions: Vec<CodeAction>` 的形式
附着在对应诊断上，每个操作包含使用核心字节坐标的 `TextEdit`。WASM converter 会保留代码操作结构，
同时把其中的编辑区间转换成 UTF-16。

公开编辑操作具有全有或全无的边界：

- `ide::apply_edits` 按原始源码坐标排序编辑，校验所有 cursor 和区间，拒绝重叠，然后应用完整编辑集合
  并重定位 cursor。
- `ide::format` 会拒绝包含 lexer 或 parser 错误的源码；否则，它会创建一个覆盖全文的替换编辑，并走
  同一条经过校验的字节编辑流水线。
- WASM `format` 和 `apply_edits` 先把输入转换为字节坐标，再调用 IDE 操作，最后把返回的 cursor 转回
  UTF-16。坐标转换或 IDE 失败会作为操作错误返回，而不会返回部分 `ApplyResult`。

## 编辑器帮助共享同一套语义签名模型

`builtin_fn` 中的 `ParamShape` 是规范参数模型，由 `head`、repeat group、`tail` 和
`repeat_min_groups` 组成；消费方必须使用共享的调用签名投影，不能自行重新推导可变参数或 tail 的位置。

签名帮助解析这套共享语义模型，并返回结构化 `DisplaySegment` 以及 `active_parameter`。WASM DTO 会原样
映射这些 segment，而不是把它们压平成一个展示字符串，最终渲染方式由调用方决定。详细的投影、活动参数和
后缀形式展示规则参见[签名帮助规范](../signature-help.md)；更完整的声明与解析模型参见
[Builtin Function Design](builtin-fn.md)。

## WASM 门面负责自己的配置边界

`Analyzer::new` 只接受 object，并拒绝未知的顶层 key。允许的 key 只有 `properties` 和
`preferred_limit`；JavaScript 不能提供 `functions`，因为构造器始终安装 Rust builtin 目录。

运行时反序列化会把缺省的 `properties` 当作空列表，并让缺省或为 `null` 的 `preferred_limit` 使用默认值
`5`。当前生成的 TypeScript DTO 则显式声明两个字段：
`{ properties: Property[], preferred_limit: number | null }`。因此，类型化集成应传入两个字段，尽管
运行时允许省略。object 结构或字段值非法时，构造会以 `Invalid analyzer config` 失败。

## 调用方完成输入契约后，求值才会开始

`prepare_formula` 先执行语义分析，再把表达式降级为自有的执行计划。准备失败时会返回 `PrepareError`，
且不会产生 `PreparedFormula`。成功后，`PreparedFormula::required_columns()` 会按首次出现顺序返回完整、
去重的必需列清单。每个 `RequiredColumn` 都包含属性名、预期类型以及仅在当前已准备输入布局中有效的
`InputSlot`；这个方法并不是返回一组裸 `InputSlot`。

同步求值开始前，调用方必须加载所有必需列，包括只在未选中分支中被引用的列。
`EvalInputsBuilder::finish` 会校验列是否缺失或重复、slot 布局、ABI kind、批次长度和 validity 长度。
结构不匹配时返回 `InputContractError`；如果输入、执行掩码或行批次的布局或长度不兼容，求值入口也会拒绝。
这些整项操作级失败都不会产生 kernel 结果。

求值期间，三种行状态保持相互独立：

- 执行掩码表示某个控制流步骤是否应在该行运行；
- `EvalBlock.ok` 记录该行是否求值成功；
- column `Validity` 记录成功行是否包含非 null 值。

当 `ok[i]` 为 false 时，该行的物理值只是占位符，下游 kernel 不得读取。对应 `EvalError` 只影响该行，
其他行仍可完成。`if`、`ifs`、`&&`、`||` 和 lambda builtin 等由执行掩码驱动的控制流，只会为确实需要的
行求值对应分支或参数计划，即使所有被引用输入列已经提前准备完毕。

完整的职责划分理由、IR 设计、null 语义和 evaluator 失败表参见
[Evaluator Design](evaluator.zh-CN.md)。

## 核验地图

| 契约领域 | 实现锚点 | 核验锚点 |
| --- | --- | --- |
| 字节 span 与展示位置 | [`analyzer/src/span.rs`](../../analyzer/src/span.rs)、[`analyzer/src/source_map.rs`](../../analyzer/src/source_map.rs) | [`test_invariants.rs`](../../analyzer/src/tests/parser/test_invariants.rs) 中的 parser span 不变量 |
| UTF-8/UTF-16 转换与 DTO | [`analyzer_wasm/src/offsets.rs`](../../analyzer_wasm/src/offsets.rs)、[`dto/v1.rs`](../../analyzer_wasm/src/dto/v1.rs) | offset 单元测试与 [`analyzer_wasm/tests/analyze.rs`](../../analyzer_wasm/tests/analyze.rs) |
| token 与错误恢复后的 AST | [`analyzer/src/lexer/token.rs`](../../analyzer/src/lexer/token.rs)、[`parser/tokenstream.rs`](../../analyzer/src/parser/tokenstream.rs)、[`parser/ast.rs`](../../analyzer/src/parser/ast.rs) | [`test_tokens_in_span.rs`](../../analyzer/src/tests/lexer/test_tokens_in_span.rs)、[`test_parser_spans.rs`](../../analyzer/src/tests/parser/test_parser_spans.rs) |
| 诊断与代码操作 | [`analyzer/src/diagnostics.rs`](../../analyzer/src/diagnostics.rs) | [`diagnostics_golden.rs`](../../analyzer/tests/diagnostics_golden.rs)、[`test_errors.rs`](../../analyzer/src/tests/parser/test_errors.rs) |
| IDE 编辑与签名帮助 | [`ide/src/edit.rs`](../../ide/src/edit.rs)、[`ide/src/signature/`](../../ide/src/signature/)、[`ide/src/display.rs`](../../ide/src/display.rs) | [`test_edit_ops.rs`](../../ide/src/tests/ide/test_edit_ops.rs)、[`test_completion_signature_help.rs`](../../ide/src/tests/ide/test_completion_signature_help.rs) |
| WASM 配置 | [`analyzer_wasm/src/lib.rs`](../../analyzer_wasm/src/lib.rs)、[`dto/v1.rs`](../../analyzer_wasm/src/dto/v1.rs) | [`analyzer_wasm/tests/analyze.rs`](../../analyzer_wasm/tests/analyze.rs) 中的构造器测试 |
| evaluator 已准备输入与行状态 | [`evaluator/src/core/inputs.rs`](../../evaluator/src/core/inputs.rs)、[`planner/prepared.rs`](../../evaluator/src/planner/prepared.rs)、[`core/types.rs`](../../evaluator/src/core/types.rs) | [`evaluator/tests/runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs) 与 [`evaluator/tests/builtins/`](../../evaluator/tests/builtins/) 下的 builtin golden |

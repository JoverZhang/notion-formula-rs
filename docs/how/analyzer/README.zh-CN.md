---
doc_id: how.analyzer
title: "Analyzer 如何保留未完成公式中的可用结果"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Analyzer 如何保留未完成公式中的可用结果

[English](README.md)

`analyzer` crate 把 UTF-8 公式源码转换为语法和语义产物，即使源码尚未写完，这些产物仍能供下游使用。
本文面向需要修改或调试词法、语法或语义分析的维护者，重点解释错误恢复流水线、贯穿各阶段的坐标与诊断
模型，以及 IDE 和 evaluator 使用的内部交接点。

本文描述的是 Current 状态的 Rust 实现。公式语言、builtin 清单、编辑器行为、WASM API 和 evaluator
运行时契约属于面向使用者的事实，应由 `docs/specs/` 维护，不在本文中定义。

## 流水线保留证据，而不是只返回合法或非法

每个阶段都会留下自己能够产生的产物，并另行报告问题：

```text
UTF-8 source
    |
    | lex
    v
tokens + lexical diagnostics
    |
    | parse (skip trivia for grammar decisions)
    v
Expr, including Error nodes + parse diagnostics
    |
    | semantic analysis mutates Expr
    +-- desugar eligible MemberCall nodes
    +-- infer types and resolve builtin calls
    +-- validate calls
    v
root Ty + SemanticMap + semantic diagnostics
```

这张图表示处理顺序，不对应某一个公开返回类型。`analyze_syntax` 返回恢复后的表达式、token 和语法诊断；
便捷入口 `analyze` 会运行全部阶段，但只返回 token、合并后的诊断和根表达式类型。需要规范化 AST 及其
`SemanticMap` 的调用方，应自行组合语法与语义入口。

### 跟随一个公式经过各阶段

假设 `Context` 包含受支持的 builtin 签名，并把 `Scores` 声明为 number list。对于下面的源码：

```text
prop("Scores").map(current + 1)
```

各阶段依次补充信息：

1. lexer 按源码顺序产生 token，并在末尾添加显式 `Eof`。如果源码中有注释或换行，它们也会作为
   trivia 留在同一个 token 流中。
2. Pratt parser 构造 `MemberCall`，其 receiver 是 `prop("Scores")`。
3. `desugar_member_calls` 识别出 `map` 支持 postfix 调用，并把节点改写成
   `map(prop("Scores"), current + 1)`。
4. 类型推导使用共享 builtin resolver 投影 mapper 参数，在临时词法作用域中把 `current` 绑定为
   `Number`，推导函数体，再用 `ImplicitLambda` 节点包裹该函数体。
5. 最终调用解析结果和表达式类型保存在 `SemanticMap` 中；校验阶段复用同一份解析结果，不重新绑定调用。
   根表达式类型为 `List(Number)`。

`builtin_fn` 负责签名模型和调用解析器。Analyzer 负责表达式遍历、临时作用域、AST 改写、语义事实和
源码诊断，用这些机制把共享签名模型应用到公式源码。

## Byte span 串联源码、token、表达式和编辑

[`Span`](../../../analyzer/src/span.rs) 和 [`TextEdit`](../../../analyzer/src/text_edit.rs) 使用 UTF-8
字节计量的半开区间 `[start, end)`。Analyzer 产生的端点都是对应源码中的有效字符边界；调用方自行构造
这些类型时也必须维持该不变量。这个 crate 不执行 UTF-16 转换。

[`SourceMap::line_col`](../../../analyzer/src/source_map.rs) 只负责得到展示位置，并没有引入第二套源码
坐标。它会把输入字节 offset 向下截断到有效字符边界，再返回从 1 开始的行号和列号。列按 Unicode
scalar value（`char`）计数，不按字节或 UTF-16 code unit 计数。

lexer 会保留行注释、块注释和换行作为 trivia，跳过空格、tab 和回车，最后一定在源码长度处添加 span
为空的 `Eof` token。parser 选择语法分支时会跳过 trivia，但返回 AST 时仍会同时返回原始 token 流。
因此，trivia 存在于源码和 token 流中，而不是 `ExprKind` 中。

Token range 使用另一种单位。[`tokens_in_span`](../../../analyzer/src/lexer/token.rs) 把非空 byte span
映射为与其相交的 token index 半开区间；因为 `Eof` 的 span 为空，非空区间不会包含它。空区间或反向
区间则映射到一个稳定插入点，即第一个起点不早于 `span.start` 的 token 之前；这个位置可以是 `Eof`
index。[`TokenQuery`](../../../analyzer/src/parser/tokenstream.rs) 在相同规则之上提供感知 trivia 的相邻
查询和区间查询，避免消费方再实现一套 token index 算法。

对于成功解析的结构，父表达式 span 由首尾非 trivia token 定位，并包含子表达式 span。两个定位 token
之间的 trivia 虽然不在 AST 中，仍位于父表达式的字节范围内。相关不变量由
[`test_invariants.rs`](../../../analyzer/src/tests/parser/test_invariants.rs) 和
[`test_tokens_in_span.rs`](../../../analyzer/src/tests/lexer/test_tokens_in_span.rs) 覆盖。

## 语法恢复修补结构，不虚构含义

parser 使用 Pratt binding power 处理 prefix、binary 和 ternary 表达式。List 在 primary 阶段解析；
随后独立的 postfix loop 在 primary 之后继续消费 prefix call 与 member call。缺少必要语法时，parser
可以插入 `ExprKind::Error`，并扫描至逗号、冒号或闭合分隔符等安全边界，让外层表达式继续供下游使用。

分隔符和分隔项恢复可以在诊断中附加 `CodeAction`，其中包含使用字节坐标的 `TextEdit`，例如插入缺失的
`)` 或删除尾随逗号。恢复操作只描述一次局部源码修补，并不保证剩余公式已经合法。所有 AST 消费方都
必须处理 `Error` 节点。

AST 保存的是表达式结构，不允许任意表达式成为调用目标。Prefix 调用的 callee 只能是 identifier。
Member 语法也只有 `receiver.method(...)` 这种调用形式能够进入语义分析；裸写
`receiver.member` 会产生诊断并恢复成错误表达式。

## 语义分析先规范化，再校验

[`analyze_expr_with_semantic_map`](../../../analyzer/src/analysis/mod.rs) 按顺序对可变表达式执行三项操作：

1. **改写受支持的 postfix 调用。** 只有 method 出现在 `postfix_capable_builtin_names()` 中时，member
   call 才会被改写。Allowlist 先取得 `builtins_functions()` 返回的受支持 signature，再通过
   `is_postfix_capable` 过滤。非空 parameter head 必须至少有两个 display parameter；head 为空时，
   repeat-first shape 必须有两个 display parameter，或由最少 repeat group 提供至少两个 position。
   Tail-only shape 不符合条件。这些规则会确定一个 receiver slot，并保留另一个 argument position；后续
   查找和校验仍使用 `Context.functions`。Filter 与对应的语义边界测试位于
   [`analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs) 和
   [`test_semantic.rs`](../../../analyzer/src/tests/analysis/test_semantic.rs)。
2. **尽可能推导事实。** 推导过程访问子表达式、记录类型，并为每个成功解析的 builtin 调用保留最终
   `ResolvedFunctionSig`。无法确定的表达式记为 `Ty::Unknown`，推导本身不产生诊断。函数类型参数会在
   临时词法作用域中完成推导，再由合成的 `ImplicitLambda` 节点包裹。
3. **校验调用。** 校验阶段检查特殊形式 `prop`、未知函数、不受支持的 postfix 调用、参数 shape、
   identifier 位置和参数类型。一个调用若有 shape 错误，只产生一条该调用的诊断，并抑制逐参数的类型
   不匹配诊断。

`prop("Name")` 不属于 builtin `FunctionSig`。类型推导和校验会直接用字符串字面量查询
`Context.properties`。

规范 postfix allowlist 与 `Context.functions` 的区别会影响自定义 Rust 调用方。如果 context 没有提供
某个规范 postfix 函数，该名称仍会先被改写，随后被报告为未知函数。反过来，调用方自定义的函数不会因为
出现在 context 中就自动支持 postfix 调用。

改写后仍然存在的 `MemberCall` 是非法调用，但推导阶段仍会访问它的 receiver 和参数，再把调用类型记为
`Unknown`。校验阶段会报告已知 callable（包括特殊形式 `prop`）不支持 postfix；未知 method 则报告为
未知函数。

保留 AST 的调用方可以观察到语义阶段的修改。完整流程结束后，符合条件的 postfix 调用已经变成 `Call`
节点，函数类型参数也可能变成 `ImplicitLambda` 节点。仅执行 inference 的辅助函数不会运行完整的
desugar-infer-validate 流程。

## 诊断聚合分为局部阶段与最终阶段

[`Diagnostics`](../../../analyzer/src/diagnostics.rs) 用于仲裁 parser 恢复期间产生的诊断。对于完全相同的
span，新诊断优先级更高时会替换已有诊断；优先级相同且消息相同时，会合并并去重 label、note 和 action；
优先级相同但消息不同时，则保留已有诊断。因此，同一个插入点上的多个恢复路径可以最终只留下一个优先级
更高的 parse error。

“同一 span 只保留一条”只适用于 `Diagnostics` 聚合器。`analyze_syntax` 在解析结束后追加 lexer
diagnostics，`analyze` 又在之后追加 semantic diagnostics；最终 vector 不保证按 span 全局去重。

`format_diagnostics` 依次按起点、终点、优先级降序和消息排序诊断，以产生确定的文本输出。它还会排序
label，并保留 note 去重后的发出顺序。结构化 code action 继续附着在 `Diagnostic.actions` 上；文本
formatter 不会渲染它们。Parser 恢复与稳定渲染由
[`test_errors.rs`](../../../analyzer/src/tests/parser/test_errors.rs) 和
[`diagnostics` golden suite](../../../analyzer/tests/diagnostics_golden.rs) 覆盖。

## 各阶段失败时保留已有产物

错误恢复有明确边界，并非无限继续：

| 阶段 | 失败行为 | 仍可使用的产物 |
| --- | --- | --- |
| Lexer | 非法字符串 escape 会产生诊断并继续扫描；未闭合的字符串或块注释以及意外字符会停止扫描。 | 停止点之前的 token、显式 `Eof` 和 lexical diagnostics |
| Parser | 缺失或不匹配的语法会产生诊断，可能附加 action，并插入 `Error` 节点或在分隔符处同步。 | token 流和尽力恢复的表达式 |
| Inference | 未解析的 identifier、错误节点和无法确定的运算会变成 `Ty::Unknown`。 | 已访问表达式的类型，以及能够得到的调用解析记录 |
| Validation | 非法 `prop` 调用、未知函数、不支持的 postfix 调用和签名不匹配会产生语义诊断。 | 已规范化的表达式和已经推导出的语义事实 |

即使存在语法诊断，`analyze` 仍会继续执行语义分析。这种行为适合交互式消费方，但 analyzer 不负责决定
某条诊断是否应阻止执行。要求更严格的消费方必须在自己的交接边界进行判断。

## 根据所需产物选择入口

| 入口 | 产物 | 边界 |
| --- | --- | --- |
| `analyze_syntax(text)` | `ParseOutput { expr, diagnostics, tokens }` | 只运行词法和语法分析；`expr` 可以包含 `Error` 节点。 |
| `analyze(text, ctx)` | `AnalyzeResult { diagnostics, tokens, output_type }` | 运行完整流水线，但不暴露修改后的表达式或 `SemanticMap`。 |
| `analyze_expr(expr, ctx)` | 根 `Ty` 和语义诊断 | 修改已经解析的表达式；调用方自行保留语法诊断。 |
| `analyze_expr_with_semantic_map(expr, ctx)` | 根 `Ty`、`SemanticMap` 和语义诊断 | 需要最终调用解析结果的消费方使用的主要交接点。 |
| `infer_expr_with_map` / `infer_expr_with_semantic_map` | 尽力得到的类型事实 | 不产生诊断，也不执行完整语义流程。 |

这个 crate 还通过 `analyzer::semantic` 重新导出 builtin 语义词汇，方便消费方使用共享类型，但这并没有把
声明和调用投影规则的所有权从 `builtin_fn` 转移给 analyzer。

## Analyzer 之后的行为由相邻 crate 负责

- `builtin_fn` 负责 builtin 声明、`ParamShape`、泛型绑定和 `resolve_call_signature`。
- `ide` 负责 cursor 解释、补全、签名展示、格式化和编辑应用。它会消费 analyzer 提供的源码、token、
  表达式、span 和尽力推导的类型。
- `analyzer_wasm` 负责 JavaScript 门面以及所有 UTF-8/UTF-16 转换。
- `evaluator` 消费规范化后的表达式和 `SemanticMap` 来准备执行；运行时值和逐行错误属于 evaluator。

把 trivia 留在 AST 之外可以缩小表达式模型，但需要保留原始格式的消费方必须同时持有源码与 token 流。
语义分析修改 AST，让下游只需处理一种规范调用形式和显式 lambda 节点，但调用方不能把 parse 后的 AST
视为跨语义阶段不可变。`Error` 和 `Unknown` 保留了局部分析结果，同时也要求每个消费方自行选择严格程度。

## 按处理阶段继续阅读源码

- 公开组合入口与返回类型：[`analyzer/src/lib.rs`](../../../analyzer/src/lib.rs)
- 词法停止条件与 token：[`lexer/mod.rs`](../../../analyzer/src/lexer/mod.rs) 和
  [`lexer/token.rs`](../../../analyzer/src/lexer/token.rs)
- Pratt parsing 与恢复：[`parser/expr.rs`](../../../analyzer/src/parser/expr.rs) 和
  [`parser/ast.rs`](../../../analyzer/src/parser/ast.rs)
- 语义顺序、改写与推导：[`analysis/mod.rs`](../../../analyzer/src/analysis/mod.rs)、
  [`analysis/desugar.rs`](../../../analyzer/src/analysis/desugar.rs) 和
  [`analysis/infer.rs`](../../../analyzer/src/analysis/infer.rs)
- 代表性语义测试：[`test_semantic.rs`](../../../analyzer/src/tests/analysis/test_semantic.rs)、
  [`test_implicit_lambda.rs`](../../../analyzer/src/tests/analysis/test_implicit_lambda.rs) 和
  [`test_resolved_calls.rs`](../../../analyzer/src/tests/analysis/test_resolved_calls.rs)

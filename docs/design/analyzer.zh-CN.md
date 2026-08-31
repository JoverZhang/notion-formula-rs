---
doc_id: architecture.analyzer
title: "analyzer 如何让不完整公式仍可供工具使用？"
language: zh-CN
source_language: en
counterpart: ./analyzer.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-31
---

# 可恢复的公式分析

[English](analyzer.md)

本文记录 `analyzer` crate 的 **Current** 行为，说明用户还在输入公式时，系统如何从不完整源码中保留
可用的语法和语义结果。主要读者是需要修改或排查 lexer、parser 与语义分析，又不希望破坏 IDE 和
evaluator 消费方式的贡献者。

本文从一段 UTF-8 源码开始，到 analyzer 生成恢复后的表达式、token、诊断和语义事实为止。完整语法、
跨 crate 坐标契约、内置函数目录、IDE 行为和 evaluator 运行时分别由其他文档维护；这里只说明它们与
analyzer 流水线的交界处。

## 流水线不只给出“通过”或“失败”

analyzer 不会把结果压缩成一个“有效/无效”判断。每个阶段都会保留当时还能生成的结果，并把问题记录为诊断：

```text
UTF-8 source
    │
    │ lex
    ▼
tokens + lex diagnostics
    │
    │ parse (trivia is skipped for grammar decisions)
    ▼
Expr, including Error nodes + parse diagnostics
    │
    │ semantic analysis mutates the Expr
    ├── desugar eligible MemberCall nodes
    ├── infer expression types and resolve builtin calls
    └── validate calls
    ▼
root Ty + SemanticMap + semantic diagnostics
```

上图展示的是内部结果和处理顺序，并不对应某一个公开返回类型。例如，`analyze_syntax` 会返回 token 和
恢复后的 `Expr`，便捷入口 `analyze` 则返回 token、汇总后的诊断以及根表达式的输出类型。既要可变表达式，
又要 `SemanticMap` 的调用方，需要自行组合语法和语义入口。

本 crate 内的源码 span 都是 UTF-8 字节坐标下的半开区间。完整的坐标和 token range 规则由
[跨 crate 契约](contracts.zh-CN.md)维护。

## 跟随一条公式看完整过程

假设语义 `Context` 包含标准内置函数目录，并把 `Scores` 声明为数字列表。当前收到的源码是：

```text
prop("Scores").map(current + 1)
```

这条表达式会在各阶段逐步获得更多含义：

1. lexer 按源码顺序生成 token，并在末尾加入显式 `Eof`。这条公式没有非语义 token；如果存在注释或
   换行，它们也会留在同一个 token 流中。
2. Pratt parser 生成一个 `MemberCall`。接收者是 `prop("Scores")` 调用，参数则是尚未绑定
   `current` 的表达式 `current + 1`。
3. 类型推断开始前，`desugar_member_calls` 确认 `map` 支持后缀调用，把节点改写成等价的前缀形式
   `map(prop("Scores"), current + 1)`。
4. 共享的内置函数解析器识别出 mapper 参数是一个以列表元素类型为参数的函数。类型推断暂时把
   `current` 绑定为 `Number`，将 mapper 主体推断为 `Number`，再用 `ImplicitLambda` 节点包装该
   主体。`SemanticMap` 保留最终调用解析结果和各表达式类型，根类型为 `List(Number)`。
5. 校验阶段直接消费同一份调用解析结果，不会自行重建另一套 `map` 签名。`prop` 不属于内置函数
   `FunctionSig`，因此会单独对照 `Context.properties` 检查。

这个例子也划清了一条重要职责边界：`builtin_fn` 负责签名模型和调用解析；analyzer 负责遍历表达式、
管理词法作用域、改写 AST，并根据共享契约生成源码诊断。

## 语法恢复保留局部结构

### token 流与表达式树保留不同信息

lexer 为每个 token 记录源码 span，把行注释、块注释和换行作为非语义 token 保留下来，并在源码末尾
附加一个空的 `Eof` token。普通空格、制表符和回车不会成为 token。parser 做语法判断时通常跳过非语义
token，但会把原始 token 流与表达式树一起返回。

因此，非语义 token 并不存进 AST。AST 只表达公式结构；源码保真由源码字符串、token 流和 span 共同
承担。formatter 等消费者会组合使用这三者，而不是在 `ExprKind` 中寻找注释或换行节点。

### parser 只恢复结构，不判断语义

parser 用 Pratt binding power 表达运算符优先级和结合方向，并据此构建一元、二元、三元、调用、列表和
成员调用表达式。缺少预期表达式时，它会插入 `ExprKind::Error`，并向逗号、冒号或闭合定界符等安全边界
同步。这样，外围表达式仍可供后续工具使用。

定界符和分隔符恢复可以在诊断中附带 `CodeAction` 编辑，例如补上缺失的 `)`，或者删除尾随逗号。恢复
并不表示公式已经有效，也不会臆造语义值。所有 AST 消费者都必须处理 `Error`；出现诊断并不等于没有
表达式树。

语法不支持裸成员访问。`receiver.member` 会产生语法诊断和 `ExprKind::Error` 节点；只有
`receiver.method(...)` 这种调用形式，才会以 `MemberCall` 进入语义分析。

## 语义分析先归一化，再做校验

`analyze_expr_with_semantic_map` 会按以下顺序操作一棵可变表达式树：

1. **展开支持的后缀调用。** 只有方法名出现在 `postfix_capable_builtin_names()` 中，成员调用才会
   被改写。这个允许列表来自 `builtins_functions()` 返回的规范目录，而不是调用方任意提供的
   `Context.functions`。符合条件的前缀和后缀形式随后进入同一条类型推断与校验路径。
2. **尽力推断语义事实。** 类型推断遍历子表达式、记录各节点类型，并为每个成功解析的内置函数调用保留
   最终 `ResolvedFunctionSig`。无法确定类型时，它记录 `Ty::Unknown`，但不生成诊断。遇到函数类型参数时，
   推断会在临时词法作用域中分析参数主体，再用合成的 `ImplicitLambda` 节点包装它。
3. **校验调用。** 校验阶段检查 `prop`、未知函数、不支持的后缀调用、参数形状、标识符位置和参数类型。
   如果内置函数的参数形状无效，该调用只产生一条形状诊断，不再为同一调用逐个报告参数类型不匹配。

展开后仍然存在的 `MemberCall` 都是无效调用，但 analyzer 仍会分析其接收者和参数。已知可调用项（包括
特殊处理的 `prop`）会报告不支持后缀调用；未知方法名会报告未知函数。该节点的结果类型保持为
`Unknown`。

AST 变化本身就是语义契约的一部分。调用方如果保留语法分析后的 `Expr`，就会在完整语义分析后看到后缀
调用变成 `Call`，函数类型参数变成 `ImplicitLambda`。

## 失败不会抹掉已经生成的结果

恢复是尽力而为，并非没有边界。关键在于每个阶段遇到问题后还会保留什么：

| 阶段 | 失败行为 | 仍可使用的结果 |
| --- | --- | --- |
| lexer | 无效字符串转义会产生诊断，但仍保留在字符串 token 中。字符串或块注释未终止，或者遇到意外字符时，词法扫描会停止。 | 停止位置之前的 token、显式 `Eof` 和词法诊断 |
| parser | 缺失或不匹配的语法会产生诊断，可能附带代码操作，并插入 `Error` 节点或同步到定界符。 | token 流和尽力恢复的表达式树 |
| 类型推断 | 未解析标识符、`Error` 节点和其他无法确定的表达式会得到 `Ty::Unknown`；类型推断本身不生成诊断。 | 已访问表达式的类型，以及能够完成解析的内置函数调用记录 |
| 校验 | 无效 `prop` 调用、未知函数、不支持的后缀调用和签名不匹配会产生语义诊断。 | 已归一化的表达式和此前推断出的语义事实 |

一站式入口 `analyze` 即使拿到语法诊断，仍会继续执行语义分析。这有利于交互式反馈，但面向执行的调用方
必须自行决定哪些诊断会阻止下一阶段。例如，evaluator 在准备执行计划时会拒绝语义诊断。

## 根据所需结果选择入口

| 入口 | 返回结果 | 需要注意的边界 |
| --- | --- | --- |
| `analyze_syntax(text)` | `ParseOutput { expr, diagnostics, tokens }` | 只执行 lexer 和 parser；返回的 `Expr` 可能含有 `Error` 节点。 |
| `analyze(text, ctx)` | `AnalyzeResult { diagnostics, tokens, output_type }` | 执行完整流水线，但不暴露变化后的 `Expr` 或 `SemanticMap`。 |
| `analyze_expr(expr, ctx)` | 根表达式的 `Ty` 和语义诊断 | 修改已经完成语法分析的表达式；调用方仍要自行处理语法诊断。 |
| `analyze_expr_with_semantic_map(expr, ctx)` | 根表达式的 `Ty`、`SemanticMap` 和语义诊断 | evaluator 构建执行计划等需要调用解析结果的消费者应使用这个主要接口。 |
| `infer_expr_with_map` / `infer_expr_with_semantic_map` | 尽力推断的类型事实 | 这些入口只做类型推断，不生成诊断，也不执行完整的语义预处理和校验顺序。 |

不同 IDE 功能会有意选择不同入口。格式化需要恢复后的 `Expr`、原始源码和 token；补全与签名帮助还会对
源码片段做尽力类型推断，因为光标往往停在尚未完成的公式中。这些策略属于 [IDE 设计](ide.md)，并不是
analyzer 入口本身的职责。

## 越过 analyzer 后，由相邻 crate 接手

- [`builtin_fn`](builtin-fn.md) 负责内置函数声明、参数形状、泛型绑定和共享调用签名解析。analyzer 会
  re-export 这些语义概念，但不维护第二套签名模型。
- [`ide`](ide.md) 负责解释光标位置、补全、签名展示、格式化和应用编辑。它消费 analyzer 的 token、
  表达式、span 和尽力推断的类型。
- [`analyzer_wasm`](wasm-boundary.md) 负责 JavaScript 门面以及所有 UTF-8/UTF-16 转换。analyzer
  自身不会切换坐标系统。
- [`evaluator`](evaluator.zh-CN.md) 消费完成语义分析的表达式和 `SemanticMap`，再构建执行计划。运行时值、
  行级错误和输入契约属于 evaluator。

这些边界让 analyzer 可以同时服务宽容的编辑流程和严格的执行流程，而不必把其中任何一种失败策略强加给
所有消费者。

## 用可变语义树换取统一表示

不把非语义 token 放进 AST，可以让表达式模型保持紧凑；代价是需要保留源码的消费者必须让源码、token 流
和表达式树始终配套。语义分析期间直接修改 AST，可以给下游执行计划构建提供一种统一调用形式和显式
lambda 节点；代价是调用方不能假设语法分析后的树在这一步前后保持不变。

`Error` 和 `Unknown` 也把失败处理的一部分责任交给了消费者。对于需要在公式尚未完成时获得局部结构和类型
的 IDE 功能，这种代价是有意接受的；需要可执行公式的消费者，则必须在继续前设置更严格的诊断边界。

扩展 analyzer 时应继续守住这些接口：

- 新语法属于 lexer/parser，并且必须定义恢复行为；
- 内置函数签名或调用解析变更属于 `builtin_fn`；
- 后缀调用资格必须继续由 analyzer 语义分析和 IDE 展示共享；
- 源码坐标变更属于跨 crate 契约变更，不是局部 parser 重构。

## 继续阅读源码

- 公开入口和返回结果：[`analyzer/src/lib.rs`](../../analyzer/src/lib.rs)
- token 与词法扫描停止条件：[`analyzer/src/lexer/mod.rs`](../../analyzer/src/lexer/mod.rs) 和
  [`token.rs`](../../analyzer/src/lexer/token.rs)
- Pratt parser 与错误恢复：[`analyzer/src/parser/expr.rs`](../../analyzer/src/parser/expr.rs) 和
  [`ast.rs`](../../analyzer/src/parser/ast.rs)
- 语义处理顺序和诊断：[`analyzer/src/analysis/mod.rs`](../../analyzer/src/analysis/mod.rs)、
  [`desugar.rs`](../../analyzer/src/analysis/desugar.rs) 和
  [`infer.rs`](../../analyzer/src/analysis/infer.rs)
- 有代表性的恢复和语义测试：[`test_errors.rs`](../../analyzer/src/tests/parser/test_errors.rs)、
  [`test_semantic.rs`](../../analyzer/src/tests/analysis/test_semantic.rs) 和
  [`test_implicit_lambda.rs`](../../analyzer/src/tests/analysis/test_implicit_lambda.rs)

完整测试目录和运行方式继续由[测试清单](testing.md)维护。

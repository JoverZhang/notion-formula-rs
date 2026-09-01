---
doc_id: architecture.index
title: "notion-formula-rs 架构地图"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# notion-formula-rs 设计

[English](README.md)

本文是一份 Current 状态的架构导览，回答两个问题：贡献者应该从哪里开始阅读，以及不同类型的设计事实由
哪份文档负责。主要读者是需要先建立端到端认识、再深入实现细节的新贡献者和活跃贡献者。

本文索引稳定架构、跨 crate 契约和设计理由。实现细节请阅读各模块自己的 README，例如
`analyzer/README.md`。文档工作流和双语维护规则参见 [`docs/README.zh-CN.md`](../README.zh-CN.md)。

## 处理流水线

```text
  Source (UTF-8 string)
       |
       v
  Lexer ──> Tokens + Trivia + Lex diagnostics
       |         (analyzer/src/lexer/)
       v
  Parser (Pratt) ──> AST + Parse diagnostics
       |              (analyzer/src/parser/)
       v
  Semantic Analysis ──> TypeMap + Semantic diagnostics
       |                 (analyzer/src/analysis/
       |                  + builtin_fn/ type model)
       |
       +──> IDE (format, complete, signature help)
       |         (ide/src/)
       |
       +──> WASM boundary (UTF-8 -> UTF-16, DTOs)
       |         (analyzer_wasm/src/)
       |
       +──> Evaluator (AST -> IR -> row-batch)
                 (evaluator/src/)
```

该图从左上方的公式源码开始，展示各 crate 之间的主数据流；它有意省略模块内部调用和错误恢复细节。
图中的英文标签与代码和共享术语保持一致。

## 目标

- 提供稳定、可复用的公式语法分析与诊断能力。
- 为交互式编辑提供 IDE 级体验，包括格式化、补全和签名帮助。
- 提供面向 WASM/TypeScript 的入口和轻量 DTO 防腐层，并保持 UTF-8/UTF-16 坐标一致。

## 模块概览

| 模块 | 职责摘要 | 模块 README |
| --- | --- | --- |
| `builtin_fn/` | 分类 DSL 目录、签名模型和共享调用解析 | `builtin_fn/README.md` |
| `builtin_fn_macros/` | 分类 DSL 的过程宏实现 | `builtin_fn_macros/README.md` |
| `analyzer/` | lexer、parser、AST、诊断和语义分析 | `analyzer/README.md` |
| `ide/` | 格式化、补全、签名帮助和编辑应用 | `ide/README.md` |
| `analyzer_wasm/` | wasm-bindgen 边界、UTF-16 映射和 DTO v1 | `analyzer_wasm/README.md` |
| `evaluator/` | 输入预准备、同步行批次运行时和生成式 builtin ABI | `evaluator/README.md` |
| `examples/vite/` | 示例集成 | `examples/vite/README.md` |
| `docs/` | 设计文档和 changelog 指南 | `docs/README.md` |

## 设计文档索引

尚未建立中文 counterpart 的文档会直接链接到英文源文档；这表示 `source-only`，并不表示存在一份未完成的中文稿。

| 文档 | 范围 |
| --- | --- |
| [`contracts.zh-CN.md`](contracts.zh-CN.md) | 跨 crate 的坐标、错误恢复、确定性、编辑与求值边界 |
| [`builtin-fn.md`](builtin-fn.md) | 内置函数声明 DSL、调用签名解析、目录和 evaluator 契约 |
| [`analyzer.zh-CN.md`](analyzer.zh-CN.md) | 可恢复语法、语义归一化与校验，以及 analyzer 的职责边界 |
| [`ide.md`](ide.md) | 补全、签名帮助、格式化和编辑应用 |
| [`wasm-boundary.md`](wasm-boundary.md) | WASM 门面、DTO、UTF-16 转换和 JavaScript 接口 |
| [`evaluator.zh-CN.md`](evaluator.zh-CN.md) | IR、Planner、kernel 和行批次求值 |
| [`testing.md`](testing.md) | 所有 crate 的测试清单 |
| [`demo-vite.md`](demo-vite.md) | Vite 示例应用 UI/UX |
| [`drift-tracker.md`](drift-tracker.md) | 开放问题和已知缺口 |

## 设计原则

- 保持简单：以尽量少的额外结构交付有力能力。
- 契约优先：先稳定接口与跨模块规则，所有变化都应可追踪。
- 尽力恢复语法分析：遇到语法错误时不立即停止，而是尽可能返回有用结果。
- 默认确定性：相同输入产生相同输出，包括排序、去重和格式化结果。
- 明确坐标边界：Rust 核心使用 UTF-8 字节，JavaScript/WASM 使用 UTF-16 code unit。

## 术语

规范英文术语、中文 counterpart、代码标识符和概念边界统一维护在共享的
[`项目术语表`](../glossary.md) 中。

parser 和 IDE 测试还会使用一个局部记号 `$0`，表示 `[0, length)` 范围内的 cursor 位置。

## 语言范围

- 语法遵循 Notion 官方指南：<https://www.notion.com/help/formula-syntax>。

当前语法摘要：

- 标识符以 Unicode 字母或 `_` 开头，后续可以是 Unicode 字母、数字或 `_`。
- 数字支持整数、十进制浮点数和科学计数法。
- 字符串使用双引号并支持 `\n`、`\t`、`\"`、`\\` 转义。
- 列表不允许 `[1, 2,]` 形式的尾随逗号。
- 算术运算符包括 `+`、`-`、`*`、`/`、`%`、`^`；`%` 表示取模，`^` 为右结合幂运算。
- 逻辑运算符包括 `&&`、`||`、`!`。
- 关键字包括 `not`、`true`、`false`。
- 支持普通函数调用和成员方法调用。
  - 普通函数调用：`name(arg1, ...)`。
  - 成员方法调用：`receiver.name(arg1, ...)`。
  - 内置函数行为和支持状态参见[内置函数规格](../specs/builtin-functions/README.zh-CN.md)。

---
doc_id: specs.index
title: "规格索引"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-18
---

# 规格索引

[English](README.md)

规格按五个使用边界组织，不按 crate 拆分。同一规则只在一处定义，其余文档链接到它。

| 文档 | 负责的契约 | 状态 |
| --- | --- | --- |
| [FormulaEngine](formula-runtime.zh-CN.md) | 定义、依赖编译、状态、列式求值 | Planned |
| [WASM API](formula-runtime-wasm.zh-CN.md) | Worker、薄客户端、DTO、坐标、生命周期 | Planned；单独保留 Current Analyzer |
| [IDE / FormulaDraft](formula-draft.zh-CN.md) | 草稿、help、quick fixes、format、edits、提交/丢弃 | Planned；单独保留 Current IDE |
| [公式文法](formula-language.zh-CN.md) | EBNF、property reference、运算符、null、失败边界 | Current |
| [Builtin](builtin-functions.zh-CN.md) | 支持的函数、签名记法、调用规则 | Current |

```text
Current = 已有实现的可观察行为。
Planned = 待实现接口；英文同步不意味着已经发布。
Rust 的无方法体 impl 是接口声明示意，不是可单独编译的普通 Rust 模块。
只有显式标记 header=<key> 的块参与头文件生成；本轮规格尚未接入生成。
```

生成规则见 [spec-codegen](../contributing/spec-codegen.zh-CN.md)；
算法和内部模块见[实现导读](../how/README.zh-CN.md)。

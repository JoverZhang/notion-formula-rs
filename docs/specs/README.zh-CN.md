---
doc_id: specs.index
title: "当前行为规格索引"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 当前行为规格索引

[English](README.md)

根据需要依赖的行为选择规格：

- [公式语言](formula-language.zh-CN.md)：源码形式、运算符语义、分析行为和求值结果
- [Formula reference](formula-references.zh-CN.md)：property reference 与 formula reference 的行为，
  包括 rename 处理
- [Builtin function](builtin-functions/README.zh-CN.md)：调用形态、类型解析、受控求值和逐行失败
- [Editor service](editor-services.zh-CN.md)：诊断、补全、签名帮助、格式化和编辑应用
- [WASM API](wasm-api.zh-CN.md)：面向 JavaScript 的配置、DTO、UTF-16 坐标、操作和错误

这些文档维护当前用户可观察行为。需要查阅 Rust 内部类型、算法或 crate 边界时，
从[实现导读](../how/README.zh-CN.md)开始。两个层级的维护规则由
[文档维护标准](../../DOCUMENTATION.zh-CN.md)统一定义。

---
doc_id: how.index
title: "实现导读"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 实现导读

[English](README.md)

该 workspace 从同一组内置函数声明和一份公式源码出发，生成分析结果、编辑器响应或逐行
求值结果。各 crate 之间有两类关系：

```text
声明与构建路径
builtin_fn_macros --macro expansion--> builtin_fn
builtin_fn --supported declarations--> evaluator/build.rs --generated ABI--> evaluator

请求与数据路径
source --analysis--> analyzer --resolved expression--> evaluator
                        |
                        +--analysis artifacts--> ide
                              |
               analyzer + ide --WASM facade--> analyzer_wasm --DTOs--> Vite example
```

`builtin_fn_macros` 校验每个声明块，并把它转换成 Rust 表达式。`builtin_fn` 维护结构化内置函数目录
和共用的调用解析。`analyzer` 生成语法与语义分析结果。`ide` 根据这些结果生成编辑器提示和编辑；
`evaluator` 通过 build script 生成 builtin ABI，再把通过校验的公式转换成可同步执行的预备计划。
`analyzer_wasm` 负责 Analyzer、IDE 数据与 JavaScript 边界之间的转换，
Vite 示例负责 UI 呈现。

## 进入相应组件

- [builtin_fn_macros](builtin_fn_macros/README.zh-CN.md)：解析、校验并转换声明，报告错误
- [builtin_fn](builtin_fn/README.zh-CN.md)：构建内置函数目录，表达 `ParamShape`，绑定泛型类型，
  解析调用
- [analyzer](analyzer/README.zh-CN.md)：执行词法分析、语法分析、错误恢复和语义分析，维护 span 与 diagnostic
- [ide](ide/README.zh-CN.md)：生成 completion 与 signature help，执行格式化和 edit application
- [evaluator](evaluator/README.zh-CN.md)：准备执行、校验 input contract、维护 mask、运行 kernel，
  分派生成的 builtin 实现
- [analyzer_wasm](analyzer_wasm/README.zh-CN.md)：转换 UTF-16 坐标与 DTO，导出 WASM API，完成打包
- [Vite example](examples/vite/README.zh-CN.md)：集成 CodeMirror，维护 UI state，运行浏览器测试

用户可观察保证由[规格文档](../specs/README.zh-CN.md)维护。修改实现时，通过[测试指南](../contributing/testing.zh-CN.md)选择检查范围。

# notion-formula-rs

一个用 Rust 实现的 Formula 引擎，用于分析、编辑和求值 Notion 风格公式。

[在线演示](https://joverzhang.github.io/notion-formula-rs/) ·
[项目文档](docs/README.zh-CN.md) · [English](README.md)

![浏览器演示中的补全候选、签名帮助和推断结果类型](docs/assets/browser-demo.webp)

## 它能做什么

- 在 Formula 尚未写完时继续分析，并返回诊断和推断出的结果类型。
- 为 Formula 编辑器提供补全、签名帮助、确定性格式化和 quick fix。
- 只准备一次 Formula，随后在 Rust 中同步求值一批行数据。
- 通过 WebAssembly 向浏览器集成提供分析和编辑器操作。

## 当前范围

Notion Formula 是这个项目的起点，并不代表项目已经完整兼容 Notion。浏览器演示聚焦分析和编辑；
同步的逐行求值目前由 Rust evaluator 提供，尚未接入演示页面。受支持的语法和行为以
[当前规格](docs/specs/README.zh-CN.md)为准。

## 继续了解

[项目意图](docs/intent/README.zh-CN.md) · [当前规格](docs/specs/README.zh-CN.md) ·
[实现导读](docs/how/README.zh-CN.md)

## License

本项目采用 [Apache License 2.0](LICENSE)。

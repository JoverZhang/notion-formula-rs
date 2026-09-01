---
doc_id: contributing.testing
title: "如何测试一次修改"
language: zh-CN
source_language: en
counterpart: ./testing.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 如何测试一次修改

[English](testing.md)

开发过程中，先运行能够覆盖当前修改边界的最小测试；如果修改跨越 crate 或语言，再在 review 前扩大范围。
仓库的 [`justfile`](../../justfile) 是命令的权威来源，本文只解释如何选择命令，以及如何审查生成的预期结果。

## 先测试拥有该行为的组件

| 修改范围 | 优先运行 | 覆盖内容 |
| --- | --- | --- |
| 文档结构、metadata、翻译或链接 | `just docs-check` | Checker 自身测试和仓库文档扫描 |
| 内置函数声明、调用形状或解析 | `cargo test -p builtin_fn` | Resolver 与声明 DSL 行为，包括 macro 编译成功/失败测试 |
| Procedural macro 的解析或展开 | `cargo test -p builtin_fn_macros` 和 `cargo test -p builtin_fn` | Macro 实现单元，以及消费方 crate 的声明 DSL 与编译成功/失败 contract |
| 词法、语法、语义分析或诊断 | `cargo test -p analyzer` | Analyzer 单元测试、集成测试和诊断 golden 测试 |
| 公式准备或逐行求值 | `cargo test -p evaluator` | 生成 contract、输入/runtime 不变量和内置函数行为 |
| 补全、签名帮助、格式化或文本编辑 | `cargo test -p ide` | IDE 单元测试、集成测试和格式化 golden 测试 |
| Rust 到 JavaScript 的转换或导出的 WASM 方法 | `cargo test -p analyzer_wasm` 和 `wasm-pack test --node analyzer_wasm` | Native helper，以及通过 `wasm-bindgen` 执行的测试 |
| Vite 示例行为 | `just test-example-vite` | WASM 构建、Vitest 单元测试和 Playwright 端到端测试 |

`cargo test -p analyzer_wasm` 不会执行 `wasm_bindgen_test` 集成测试。如果修改可能影响序列化、UTF-16
offset、面向 JavaScript 的错误或导出方法，应同时运行两条 WASM 命令。

Vite shortcut 会安装锁定版本的 Node 依赖并重新构建 WASM package，然后再运行前端测试。如果直接调用
`pnpm -C examples/vite test` 或 `test:e2e`，需要先准备依赖和生成的 package。各条前端命令以
[`examples/vite/package.json`](../../examples/vite/package.json) 中的 script 为准。

修改跨越多个 Rust crate 时，先完成 focused crate 测试，再运行 `just test`。它会通过 `justfile` recipe
执行仓库选定的 Rust 测试和 Vite 示例测试，但并不等于 `cargo test --workspace`。如果需要运行当前所有
workspace member，包括 `builtin_fn_macros`，应使用这条 Cargo 命令。如果还需要完整覆盖依赖准备、格式
检查、lint、type check、文档检查和测试，运行 `just verify`。`just docker-test` 会在仓库的 CI image 中
执行同一套 verification recipe。

## 根据风险选择测试形态

优先在能够观察到回归的最窄稳定边界上编写测试：

- 单元测试隔离一个 crate 内部的算法和状态变化。
- 集成测试覆盖 crate 边界、生成 contract，或使用调用方输入执行一份 prepared formula。
- 编译成功/失败测试保护内置函数声明 macro 的 Rust 侧 contract。
- Golden fixture 把结构化诊断、格式化公式和逐行结果变成可 review 的文本。
- WASM 与前端测试覆盖 native Rust 测试无法观察的转换和交互行为。

如果一份宽泛的 baseline 只能证明 happy path 仍然可用，就为真正的边界条件增加 focused case。反过来，
也不要在每一层重复同一个断言。让一个测试负责该行为；只有接线或表示转换本身存在风险时，才补充更高层
覆盖。

测试放在拥有它的 crate 附近，目录会随实现变化。应从表格中的 package 找到相关 runner，不要把复制出来
的完整测试目录当成 contract。

## 有意识地更新 golden 预期

Golden 测试把易读的输入与提交到仓库的 `.snap` 结果比较。Snapshot 变化表示行为或表示方式发生了变化，
不是可以直接接受的普通生成文件。更新前先阅读 diff，并确认无关 case 没有一起改变。

Analyzer 诊断和 IDE 格式化使用仓库 recipe：

```bash
just test-analyzer-bless
just test-ide-bless
```

Evaluator 内置函数 fixture 使用 focused runner：

```bash
BLESS=1 cargo test -p evaluator --test builtin_golden
```

Bless 完成后，用不带 `BLESS=1` 的同一条测试重新验证。修改过的输入和 snapshot 应一起提交。不要为了让
测试变绿就直接 bless；先确认是哪项 contract 或实现变化解释了新输出。

## 在 review 前扩大验证范围

交付修改前：

1. 对修改触及的每个边界运行对应的 focused 命令。
2. Markdown、文档工具或本地链接变化时，运行 `just docs-check`。
3. Rust、WASM 或前端源码变化时，运行 `just check`。Clean checkout 应先运行 `just deps`；也可以直接
   使用包含依赖准备的 `just verify`。
4. 行为变化跨 crate 或语言时，运行 `just test`；确实需要完整仓库流程时，再运行 `just verify`。

把实际运行的命令和结果记录在 PR 中。长期文档维护测试策略，PR 则保留本次修改实际经过了哪些验证。

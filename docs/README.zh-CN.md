---
doc_id: docs.index
title: "文档维护指南"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-08-30
---

# 文档

[English](README.md)

阅读入口：[`docs/design/README.zh-CN.md`](design/README.zh-CN.md)。

## 各类文档放在哪里

- `docs/design/README.md`：处理流水线总览和设计文档索引。
- `docs/design/contracts.md`：跨 crate 的硬性规则，包括 span、token、确定性和编辑操作。
- `docs/design/builtin-fn.md`：内置函数声明 DSL、调用签名解析、目录和 evaluator 契约。
- `docs/design/analyzer.md`：词法分析、语法分析和语义分析流水线（`analyzer` crate）。
- `docs/design/ide.md`：补全、签名帮助和格式化（`ide` crate）。
- `docs/design/wasm-boundary.md`：WASM 门面、DTO 和 UTF-16（`analyzer_wasm` crate）。
- `docs/design/evaluator.md`：IR、Planner、kernel 和行批次求值（`evaluator` crate）。
- `docs/design/testing.md`：所有 crate 的测试清单。
- `docs/design/demo-vite.md`：Vite 示例应用的 UI/UX。
- `docs/design/drift-tracker.md`：开放问题和已知缺口。
- `docs/glossary.md`：共享的中英文术语与代码标识符。
- 代码旁的 `README.md`：模块或 crate 文档，例如 `analyzer/README.md`。
- `docs/changelogs/YYYYMMDD-short-slug.md`：用户可见的变更记录。

## 双语文档约定

英文文档保留现有路径，完整的简体中文 counterpart 使用相邻的 `.zh-CN.md` 后缀：

```text
docs/design/evaluator.md <-> docs/design/evaluator.zh-CN.md
docs/README.md           <-> docs/README.zh-CN.md
```

现有的单语源文档不需要创建空白或不完整的 counterpart；建立配对前也不添加下述双语 metadata。一旦建立双语配对：

- 两种语言都必须是完整、自然的独立文档，不采用逐段交错翻译；
- 在标题附近提供双向链接，并复用与语言无关的图、代码标识符、schema 和资源；
- 每轮编辑选择一种源语言，先核验其中的技术事实，再更新 counterpart；
- 两种版本必须保留相同的范围、生命周期状态、确定程度、保证、限制、示例和失败行为；
- 使用 [`glossary.md`](glossary.md) 中的规范术语，代码标识符必须保持原样。

术语表有意采用一份共享的双语映射，而不是拆成两个 counterpart：精确的术语配对就是它的接口，拆分反而会
产生第二个事实来源。

成对的架构文档和契约文档使用以下 metadata：

```yaml
---
doc_id: architecture.example
title: "面向读者的问题式标题"
language: zh-CN
source_language: en
counterpart: ./example.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: YYYY-MM-DD
---
```

counterpart 使用相同的 `doc_id`，分别设置 `language: en` 或 `language: zh-CN`，并链接回另一版本。
只有在需要明确暴露已知漂移时才使用 `translation_status: needs-update`；稳定的 Current 架构或契约发生变化时，
通常应在同一次变更中把两个版本恢复为 `synced`。

生命周期标签具有固定含义：

- **Current**：已在当前实现中核验的行为。
- **Planned**：已经接受、但尚未完整实现的方向。
- **Exploratory**：仍在讨论、尚未承诺的选项。
- **Deprecated**：仍然存在、但正在移除或替换的内容。
- **Historical**：仅用于解释早期决策或迁移过程的内容。

双语文档完成前，需要同时检查技术证据、失败行为、生命周期状态、术语和语义一致性。交付说明必须列出
已经核验的事实、仍然存在的不确定性，以及最终的翻译状态。

运行 `just docs-check` 可以检查本地 Markdown 链接以及双语约定中可机械验证的部分，包括必填 metadata、相邻且
互指的 counterpart、一致的共享字段和合法的状态值；语义一致性仍须人工复核。

## 修改代码时

- 更新被修改模块旁的 README。
  - 直接修订已有内容并删除失效信息，不要无限追加。
- 如果变更破坏或修改了 `docs/design/README.md` 中的契约：
  - 增加测试；
  - 在 PR 或 commit message 中明确说明。
- 如果变更会影响用户可见行为、接口、DTO 或兼容性：
  - 增加一条 changelog，参见下文。

## Agent 编辑策略

无需明确批准即可进行：

- 修复链接、增加索引、通过文件路径澄清文档；
- 在 drift tracker 中增加状态说明或 TODO。

未经明确批准不得进行：

- 修改 span/offset、DTO schema、确定性规则等契约。

## 已在仓库验证的命令

在仓库根目录运行：

```bash
just test
just verify
just deps
just check
just docs-check
just fix
just gen-ts

cargo test
cargo test -p analyzer
cargo test -p analyzer_wasm
BLESS=1 cargo test -p analyzer
```

Vite 示例应用，在 `examples/vite/` 中运行：

```bash
pnpm -s run wasm:build
pnpm -s run dev
pnpm -s run test
pnpm -s run test:e2e
pnpm -s run check
```

## Changelog

- 编写指南：`docs/changelog_entry_guidelines.md`
- 模板：`docs/_description_templates/changelog_entry_template.md`

## 模板

- 模块 README：`docs/_description_templates/module_README_template.md`
- 设计契约：`docs/_description_templates/design_contract_template.md`

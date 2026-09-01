---
doc_id: documentation.policy
title: "文档维护规范"
language: zh-CN
source_language: en
counterpart: ./DOCUMENTATION.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 文档维护规范

[English](DOCUMENTATION.md)

本文是仓库唯一权威的文档维护标准，说明项目知识应该放在哪里，以及什么变化需要更新文档。human 与 agent 贡献者都应遵守本文。其他文档可以提供导航或工作流指引，但不能重新定义文档层级、编辑权限、翻译状态或写作要求。

## 每项事实只属于一个层级

- `docs/intent/` 说明系统为什么存在，包括受众、目标、非目标、边界、取舍和重新评估条件。内容应保持简短。
- `docs/specs/` 说明当前可被使用者观察到的契约，不照搬 crate 结构，也不记录设计演进。
- `docs/how/` 说明当前实现如何工作。目录名与源码 crate 一致，默认每个 crate 只有一篇实现指南。
- `docs/contributing/` 说明测试、changelog 编写等项目特有的开发规范。
- `docs/changelogs/` 记录使用者可见的变化。根目录 `GLOSSARY.md` 是 English-only 的术语权威来源。

每项事实只在一个文档中维护，其他文档通过链接引用。Planned、Exploratory、Deprecated 和 Historical 内容必须与 Current 行为明确分开。

## 尊重 human-controlled 文档

`DOCUMENTATION.*`、`docs/intent/` 和 `docs/specs/` 由 human 控制。只有用户明确要求相应的规范、意图或契约变更时才能编辑。翻译授权只允许补充保持原意的 counterpart，不允许改变技术决定。

如果代码变更要求更新 human-controlled 区域，但用户没有授权，应停止并询问用户；缺少授权属于 code review 阻塞问题。

当任务范围内的实现或开发流程发生实质变化时，agent 可以同步更新 `docs/how/` 和 `docs/contributing/`。

## 根据变化选择文档层级

| 变化 | 文档 owner |
|---|---|
| 目标、非目标、系统边界或已接受的取舍 | `docs/intent/` |
| 使用者可见行为、schema、错误、顺序或兼容性 | `docs/specs/` |
| `crate` 结构、算法、数据流、内部接口或调试入口 | `docs/how/` |
| 测试或贡献流程 | `docs/contributing/` |

文档中的事实没有变化时，不做形式性的文档修改。

## 按语义维护双语文档

除 `docs/manifest.toml` 声明的类别外，正式 Markdown 使用相邻的英文 `.md` 与简体中文 `.zh-CN.md` counterpart。每轮编辑可以选择任一 source language；先核验其中的事实，再以相同技术层次写出自然的 counterpart。

翻译状态只有三种：

- `synced`：两个 counterpart 都存在，并且技术语义一致；
- `pending`：已经声明的 counterpart 尚未编写；
- `needs-update`：两个文件都存在，但已知其中一个落后。

`pending` 和 `needs-update` 用来暴露中间阶段的翻译债务。一组文档完成时必须恢复为 `synced`。只有重新核验技术事实后才能更新 `last_verified`；移动文件、翻译或只修改表达不能更新该日期。

运行 `just docs-check` 检查分类、metadata、counterpart 和本地链接。语义一致性仍然需要 review。

## 从读者问题出发

- 从读者需要回答的问题开始，使用直接、具体的句子。
- 技术结论必须来自代码、测试、schema、运行时证据或 human 明确作出的决定，不得从实现结构反推设计意图。
- 在结论附近提供少量有用的实现或测试入口。本次核验过程和命令记录放在 PR 中。
- 两种语言应保持相同的范围、确定程度、保证、限制、失败行为、示例和生命周期状态，同时分别使用自然表达。
- 删除无助于回答核心问题的模板章节、总结和历史沉积。

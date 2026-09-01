---
doc_id: intent.project
title: "项目意图"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 项目意图

[English](README.md)

## 为什么建设这个项目

交互式公式体验需要语法反馈、编辑辅助和求值对公式含义保持一致。`notion-formula-rs` 围绕同一套公式语言定义提供这些能力，使集成方不必自行协调彼此独立的 parser、type rule 和 runtime。

项目优先保证行为确定、过程可理解、支持边界明确。Notion-style formula 是语言词汇的起点，但项目更重视在已声明范围内保持正确，不以宣称完整复刻其他产品为目标。

## 它服务谁

项目服务于把公式编写能力嵌入 editor 或 application 的团队，包括通过 JavaScript/WASM boundary 接入的 browser integration，也服务于在这些集成中编写和求值公式的人。

## 目标与非目标

项目的目标是提供一致的公式语言、编写过程中的有效 analysis、行为可预期的 editor services，以及对受支持公式进行求值的能力。

项目不是面向终端用户的独立产品，也不负责公式存储、持久 formula identity、rename transaction、部署或运维。完整 Notion compatibility 不是目标，Rust `pub` item 也不会自动成为稳定的 integration contract。

## 边界与重新评估

当前边界把公式行为和 JavaScript/WASM integration surface 视为面向使用者的承诺；Rust implementation interface 则可以随 workspace 演进。

如果项目需要承诺稳定 Rust API、管理 formula identity 或 persistence、保证与上游语言严格兼容，或成为独立运营的产品，就应重新评估这一边界。这些变化会引入新的使用者和责任，而不只是扩展现有实现。

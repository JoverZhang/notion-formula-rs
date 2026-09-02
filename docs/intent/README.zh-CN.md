---
doc_id: intent.project
title: "项目意图"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 项目意图

[English](README.md)

## 为什么做这个项目

AI 已经能直接写代码和 SQL。Formula 这种交互方式还能存在多久，我没有答案。我还是想试着做一个基于静态类型推断和列式计算的 Formula 引擎。如果它恰好能解决你的问题，我会感到荣幸。

我相信，随着定制工具的成本不断降低，越来越多人会定制 DSL，用来表达自己的业务规则和开发约束（这个判断并不取决于 Formula 是否流行）。

`notion-formula-rs` 已经用一套内部 DSL 声明 builtin functions，并为这套 DSL 写了一个轻量级编译器。以后我还想为行为、规格和实现约束设计新的 DSL，尝试类似 BDD 的做法。DSL-driven development 目前还在设想阶段。

如果这个 Formula 引擎没能帮到你，我仍希望项目里的 DSL 实践能带来一些启发：如何为具体问题设计一门小语言，以及如何用它声明规则、生成代码和约束实现。

## 当前要做成什么

`notion-formula-rs` 为类似 Notion Database 的多维表格系统提供 Formula 引擎。项目希望同一份 Formula 可以在服务端和 Web 端运行。服务端通过列式计算处理成批数据。Web 端在用户编辑时直接完成分析和求值，让错误、类型和计算结果及时显示出来。两端使用同一套语法和计算规则。

## 项目边界

Notion Formula 是这个项目的起点。项目会尽可能兼容它的语法和求值行为。静态类型推断会在公式运行前检查类型，并尽可能给出明确的结果类型。

Formula 之外的字段由宿主系统定义、存储并提供值。`notion-formula-rs` 提供字段类型和值的接入方式，让 Formula 可以引用这些字段。项目不实现 Notion Database 的其他功能。

## 一份私人的动机

项目里与 IDE 和 LSP 原理有关的工作，也有我的一点私心。我想趁这些技术在这个世界上尚有最后的余温，自己做上一遍，看看它们究竟是怎么工作的，满足青春时留下的好奇心。

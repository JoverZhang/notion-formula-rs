---
doc_id: contributing.changelogs
title: "如何编写 changelog 条目"
language: zh-CN
source_language: en
counterpart: ./changelogs.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 如何编写 changelog 条目

[English](changelogs.md)

一次修改会改变使用者或集成方能够观察到的行为时，应在 [`docs/changelogs/`](../changelogs/) 下新增文件。
Changelog 条目保留某个时间点发生了什么变化；Specs 负责说明系统的 Current contract。

## 判断是否需要条目

以下变化需要条目：

- 使用者可见的 parser、diagnostic、completion、signature help、formatting 或 evaluation 行为；
- Public Rust/WASM API 或 DTO；
- schema、坐标、顺序、确定性或受控错误的兼容性；
- 旧行为可能影响使用者公式或集成的 bug fix。

如果一次 refactor 不可能改变可观察输出，或者文档修改只修正措辞和链接，可以不写条目。不要把 changelog
当作开发日志。

## 为双语条目命名和写作

文件名使用 `YYYYMMDD-short-slug.md`，并在相邻位置建立
`YYYYMMDD-short-slug.zh-CN.md`。日期是落地日期；slug 使用小写和连字符，并直接指出变化的行为。起草时从
[`docs/_templates/`](../_templates/) 下的双语模板开始。

一份条目只说明一项变化：

- `Type` 取 `Added`、`Changed`、`Fixed`、`Removed` 或 `Security`；
- `Component` 写受影响的使用者可见区域；
- `Summary` 说明可观察到的差异；
- `Compatibility notes` 说明 breaking change、迁移方式，或者明确说明没有兼容性影响；
- `Links` 指向 PR、issue 或 Current Specs owner 等稳定背景资料。

优先使用读者可以搜索到的 public name。只有实现细节有助于解释兼容性时才写入。实际运行的命令和 review
证据属于 PR，不放进新 changelog 条目。

历史条目描述的是落地时的行为。可以修正失效链接或事实错误，但不要按照 Current 实现重写旧条目。双语版本
应保持相同的历史范围，只有语义一致时才能把这一对条目标记为 `synced`。

---
doc_id: specs.formula-references
title: "公式如何引用 property，重命名后会发生什么？"
language: zh-CN
source_language: en
counterpart: ./formula-references.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Formula reference

[English](formula-references.md)

这份 Current 规格说明公式如何命名 input property、集成方必须提供哪些 property input，以及 property rename 会产生什么结果。本文也会明确指出当前不存在 formula identity，避免把 property reference 误解成持久化的 formula reference。

## 使用完全一致的名称引用 property

Property reference 只有 `prop("Name")` 一种形式。`prop` 必须接收且只接收一个双引号 string literal。`prop("Na" + "me")` 这样的 computed string、非 string argument、额外 argument，或者 `value.prop("Name")` 这样的 postfix syntax，都不构成 property reference。

String literal 解码后的内容必须与 analysis 或 evaluation context 中的 property name 完全一致。匹配区分大小写：`prop("Title")` 与 `prop("title")` 是两个不同名称。Context 中不存在的名称会在 analysis 时产生 semantic error，使 expression 无法针对该 context 完成 preparation。[`validate_prop_call`](../../analyzer/src/analysis/mod.rs) 和 [`Context::lookup`](../../analyzer/src/analysis/mod.rs) 是这些规则在 analysis 中的实现入口。

同一个 context 中的 property name 必须唯一，才属于本文规定的输入。若集成方提供重复名称，系统选择哪一个 duplicate 没有规格保证，不得把当前偶然结果当作兼容性承诺。

## 提供所有静态引用的 property

Preparation 会在 row evaluation 之前检查完整 expression 中的 property reference。每个名称只出现一次，并按照它在 source 中第一次出现的顺序排列。重复写 `prop("A")` 不会产生第二项要求。

Reference discovery 不跟随 runtime branch selection。例如，`true ? prop("A") : prop("B")` 同时需要 `A` 与 `B`，即使 evaluation 只执行 `A` branch。集成方必须在 evaluation 开始前，按照预期 type 和 row layout 提供所有发现的 property。缺少 required property 会使整份 input 被拒绝；这与 property 已经存在、但某一 row 的 value 为 null 不同。

Required-property construction 的实现入口是 [`evaluator/src/planner/planner.rs`](../../evaluator/src/planner/planner.rs)。[`evaluator/tests/runtime_structure.rs`](../../evaluator/tests/runtime_structure.rs) 一起覆盖了完整发现、去重、按 source 首次出现排序，以及未选中 branch 中的 reference discovery。

## Rename 后更新 source 并重新执行 preparation

Property reference 在 formula source 中保存的是名称，不携带持久化的 property identity。因此，在 context 中重命名 property 不会改写 formula source，也不会重新定向已经 prepared 的 formula。

假设 context 中的 `Old` 已经改名为 `New`，不修改 source 而重新执行 analysis 或 preparation 时，`prop("Old")` 会成为 missing reference。集成方必须把 source 改为 `prop("New")` 并重新执行 preparation，才能引用新名称。此前 prepared 的 formula 只保留该次 preparation 产生的 input requirement，不会自动感知后续 context change。

这条规则避免产生并不存在的 identity guarantee：host application 可以自行实现 rename transaction，但当前项目既不协调这项 transaction，也不承诺 property name 改变后 reference 仍保持不变。

## Production contract 中不存在并存的 Formula ID 与 Formula name

Production analyzer、editor service、WASM boundary 和 evaluator 会接收 formula source 与 property context，但没有定义 `FormulaId`、`FormulaName`、formula-rename API 或持久化的 formula-reference DTO。因此，当前没有任何 production feature 要求 Formula ID 与 Formula name 并存；也谈不上从 production identity model 中移除其中一个，因为这个 model 尚不存在。

[`examples/vite/src/app/types.ts`](../../examples/vite/src/app/types.ts) 中的 `FormulaId` 只用于选择 demo 里的 formula panel。它属于 example UI state，不是 formula identity contract。若 application 需要存储 formula、在 formula 被重命名前后保持稳定 identity，或让 formula 相互引用，就必须在当前项目边界之外定义这些能力。

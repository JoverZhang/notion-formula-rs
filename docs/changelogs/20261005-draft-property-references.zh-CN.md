---
doc_id: changelog.20261005-draft-property-references
title: "提供结构化 Draft 属性引用信息"
language: zh-CN
source_language: en
counterpart: ./20261005-draft-property-references.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 提供结构化 Draft 属性引用信息

[English](20261005-draft-property-references.md)

- Type: Added
- Component: FormulaDraft 与 WASM API

## Summary

Draft 快照现在提供解码后的属性 ID，以及调用和字符串字面量的准确区间。编辑器可以保持字段绑定，替换 ID 时保留注释，无需再实现公式解析器。原生区间采用 UTF-8，WASM 区间采用 UTF-16；表达式其他位置出错时，仍会返回完整的属性引用。

## Compatibility notes

FormulaDraftState 快照新增必填字段 `property_references`。原生结构体构造和穷尽匹配需要相应调整；手动构造的 TypeScript 测试数据也须包含该字段，没有引用时使用 `[]`。使用这些快照的客户端须更新 SDK。

## Links

- [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
- [Draft 引用契约](../specs/ide.zh-CN.md#formuladraft)

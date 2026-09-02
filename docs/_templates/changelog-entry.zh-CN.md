---
doc_id: template.changelog-entry
title: "Changelog 条目模板"
language: zh-CN
source_language: en
counterpart: ./changelog-entry.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Changelog 条目模板

[English](changelog-entry.md)

先为本轮编辑选择 `en` 或 `zh-CN` 作为 source language，并在两份 counterpart 中使用同一个值。把下面的
结构复制到 `docs/changelogs/YYYYMMDD-short-slug.zh-CN.md`，替换所有 placeholder，再在相邻位置建立英文
counterpart。

```markdown
---
doc_id: changelog.YYYYMMDD-short-slug
title: "简短说明"
language: zh-CN
source_language: <en-or-zh-CN>
counterpart: ./YYYYMMDD-short-slug.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: YYYY-MM-DD
---

# 简短说明

[English](YYYYMMDD-short-slug.md)

- Type: Added | Changed | Fixed | Removed | Security
- Component: 使用者可见区域

## Summary

用一到两句话说明可观察到的变化。

## Compatibility notes

- 说明 breaking change 和迁移方式，或者明确说明没有兼容性影响。

## Links

- PR、issue 或 Current Specs owner
```

---
doc_id: experiments.notion-empty-semantics
title: "Notion 的 empty() 与空值行为"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: historical
document_status: stable
translation_status: pending
last_verified: 2026-09-28
---

# Notion 的 `empty()` 与空值行为

无参 `empty()` 返回 null，行为类似 JavaScript 的 `null`；具体差异见下方实验。

本次于 2026-09-28 使用 Notion API `2025-09-03` 验证。`N`、`T`、`D` 分别是未填写的 Number、Text、Date 属性；Boolean 空值由 `if(false, true, empty())` 产生。对照值为 `0`、`""`、`false`、Unix epoch 和 `[]`。完整结果见 [results-2026-09-28.json](results-2026-09-28.json)。

## 关键结果

- 空 Number 与 `0`、空 Text 与 `""`、空 Boolean 与 `false` 能被相等比较区分。
- `mean([2, 空 Number, 4])` 为 `3`，替换成 `0` 后为 `2`。
- 列表保留空元素；`map` 保留空结果的位置；`join` 保留对应分隔符。
- `map(empty(), current)` 与 `join(empty(), ",")` 在设置公式时返回类型错误。

## API 中的列表结果

将 `[1, empty(), 2]` 保存为 Formula 属性，调用 `GET /v1/pages/{page_id}`，读取 `properties[formula_name].formula`，实际得到：

```json
{"type": "string", "string": "1,2"}
```

这是 API 返回的字段，脚本未调用 `join` 或自行拼接。Notion 的 [Formula 响应类型](https://developers.notion.com/reference/page-property-values#formula)没有数组；本例返回了字符串。空元素被省略的具体转换机制未公开。

对同一列表另建公式进行求值：

```text
length([1, empty(), 2])                → 3
join(map([1, empty(), 2], index), ",") → "0,1,2"
join([1, empty(), 2], ",")             → "1,,2"
```

求值时空位置仍存在，不能用直接返回的字符串判断列表长度。记录范围为当日 Notion 的可观察行为；本项目采用的规则由[语言规格](../../specs/formula-language.zh-CN.md)维护。

## 复现

设置环境变量 `NOTION_TOKEN` 和 `NOTION_PARENT_PAGE_ID`，后者为 integration 已获授权的父页面 ID。在仓库根目录运行：

```sh
python3 docs/experiments/notion-empty-semantics/probe.py --output /tmp/notion-empty-results.json
```

脚本创建独立实验页面、数据库和空值／默认值控制行。结果文件只保存实验条件、公式、Formula 返回字段和预期错误，不输出工作区链接、资源 ID 或凭据。复测使用新的日期文件，保留既有记录。

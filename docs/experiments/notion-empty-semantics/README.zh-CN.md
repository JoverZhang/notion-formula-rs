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

实验目的：确定复现 Notion 行为需要区分哪些求值状态。以下是 2026-09-28、Notion API `2025-09-03` 的实测；完整公式、输入和响应保存在[结果文件](results-2026-09-28.json)。

## 正常值与空值

`N`、`T`、`D` 是未填写的 Number、Text、Date 属性。Boolean 空值由 `if(false, true, empty())` 产生；日期对照取 Unix epoch。表中的 `N`、`T`、`D` 简写属性读取；`L` 对照 `if(false, [1], empty())` 与 `[]`。

| 观察 | 空值 | 类型默认值 |
|---|---|---|
| `N == 0` | `false` | `true` |
| `length(unique([N, 0]))` | `2` | `1` |
| `mean([2, N, 4])` | `3` | `2` |
| `T == ""` | `false` | `true` |
| Boolean 与 `false` 比较 | `false` | `true` |
| `empty(D)` | `true` | `false` |
| `length(flat([L]))` | `1` | `0` |

空值必须保留。`empty(N)` 和 `N + 1` 却分别与 `0` 的结果相同：`true`、`1`，单测这些操作会漏掉区别。

## 列表中的空位与公式创建

列表中的空元素保留索引；`map` 回调返回空值也保留该位置。

```text
length([1, empty(), 2])                              → 3
join(map([1, empty(), 2], index), ",")                 → "0,1,2"
join([1, empty(), 2], ",")                            → "1,,2"
length(map([1, 2, 3], if(current == 2, empty(), current))) → 3
join(map([1, 2, 3], if(current == 2, empty(), current)), ",") → "1,,3"
```

下面的对照区分创建阶段的类型拒绝与通过检查后的 null 输出：

```text
empty().map(current)                     → HTTP 400 validation_error: Type error with formula
if(false, [], empty()).map(current)      → {"type":"string","string":null}
if(false, [], empty()).map(current).length() → {"type":"number","number":null}
```

## 非法正则的求值结果

`Pattern="a"` 时 `test("abc", prop("Pattern"))` 为 `true`；`Pattern="["` 时该公式已创建，但求值结果为 `{"type":"boolean","boolean":null}`。以下是非法正则行的对照：

```text
empty(empty())                                      → true
empty(test("abc", prop("Pattern")))                → null
format(empty())                                     → ""
format(test("abc", prop("Pattern")))               → null
if(false, test("abc", prop("Pattern")), true)       → true
[test("abc", prop("Pattern"))].length()            → null
[1].map(test("abc", prop("Pattern"))).length()    → 1
[1].map(test("abc", prop("Pattern"))).join(",") → ""
```

这组执行失败在 API 中输出 `null`，但公式中的传播与普通空值不同。`Divisor=0` 的对照也返回 `number:null`，但 `empty(1 / prop("Divisor"))` 为 `true`，`format(1 / prop("Divisor"))` 为 `""`。API 中的 `null` 同时覆盖了不同的求值状态。

## API 返回字段

将 `[1, empty(), 2]` 保存为公式，通过 `GET /v1/pages/{page_id}` 读取 `properties[公式属性名].formula`，得到：

```json
{"type":"string","string":"1,2"}
```

同一列表的 `length` 为 `3`、`join` 为 `"1,,2"`。这个字符串是 API 的输出表示，不能用它还原列表结构。实验只记录当日 Notion 的外部行为；本项目采用的规则见[语言规格](../../specs/formula-language.zh-CN.md)。

## 复现

设置 `NOTION_TOKEN` 和 `NOTION_PARENT_PAGE_ID`，后者为 integration 已获授权的父页面 ID。在仓库根目录运行：

```sh
python3 -B docs/experiments/notion-empty-semantics/probe.py --output /tmp/notion-empty-results.json
```

脚本创建独立实验页面、数据库和带明确输入的对照行。`rows` 标注各输入行及求值结果；`formula_creation_errors` 是定义阶段被拒绝的公式，不对应数据行。输出不包含页面链接、资源 ID 或凭据。复测使用新的日期文件，保留既有记录。

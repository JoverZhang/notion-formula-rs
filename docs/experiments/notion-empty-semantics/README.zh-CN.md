---
doc_id: experiments.notion-empty-semantics
title: "Notion 的 empty() 与空值行为"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-09-28
---

# Notion 的 `empty()` 与空值行为

[English](README.md)

无参 `empty()` 返回 null，行为类似 JavaScript 的 `null`。本实验关注空值如何参与求值，以及它与执行失败在输出上的区别。

实测日期：2026-09-28；Notion API：`2025-09-03`。完整输入与响应见[结果文件](results-2026-09-28.json)。

## 空值

列表中的空元素占据位置，`map` 返回空值时也保留该位置：

```text
length([1, empty(), 2]) → 3
join(map([1, empty(), 2], index), ",") → "0,1,2"
join(map([1, 2, 3], if(current == 2, empty(), current)), ",") → "1,,3"
```

具有列表类型上下文的空值可以通过类型检查，求值结果为空：

```text
if(false, [], empty()).map(current) → null
if(false, [], empty()).map(current).length() → null
```

## 输出表示

直接 API 输出取自 `GET /v1/pages/{page_id}` 的 `properties[公式属性名].formula`。根据实测，列表输出可用下面的模型解释：

```js
const values = arr.flat(Infinity).filter(value => value !== null);
return values.length === 0 ? null : values.join(",");
```

这是对 Notion database API 列表输出逻辑的推测。JS `null` 对应 Notion 的 `empty()`；模型覆盖下列已测值，不延伸为其他类型的字符串转换规则。

| 数组表达式 | API 返回值 | 观察 |
|---|---|---|
| `[[[1, 2]], 3]` | `"1,2,3"` | 递归展开 |
| `[[1, empty(), 2]]` | `"1,2"` | 过滤空值 |
| `[[[]]]` | `null` | 展开后无元素 |
| `[[""], 1]` | `",1"` | 保留空字符串 |
| `[[0], [false]]` | `"0,false"` | 保留零和 false |

显式 `format`、`join` 保留空元素对应的分隔符，行为与直接 API 输出不同：

```text
[1, empty()]           → API: "1"
format([1, empty()])   → "1,"
join([1, empty()], ",") → "1,"
```

## 运行时失败

公式 `test("abc", prop("Pattern"))` 可以通过类型检查。`Pattern="a"` 时返回 `true`；输入改为非法正则 `"["` 后，API 返回：

```json
{"type":"boolean","boolean":null}
```

虽然输出也是 null，失败结果与普通空值的后续行为不同。以下均使用 `Pattern="["`：

```text
empty(empty()) → true
empty(test("abc", prop("Pattern"))) → null
format(empty()) → ""
format(test("abc", prop("Pattern"))) → null
```

外层操作也影响失败如何传播：

```text
if(false, test("abc", prop("Pattern")), true) → true
[test("abc", prop("Pattern"))].length() → null
[1].map(test("abc", prop("Pattern"))).length() → 1
[1].map(test("abc", prop("Pattern"))).join(",") → ""
```

因此，API 的 null 输出本身不足以区分普通空值与运行时失败。

---

**类型检查。** `empty().map(current)` 在创建公式时返回 HTTP 400 `validation_error`，尚未进入求值。结果文件中的 `formula_creation_errors` 单独记录此类拒绝。

**复现。** 设置环境变量 `NOTION_TOKEN` 和 `NOTION_PARENT_PAGE_ID`（integration 已获授权的父页面 ID），在仓库根目录运行：

```sh
python3 -B docs/experiments/notion-empty-semantics/probe.py --output /tmp/notion-empty-results.json
```

脚本创建独立实验页面和数据库。`rows` 记录具名输入行及求值结果；`list_rendering` 行用 `operation` 区分输出路径。输出不含私人资源标识或凭据。复测使用新的日期文件，保留既有记录。

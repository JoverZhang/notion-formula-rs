---
doc_id: experiments.notion-list-repeat-semantics
title: "Notion 列表空值与 repeat 次数"
language: zh-CN
source_language: zh-CN
counterpart: ./README.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-09-29
---

# Notion 列表空值与 repeat 次数

[English](README.md)

本次测试的 Notion Database 公式中，聚合函数忽略空 Number，`sort` 移除 null 元素，`flat` 保留 null 元素。测试覆盖的有限 `repeat` 次数表现为：负数归零，正小数向上取整，最多重复 10,000 次。这些实验为 [issue #53](https://github.com/JoverZhang/notion-formula-rs/issues/53) 的运行时语义提供观察依据。

实验日期为 2026-09-29，使用 `Notion-Version: 2025-09-03`。[首轮结果](results-2026-09-29.json)与[补充结果](results-2026-09-29-followup.json)保留了 `GET /v1/pages/{page_id}` 返回的带类型结果。39 行均读取两次，509 个公式结果在两次读取中全部一致。提交的 217 个公式定义中，43 个在创建时被拒绝，单独记录。

## 聚合函数忽略有类型的 null

`sum([2, empty(), 4])` 以及对应的 `mean`、`median`、`min`、`max` 表达式均被拒绝，响应为 HTTP 400、`validation_error`、`Type error with formula`。直接传入未确定元素类型的 `[]` 或全部由 `empty()` 构成的参数，也得到相同响应。这些是创建阶段的观察，不能当作运行时 null 规则。

补测使用了三个能够通过类型检查、API 结果为 null 的 Number 表达式：空 Number 属性 `N`、`if(false, 1, empty())`、`find([1], current < 0)`。属性回读确认为 `number: null`。三种空值来源在 `[2, source, 4]` 中都保留位置，列表长度均为 3。

| 函数 | `[2, null, 4]` | `[-2, null, -4]` | `[null]` | 有类型的空列表 |
| --- | ---: | ---: | ---: | ---: |
| `sum` | 6 | -6 | 0 | 0 |
| `mean` | 3 | -3 | API null | API null |
| `median` | 3 | -3 | API null | API null |
| `min` | 2 | -4 | API null | API null |
| `max` | 4 | -2 | API null | API null |

表中的 `null` 代指上述三个有类型的空值来源，不是公式字面量。有类型的空列表使用 `filter([1], current < 0)` 构造。三种来源的结果一致，`F(2, source, 4)` 也与列表形式一致。零值对照可以区分“忽略空值”和“空值转零”：`mean([2, 0, 4])`、`median([2, 0, 4])` 返回 2，`min([2, 0, 4])` 返回 0。

`Pattern="["` 时，`sum([2, if(test("abc", prop("Pattern")), 1, 1), 4])` 返回 API null，外层的 `empty` 和 `format` 也返回 API null。改用合法正则 `"a"` 后，聚合结果为 7。因此，这个实际执行失败与上述普通 null 来源表现不同；该观察不能证明所有函数族的失败都采用相同传播规则。

## sort 移除 null，flat 保留 null

结构观察使用 `length` 和按下标读取的 `at`，再用 `join` 及为元素添加标记的 `map` 交叉验证。[此前的实验](../notion-empty-semantics/README.zh-CN.md)说明了为什么不能只看 API 对列表的直接显示结果。

`sort` 案例使用 `empty()` 元素；`flat` 案例使用 `empty()` 和列表类型的空值 `if(false, [1], empty())`。这些案例没有覆盖所有可能的有类型空值来源。

| 表达式 | 结构观察 |
| --- | --- |
| `sort([2, empty(), 1])` | 长度为 2，元素为 1、2 |
| 把 null 移到首位或末位 | 排序结果相同 |
| `sort([1, empty(), 0, -1, empty()])` | 长度为 3，元素为 -1、0、1；零被保留 |
| `sort([empty(), empty()])` | 长度为 0 |
| `flat([1, empty(), 2])` | 长度为 3；用竖线连接得到 `"1\|\|2"` |
| `flat([[1, empty()], [2]])` | 长度为 3；按下标读取为 1、API null、2 |
| `flat([[empty()], []])` | 长度为 1，元素为空值 |
| `flat([[], empty(), [2]])` | 长度为 2，空值位于 2 之前 |

深度对照区分了单层展开与递归展开：`length(flat([[[]]]))` 返回 1，`join(map(flat([[[]]]), length(current)), "|")` 返回 `"0"`。保留下来的元素是空列表。这与仓库 [Builtin function signatures](../../specs/builtin-functions.zh-CN.md) 当前规定的递归 `flat` 不同。

## repeat 对测试中的有限次数取整并限制上限

实验同时使用了字面量次数和 Number 属性。文本为 `"ab"` 时：

| 次数 | 观察结果 |
| --- | --- |
| -2、-1.9、-1、-0.9、-0.1、0 | 空字符串；长度为 0；`empty(...)` 为 true |
| 0.1、0.9、1 | `"ab"` |
| 1.1、1.9 | `"abab"` |
| 2.9 | `"ababab"` |
| 9,999 | 长度为 19,998 |
| 10,000 | 长度为 20,000 |
| 10,001、100,000、1,000,000、1,000,001 | 长度为 20,000 |

补测中，文本 `"x"` 的次数分别为 9,999.1、10,000、10,000.1、10,001，结果均为恰好 10,000 个 `x`。次数为 10,001 时，`"abc"` 的结果长度为 30,000，狐狸 emoji 重复了 10,000 次。API 直接返回的字符串与重复次数、公式内部测得的长度一致。因此，这些案例支持“重复次数上限”，而不是 API 截断或固定的输出长度上限。

已测试的有限次数符合以下实验模型：

```text
repetitions = min(10000, max(0, ceil(count)))
```

Count 属性为空时，重复结果的长度、`empty` 检查和预览都返回 API null。次数表达式为 `0 / 0`、`1 / 0`、`-1 / 0` 时也如此；这些案例没有把 `repeat` 与次数表达式本身的求值分离。直接用 `empty()` 作为次数会在创建时被拒绝。文本为空时，次数 3、-1、1,000,001 均得到空字符串。

有限次数的观察结果与实验时计划中的非负约束，以及当时 evaluator 的小数截断、1,000,000 次上限不同。
随后采用的契约见 [Planned repeat](../../specs/builtin-functions.zh-CN.md#planned-repeat)；本实验保留原始观察，不定义当前契约。

## 复现与解释限制

[官方函数说明](https://www.notion.com/help/formula-syntax)没有定义这些边界情况。仅凭 API null 无法判断内部是普通 null、非有限数还是执行失败，这限制了对全空聚合结果的解释。参见 [formula property values](https://developers.notion.com/reference/page-property-values#formula)。

设置 `NOTION_TOKEN` 和 `NOTION_PARENT_PAGE_ID`，然后从仓库根目录运行。未设置 token 环境变量时，探针会以不回显的方式读取；未设置父页面 ID 时，只搜索标题为 `Notion empty semantics probe` 的既有页面。

```sh
python3 -B docs/experiments/notion-list-repeat-semantics/probe.py \
  --output /tmp/notion-list-repeat-initial.json
python3 -B docs/experiments/notion-list-repeat-semantics/probe.py \
  --cases docs/experiments/notion-list-repeat-semantics/followup-typed-nulls.json \
          docs/experiments/notion-list-repeat-semantics/followup-repeat-boundary.json \
  --output /tmp/notion-list-repeat-followup.json
```

每次运行都会新建独立的实验页面和数据库。输出包含表达式、提交的输入、带类型的响应、创建错误及重复读取的比较结果；补测还保存了从 API 回读的实际输入。凭据和私有资源 ID 不进入输出。探针拒绝覆盖既有结果；重跑时使用新的日期文件名并保留已有记录。

---
doc_id: experiments.notion-lets-semantics
title: "Notion 的 lets 允许重复变量名吗？"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-11
---

# Notion 的 lets 允许重复变量名吗？

[English](README.md)

**允许：被测 Notion 数据库公式接受在同一个 `lets` 中重复定义变量名，后续引用使用后面的绑定。** 本次关注的 `lets(a, 1, label, "v", a, a + 1, label + format(a))` 创建成功并返回 `"v2"`。第二次绑定 `a` 时，右侧表达式读取此前的值 1。

实验于 2026-10-11 进行，使用 `Notion-Version: 2025-09-03`。[脱敏原始结果](results-2026-10-11.json)保留公式创建错误、存储输入，以及 `GET /v1/pages/{page_id}` 返回的类型化公式结果。14 个定义中，12 个创建成功，2 个被拒绝。4 行均读取两次，16 个公式结果全部一致。[用例文件](cases.json)包含完整实验矩阵。

## 重复绑定、右侧取值与作用域

以下表达式均成功创建为数据库公式属性，并返回对应结果：

| 表达式 | 观察到的 API 结果 |
| --- | --- |
| `lets(a, 1, a, 2, a)` | Number `2` |
| `lets(a, 1, label, "v", a, a + 1, label + format(a))` | String `"v2"` |
| `lets(a, 1, a, a + 1, a, a * 3, a)` | Number `6` |
| `lets(a, 1, a, "v" + format(a), a)` | String `"v1"` |
| `lets(a, 1, before, a, a, 2, format(before) + ":" + format(a))` | String `"1:2"` |
| `let(a, 7, lets(a, 1, a, a + 1, a) + a)` | Number `9` |
| `let(a, 7, lets(a, a + 1, a))` | Number `8` |
| `lets(a, 1, b, a + 1, a + b)` | Number `3` |

常量重复绑定样例单独验证了调用被接受、最终引用使用新值；目标公式和连续自引用样例进一步验证右侧表达式可以读取此前的绑定。Number 改为 String 的样例表明，被测重复绑定也允许改变类型。`a` 变为 2 后，`before` 仍为 1，支持“新绑定依次遮蔽同名旧绑定”的解释，先前保存的值没有被改写。两个外层 `let` 对照分别验证右侧表达式能访问外层绑定，以及内层调用结束后外层值仍然保留。

目标公式的直接 API 响应为 `{"type":"string","string":"v2"}`。不重复绑定的对照 `lets(a, 1, label, "v", label + format(a))` 返回 `"v1"`。嵌套对照 `let(a, 1, let(label, "v", let(a, a + 1, label + format(a))))` 返回 `"v2"`。

属性输入对照与字面量结果一致。`lets(a, prop("Count"), label, prop("Text"), a, a + 1, label + format(a))` 和 `let(a, prop("Count"), let(label, prop("Text"), let(a, a + 1, label + format(a))))` 均返回以下 String：

| 存储的 Count | 存储的 Text | 两式结果 |
| ---: | --- | --- |
| 1 | `"v"` | `"v2"` |
| 0 | `"zero:"` | `"zero:1"` |
| -3 | `"数:"` | `"数:-2"` |

未绑定引用对照 `lets(a, a + 1, a)` 和 `lets(a, b + 1, b, 2, a)` 在安装公式时被拒绝，返回 HTTP 400 `validation_error`、`Type error with formula`，没有进入逐行求值。这些拒绝结果将“没有可用的先前绑定或外层绑定”与上述被接受的重复变量名情况区分开来。

## 官方资料说明到哪里

2026-10-11 核对的[官方函数说明](https://www.notion.com/help/formula-syntax)将 `let` 描述为给表达式中的一个变量赋值，将 `lets` 描述为给多个变量赋值。示例包括返回 `"Hello world"` 的 `lets(a, "Hello", b, "world", a + " " + b)`，以及另一个包含两个 Number 变量的表达式。

这些示例说明了普通的多变量用法，但没有规定重复变量名、右侧表达式能否读取此前的绑定，以及绑定按什么顺序可见。文档也没有明确定义混合类型的顺序绑定、嵌套词法作用域，或与嵌套 `let` 的等价关系。因此，目标公式需要实验证据，不能直接用假定的通用展开规则推导其结果。

[官方公式属性文档](https://developers.notion.com/reference/page-property-values#formula)描述了只读的求值结果，其类型为 `string`、`number`、`boolean`、`date` 或 `unsupported`。没有值时，受支持的结果可以包含 null。这些 API 对象用于观察结果，不定义 `lets` 的变量查找规则，也不解释 null 产生的原因。

## 限制与复现

这些观察支持被测数据库公式采用依次遮蔽同名绑定的行为。字面量和属性输入的嵌套对照结果相同，不代表已证明所有 `lets` 调用都等价于嵌套 `let`。实验没有检查公式编辑器，也没有穷尽变量类型与嵌套场景；本记录不修改仓库的运行时或规范。

设置 `NOTION_TOKEN` 与 `NOTION_PARENT_PAGE_ID`，在仓库根目录运行[探针](probe.py)：

```sh
python3 -B docs/experiments/notion-lets-semantics/probe.py \
  --output /tmp/notion-lets-results.json
```

未设置 token 环境变量时，探针通过不回显的提示读取凭据；未设置 parent ID 时，通过[已有数据库辅助函数](../notion-list-repeat-semantics/probe.py)查找先前的实验页。相邻[用例文件](cases.json)定义字面量与属性输入检查。探针创建独立的实验页和两个数据库，安装公式定义，并将每行读取两次。保存的记录排除凭据与私有资源 ID，拒绝覆盖已有文件；复测请采用新的日期文件名并保留原始证据。

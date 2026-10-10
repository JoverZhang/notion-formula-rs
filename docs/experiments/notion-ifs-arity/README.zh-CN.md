---
doc_id: experiments.notion-ifs-arity
title: "Notion 的 ifs 必须提供 else 吗？"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-10-10
---

# Notion 的 ifs 必须提供 else 吗？

[English](README.md)

**不必须：被测 Notion Database 公式接受省略最后 else 的 `ifs` 调用。** 2、4、6 个参数的调用返回首个命中条件对应的值。全部条件未命中时，被测 Number 结果在 `empty`、`format` 和列表观察公式中表现为普通空值。

实验于 2026-10-10 进行，使用 `Notion-Version: 2025-09-03`。[原始结果](results-2026-10-10.json)保留公式创建错误、存储输入，以及 `GET /v1/pages/{page_id}` 返回的类型化公式结果。51 个定义中，43 个创建成功，8 个被拒绝。6 行均读取两次，75 个公式结果全部一致。[用例文件](cases.json)包含完整实验矩阵。

## 省略 else 的调用被接受

以下表达式均成功创建为数据库公式属性，并在条件命中时返回对应的值：

| 表达式 | 观察到的 Number 结果 |
| --- | ---: |
| `ifs(true, 11)` | 11 |
| `ifs(false, 11)` | API null |
| `ifs(true, 11, false, 22)` | 11 |
| `ifs(false, 11, true, 22)` | 22 |
| `ifs(true, 11, true, 22)` | 11 |
| `ifs(false, 11, false, 22)` | API null |
| `ifs(false, 11, false, 22, true, 33)` | 33 |
| `ifs(false, 11, false, 22, false, 33)` | API null |

显式默认值对照也正常工作：`ifs(false, 11, 99)`、`ifs(false, 11, false, 22, 99)` 及对应的七参数调用均返回 99；`ifs(true, 11, 99)` 返回 11。

属性条件与字面量条件的结果一致。对于 `ifs(prop("Count") > 0, 11, prop("N") > 0, 22)`，输入 `(Count, N)` 为 `(1, 0)`、`(0, 1)`、`(1, 1)`、`(0, 0)` 时，分别返回 11、22、11 和 API null。增加默认值 99 后，全部未命中时返回 99。

## 未命中的结果在观察中表现为空值

`ifs(false, 11)` 的直接 API 响应为：

```json
{"type":"number","number":null}
```

仅凭这个响应无法区分普通空值与求值失败。下面的观察公式可以区分本次被测情况；格式化观察在 `format(value)` 两侧添加尖括号，让空字符串可见。

| 被观察表达式 | `empty(value)` | `"<" + format(value) + ">"` |
| --- | --- | --- |
| `ifs(false, 11)` | `true` | `"<>"` |
| `ifs(false, 11, empty())` | `true` | `"<>"` |
| `empty()` | `true` | `"<>"` |
| `0` | `true` | `"<0>"` |
| `test("abc", prop("Pattern"))`，Pattern 为 `"["` | API null | API null |

非法正则作为实际执行失败的对照，其直接结果同样为 API null，但外层观察结果与未命中的 `ifs` 不同。列表观察进一步表明，未命中的值保留位置：`length([ifs(false, 11)])` 为 1，`join([1, ifs(false, 11), 2], "|")` 为 `"1||2"`。

其他未命中样例中，Text、Boolean、Date 分支分别返回 `string: null`、`boolean: null`、`date: null`。List 表达式 `ifs(false, [11, 22])` 被 API 表示为 `{"type":"string","string":null}`。这些表示不能确定 Notion 内部的值类型；上表与普通空值的对照只覆盖 Number 样例。

## 创建错误与分支执行对照

以下定义在安装公式时即被拒绝，尚未进行逐行求值：

| 定义 | HTTP 响应 |
| --- | --- |
| `ifs()`、`ifs(true)`、`ifs(false)`、`ifs(42)`、`ifs("fallback")` | 400 `validation_error`，`Type error with formula` |
| `if(true, 11)`、`if(false, 11)` | 400 `validation_error`，`Type error with formula` |
| `ifs(true,)` | 400 `validation_error`，`Parse error with formula` |

三参数 `if` 对照返回所选分支的值。因此，二参数 `if` 的拒绝结果不能用于判断二参数 `ifs`，后者在本实验中被接受。

属性条件下的失败对照支持被测调用会跳过未选中的表达式。Pattern 为 `"["` 时，`empty(ifs(prop("Count") > 0, test("abc", prop("Pattern"))))` 在 Count 为 0 时返回 `true`，Count 为 1 时返回 API null。命中后执行非法正则的结果，与无匹配时的空值可以区分。

Count 为正数时，`ifs(prop("Count") > 0, true, test("abc", prop("Pattern")), false)` 和 `ifs(prop("Count") > 0, true, test("abc", prop("Pattern")))` 都返回 `true`，后续条件或默认值中的非法正则没有阻止首项返回。Count 为 0 时，两者都返回 API null。把 Pattern 改为 `"a"` 后，直接正则对照返回 `true`；后续条件与默认值两种情况分别返回 `false` 和 `true`。

额外的边界样例也被接受：`ifs(1, 11)` 和 `ifs("true", 11)` 返回 11，`ifs(false, 11, 42, 22)` 返回 22。混合 Number/Text 分支随条件选择 11 或 `"hit"`。这些样例不足以定义通用的真假值或类型转换规则。

## 依据、限制与复现

[Notion 官方函数说明](https://www.notion.com/help/formula-syntax)描述了首个真条件的选择规则，并给出带默认值的示例，但未明确要求默认值，也未说明省略行为。本次公式创建与求值结果证实了被测数据库调用接受省略 else；实验没有检查 Notion 编辑器的签名展示，也没有穷尽所有参数数量与值类型。

[官方公式属性文档](https://developers.notion.com/reference/page-property-values#formula)描述了类型化 API 结果，并允许没有值的受支持结果为 null。如上面的空值与失败对照所示，API null 本身不能解释无结果的原因。本记录保留 API 观察，不推断 Notion 内部表示，也不修改本项目的运行时或规范。

设置 `NOTION_TOKEN` 与 `NOTION_PARENT_PAGE_ID`，在仓库根目录运行[探针](probe.py)：

```sh
python3 -B docs/experiments/notion-ifs-arity/probe.py \
  --output /tmp/notion-ifs-results.json
```

未设置 token 环境变量时，探针通过不回显的提示读取凭据；未设置 parent ID 时，查找此前名为 `Notion empty semantics probe` 的实验页。探针使用相邻的用例文件及[已有数据库辅助函数](../notion-list-repeat-semantics/probe.py)，创建独立的实验页和两个数据库，安装公式定义，并将每行读取两次。保存的记录排除凭据与私有资源 ID，拒绝覆盖已有文件；复测请采用新的日期文件名并保留原始证据。

---
doc_id: specs.editor-services
title: "编辑器服务提供哪些可依赖的行为？"
language: zh-CN
source_language: en
counterpart: ./editor-services.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# 编辑器服务

[English](editor-services.md)

这份 Current specification 规定公式作者和编辑器集成方可以依赖的 diagnostic、completion、signature help、
formatting 和 edit 行为。这些服务在公式尚未写完时仍会尽量提供结果，但 best-effort 结果不会让语法错误的源码
变成可求值公式。

本文规定的是服务行为，不是 transport。WASM API specification 负责导出方法名、DTO field、UTF-16 转换、
边界校验、序列化和 JavaScript error delivery；公式语法与求值语义按适用范围由 formula-language 和
builtin-function specification 共同规定。Vite 示例里的 completion popover、分组、焦点和 quick-fix 选择属于
example policy，不是编辑器服务的保证。

## 按服务返回的顺序处理 diagnostic

一次分析可以同时返回语法和语义问题。结果按以下三个阶段拼接：

1. parser diagnostic；
2. lexer diagnostic；
3. semantic diagnostic。

服务会保留这三个阶段的顺序，不会再按源码位置做全局排序，也不会对不同阶段产生的 diagnostic 做全局去重。
单个阶段可以合并自己在同一 span 上的报告；因此，consumer 应保留返回顺序，但不能把它理解成 source-order
sort。该顺序的实现入口是 [`analyzer::analyze`](../../analyzer/src/lib.rs)。

部分 parser recovery diagnostic 会带有用于插入、替换或删除源码的 action。Action 是可选信息：并非每个
parser diagnostic 都有 action，lexer 和 semantic diagnostic 目前都没有。Action edit 使用原文档位置，并
遵循下文统一的 edit 规则。Client 决定是否以及何时展示或应用它；分析过程不会自动应用 action。代表性的
recovery action 由 [`test_errors.rs`](../../analyzer/src/tests/parser/test_errors.rs) 覆盖。

Diagnostic message 是面向人的说明文字。完整英文句子不是机器可读的 compatibility key，集成方不能根据
message 全文分支。Transport 层的 diagnostic field 由 WASM API specification 规定。

## 根据 cursor context 请求 completion

Completion candidate 取决于 cursor 附近的源码，以及配置的 property 和受支持函数：

| Cursor context | Current candidate |
| --- | --- |
| 表达式起点，包括空 argument | 配置的 property、`not`、`true`、`false` 和受支持函数 |
| 严格位于 identifier、`not`、`true` 或 `false` 内部，或者 Current classifier 把 identifier 识别为配置 property name、受支持 function name、`not`、`true` 或 `false` 的前缀 | 上述 expression-start candidate |
| 完整 identifier、literal 或 `)` 之后，且不属于前缀补全 | `==`、`!=`、`>=`、`>`、`<=`、`<`、`+`、`-`、`*`、`/` 和支持 postfix 的函数 |
| receiver 与 `.` 之后 | 已知 receiver type 可接受的 postfix 函数 |
| 严格位于 string literal 内部 | 不返回 completion candidate |

源码只包含 lexer 会跳过的水平 whitespace，即 space、tab 或 carriage return 时，属于 expression-start 规则的
Current 例外。Cursor 位于 `0` 时返回 expression-start candidate；cursor 位于其他位置时不返回 candidate。
Newline 会作为 trivia 保留下来，不触发该例外。

对于 property 和 function name，prefix classifier 会比较转为小写后的 prefix，却以原始拼写判断 exact name。
因此，与配置完全相同的 mixed-case name（例如 `Title`）仍可能走 prefix 路径，完全相同的 lowercase name 则
不会。Identifier prefix completion 会把识别到的 identifier 作为 replacement range，普通插入位置使用 cursor
处的空 range。

Receiver type 未知时，member completion 目前保留全部 postfix-capable 函数；type 已知时，会移除其 receiver
parameter 无法接受该 type 的函数。`.` 后的 query 还会进一步移除不匹配的函数。这些边界由
[`test_completion_position.rs`](../../ide/src/tests/ide/test_completion_position.rs) 和
[`test_completion_ranking.rs`](../../ide/src/tests/ide/test_completion_ranking.rs) 覆盖。

Parser 还接受 `%`、`^`、`&&` 和 `||`，但 Current 的 after-atom completion set 不会提供它们。集成方不能
根据 grammar 推断 completion catalog。

配置中标为 disabled 的 property 仍会连同 disable reason 一起返回。它没有 primary edit 或目标 cursor，也
不会进入 `preferred_indices`。Enabled candidate 会携带针对当前 insertion 或 replacement range 的 edit。
Function 和 postfix-function edit 插入括号，并把目标 cursor 放进括号内。`not`、`true` 和 `false` 会在末尾
插入一个空格。Property candidate 插入 `prop("Name")`，cursor 位于整个 call 之后。相关行为由
[`test_completion_smoke.rs`](../../ide/src/tests/ide/test_completion_smoke.rs) 覆盖。

Property name 会被原样放进双引号。Current completion service 不会转义配置名称中的双引号、反斜杠或其他
影响源码的字符，因此 candidate 可能插入无效公式。集成方不能把 property completion 当成 escaping guarantee。
Candidate 的构造入口是 [`completion/items.rs`](../../ide/src/completion/items.rs)。

Completion 与 signature help 是两份独立结果。特别是，当 cursor 严格位于 string 内部时，completion 为空，
但 containing known call 仍可提供 signature help。

## 把排序和 preferred_indices 当作确定性的选择提示

当 expression-start cursor 位于已知 call argument 中，而且该 argument 能映射到 concrete expected type 时，
candidate 会先按该 type 排序。没有 projected parameter，或者 expected type 为 `Unknown` 或 generic 时，会跳过
该阶段。Type ranking 让每种 `CompletionKind` 保持在一个连续 bucket 中。Bucket 内先排 enabled candidate，
再排 disabled candidate；同一分区内，type match 更强的 candidate 靠前。各 bucket 按其中最佳 match 排序，
分数相同时使用固定 kind priority。因此，一个 compatible candidate 仍可能排在前一个 bucket 的 incompatible
candidate 之后。该阶段只排序，不移除 candidate；后续 query ranking 还可能改变最终顺序。

Replacement text 经过 normalization 后非空，而且每个字符都是 ASCII letter、digit、下划线或 whitespace 时，
才能形成 query。匹配会忽略 ASCII 大小写和下划线。对 function 和 property label 而言，exact match 先于
containing match，后者又先于保持字符顺序的 fuzzy subsequence match；其余顺序由确定性的紧凑度、candidate
kind 和原始顺序规则决定。普通 expression completion 会把不匹配 candidate 保留在匹配项之后，只有 `.` 后的
member completion 会移除不匹配项。Function label 的 `()` 与 postfix label 开头的 `.` 不参与匹配。

Replacement text 只要包含 non-ASCII 且不是 whitespace 的 character，就不会形成 query。服务会跳过 query
ranking，保留此前阶段产生的顺序，并返回空的 `preferred_indices`。Current 排序与 query 边界位于
[`completion/ranking.rs`](../../ide/src/completion/ranking.rs)，并由
[`test_completion_ranking.rs`](../../ide/src/tests/ide/test_completion_ranking.rs) 覆盖。

`preferred_indices` 是指向最终已排序 item list 的 selection hint。数量不超过配置的 preferred limit，顺序与
最终 list 一致，而且只指向匹配 query 的 enabled function 或 property candidate。没有 query、limit 为 0，
或没有匹配的 enabled candidate 时，结果为空。它不是第二份 candidate list，也不授权 client 重排返回 item。

## 为 active call 提供一个 best-effort signature

Signature help 只检查 cursor 之前最内层尚未匹配的 `(`。紧邻该括号之前的 token 是已知 function name，且
cursor 位于括号之后时，才会返回 signature。最内层括号属于 grouping expression 或 unknown callee 时，服务
不会回退到外层 known call，而是直接不返回 signature。Cursor 仍在括号前或已经离开 call 时同样没有结果。
缺失右括号不会阻止 help，因此 `if(` 这样的 partial call 仍能得到结果。

Current 服务返回一个 structured signature，并将它选为 active signature。显示的 parameter 和 return type
会结合已有 argument 的 best-effort type；未知或未完成的 argument 可以保留 generic 或 `unknown`。Normal
call 不显示 receiver prefix；受支持的 postfix call 会单独显示 receiver，并从可见 parameter index 中排除
它。产生这些 slot 的 declaration 与 call-shape 机制不属于本文。

Active parameter 跟随当前 top-level argument position。Nested call 或 list 内的 comma 不会推进它。Partial
call 中的空 argument 仍会选中正在编辑的 slot。对于 repeated 或其他经过 projection 的 call shape，存在
直接映射时，服务会选中当前 argument 对应的 displayed slot；没有直接映射时，只要至少存在一个 displayed
parameter，就返回最后一项的 index。零参数 signature 没有 displayed parameter，但仍返回 `0`。有直接映射的
case 由 [`test_completion_signature_help.rs`](../../ide/src/tests/ide/test_completion_signature_help.rs) 覆盖；
目前没有专门覆盖 mapping 缺失 fallback 的 regression test。

## 只格式化语法有效的源码

Formatting 是 full-document、all-or-nothing operation。Lexing 或 parsing 只要产生任意 diagnostic，formatting
就失败，不会返回部分格式化的源码。Semantic problem 本身不阻止 formatting，因为该操作不执行 semantic
analysis。

对可接受源码，在 formatter 覆盖的 syntax 范围内，formatting 结果是确定且 idempotent 的，并遵守以下规则：

- 每级缩进使用两个空格；
- binary 和 ternary operator 两侧、comma 之后使用约定的空格；
- 末尾输出一个 newline；
- 按 comment 与 syntax 的附着关系保留 comment；
- 对 compound construct，只有结构本身允许，且缩进加 rendered byte length 不超过 80 bytes 时才选择
  inline layout，否则使用对应的 multiline layout。

Atomic identifier 和 literal 不执行这项宽度检查，因此可能形成更长的一行。除此之外，80-byte threshold 是
Current 固定布局规则，不是可配置的 editor width。Formatter 返回完整的新 document，并按照下文规则根据
这次 replacement 重定位传入的 cursor。因此，严格位于发生变化的 full-document range 内的 cursor 通常移到
开头；原本位于 document end 的 cursor 则在长度调整后仍位于末尾。实现和测试入口包括
[`format.rs`](../../ide/src/format.rs)、[`format` goldens](../../ide/tests/format/) 和
[`test_format_idempotence.rs`](../../ide/src/tests/ide/test_format_idempotence.rs)。

## 所有 edit 都以原文档为坐标基准

一个 edit batch 中的每个 range 都以同一份 original source 为基准。应用前，服务先按 range start、再按 range
end 进行 stable sort；实际修改时则从文档末尾向前执行，避免前面的 edit 改变后续 original position。

非空 range 不能重叠，相邻 range 可以同时存在；同一 position 上可以有多个 zero-width insertion，stable sort
会在结果中保留调用方提供的顺序。Range 非法或发生 overlap 时，整个 batch 都会被拒绝，不会只应用前半部分。
Coordinate unit、character-boundary check 和失败的表达方式属于 WASM API specification。

返回 cursor 的重定位规则是确定的：

- edit 在 cursor 处或之前结束时，cursor 按 inserted length 减 replaced length 的差值移动；
- cursor 严格位于 replaced range 内时，移动到该 range 的 start；
- cursor 正好位于 range start 时，停在 replacement 前；
- edit 严格位于 cursor 之后时，不移动 cursor。

因此，在 cursor 处做 zero-width insertion 会把 cursor 移到新文本之后；从 cursor 开始的 replacement 则让
cursor 留在 replacement start。Formatting 的单个 full-document edit 也使用相同规则。排序、overlap 与
cursor 行为的实现和测试入口是 [`edit.rs`](../../ide/src/edit.rs)、
[`text_edit.rs`](../../ide/src/text_edit.rs) 和
[`test_edit_ops.rs`](../../ide/src/tests/ide/test_edit_ops.rs)。

## Transport 与 presentation 不属于本契约

本文不承诺 Rust `pub` API 稳定性、特定 completion widget、diagnostic 分组策略、signature popover 布局或
自动选择 action，也不规定 serialized enum spelling、optional field 表示、position unit、Unicode scalar
boundary 转换、clamping 或 JavaScript exception message。这些 transport 细节由 WASM API specification
负责；应用呈现由使用 editor service 的 client 负责。

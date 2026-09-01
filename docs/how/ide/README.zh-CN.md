---
doc_id: how.ide
title: "IDE crate 如何生成编辑器帮助并应用编辑"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# IDE crate 如何生成编辑器帮助并应用编辑

[English](README.md)

`ide` crate 接收公式源码、字节 cursor 和 Analyzer 语义上下文，生成补全与签名帮助；它还负责格式化公式，
并在应用文本编辑后重定位 cursor。本文面向需要扩展或调试这些机制的维护者，说明 Current Rust 实现。

这个 crate 负责编辑器流程编排、呈现策略、格式化和字节编辑应用。公式语义、内置函数参数投影、UTF-16
转换和对外 editor-service 契约分别由 `analyzer`、`builtin_fn`、`analyzer_wasm` 和 `docs/specs/`
维护，不属于本文范围。面向 client 的准确结果仍以 editor-services spec 为准；下文说明当前实现通过哪些
pass 生成这些结果，不形成第二份兼容性契约。

## 一次 help session 生成两项相互独立的结果

[`ide::help`](../../../ide/src/lib.rs) 会创建 `HelpSession`，并通过 `analyzer::analyze_syntax` 对源码执行
词法和语法分析。Session 保存返回的 token，随后按固定顺序执行：

```text
source + UTF-8 byte cursor + semantic::Context
                       |
                       v
              detect_cursor_context
                 /             \
                v               v
       signature help        candidates
                |               |
                |          type ranking
                |               |
                |        attach primary edits
                |               |
                |   query ranking + preferred picks
                |               |
                +-------+-------+
                        v
                    HelpResult
```

阅读时从顶部输入向下，直到合并后的 `HelpResult`。两条分支共享同一份 token snapshot 和 call context，
但任意一条都可以没有结果。图中省略了为推断 receiver 和 argument 类型而进行的额外 best-effort 解析。

`HelpResult` 刻意将 `CompletionResult` 与 `Option<SignatureHelp>` 分开。Cursor 位于字符串内部时，补全
可以关闭，而外层调用的签名帮助仍然存在；反过来，未知 callee 会让签名帮助消失，但不会阻止按位置生成
补全。相关编排和独立性由
[`test_completion_position.rs`](../../../ide/src/tests/ide/test_completion_position.rs) 和
[`test_edit_ops.rs`](../../../ide/src/tests/ide/test_edit_ops.rs) 覆盖。

`ide` 内部接收 Analyzer 的 `Span` 和 `TextEdit`，并使用 UTF-8 字节坐标。`help` 没有错误返回值：无法放进
`u32` 的 cursor 会饱和为 `u32::MAX`；需要切片源码的 helper 遇到无效字符边界时会放弃相应推断。下文的
编辑操作则会走更严格的校验路径。编辑器侧 UTF-16 坐标由 `analyzer_wasm` 转换，不由本 crate 处理。

## Cursor context 从不完整语法判断编辑意图

[`detect_cursor_context`](../../../ide/src/context.rs) 根据 cursor 附近的 token 得到四项信息：最内层调用、
粗粒度 `PositionKind`、替换 span，以及可选的规范化 query。

调用检测会维护 cursor 之前尚未闭合的左括号栈。只有左括号前一个非 trivia token 是 identifier 时，最内层
未配对的 `(` 才会成为调用；cursor 位于尚未闭合的分组括号内时，detector 不会回退到更外层调用。选中一个
调用后，仅位于该 argument list 顶层的逗号会增加 `arg_index`；嵌套括号和方括号内的逗号不会增加这个调用
的 index。许多不完整调用仍可通过这条 token 路径处理，不要求 AST 完整。

位置检测会把附近 token 序列归为：

- 表达式起点或正在扩展 identifier 时为 `NeedExpr`；
- identifier、literal 或右括号之后为 `AfterAtom`；
- identifier、literal 或右括号后跟 `.`（可以再带 method 前缀）时为 `AfterDot`；
- 无法识别为受支持的补全位置时为 `None`。

Cursor 严格位于字符串 literal 内部时，位置会强制变成 `None`，但先前检测到的 call context 仍会保留。
对于 `NeedExpr` 和 `AfterDot`，替换 span 会尽量覆盖正在编辑的 identifier；其他位置使用空 span 插入。
只有 span 内容全部由 ASCII 字母或数字、下划线和空白组成时才会生成 query。规范化会去掉下划线和空白，
并折叠 ASCII 大小写。遇到非 ASCII 或标点时，会跳过 query ranking，不会冒险执行无效切片或产生误导性
匹配。

## 补全先生成候选项，再应用编辑器策略

`PositionKind` 决定 [`completion/items.rs`](../../../ide/src/completion/items.rs) 使用哪组候选项：

- `NeedExpr` 提供已配置属性、`not`/`true`/`false`，以及 `semantic::Context` 中的全部函数。属性插入
  `prop("Name")`，函数插入调用括号。
- `AfterAtom` 提供二元运算符和可用于 postfix 调用的函数，并一并插入开头的点号。
- `AfterDot` 只提供可用于 postfix 调用的函数；源码已经包含点号，因此 insert text 不再带点号。当前 token
  gate 不把右方括号识别为 completion receiver，所以 list literal 不会进入这个分支。
- `None` 不生成候选项。

Postfix 补全同时受 Analyzer 共享的 postfix-capable 集合和当前 `Context` 中函数签名约束；`ide` 不维护第二
份 allowlist。点号之后，`HelpSession::infer_postfix_receiver_ty` 会重新解析点号之前的源码前缀，再请求
Analyzer 推断 receiver 类型。类型已知时，首个参数无法接受该类型的函数会被过滤。解析或推断只得到
`Unknown` 时，当前实现会保留完整的 postfix-capable 集合作为 best-effort fallback。

带 `disabled_reason` 的属性仍会展示，方便 UI 解释不可用原因；但
[`attach_primary_edits`](../../../ide/src/completion/ranking.rs) 不会为它们设置 primary edit 或目标 cursor。
可用的普通函数和 postfix 函数会把 cursor 放在新插入的左括号之后；属性则把 cursor 放在整个
`prop(...)` 表达式末尾。

当 `NeedExpr` 同时位于调用参数中时，`expected_call_arg_ty` 会从函数签名中尽力取得声明参数类型。具体
expected type 会按类型兼容性重排补全分类和候选项；位于类型顶层的 `Unknown` 或 `Generic(_)` 会跳过这
一步。不兼容项只会后移，不会被删除，因此公式尚不完整时仍可继续补全。

如果存在 query，[`rank_by_query`](../../../ide/src/completion/ranking.rs) 会按完全匹配、子串匹配和 fuzzy
subsequence 质量排列函数与属性 label。普通表达式补全保留不匹配项；点号后的 member 补全则删除不匹配
method。原始候选位置是最后的 tie-break，因此 `Context` 稳定时输出也稳定。`preferred_indices` 从最终列表
中选择不超过 `CompletionConfig::preferred_limit` 个可用且匹配的函数或属性，不会另建第二份候选清单。

## 签名帮助只负责适配共享的解析后签名

[`compute_signature_help_if_in_call`](../../../ide/src/signature/mod.rs) 只在 call context 存在、cursor 已经过
左括号，并且能在 `semantic::Context` 中找到 callee 时继续；否则返回 `None`。Current 实现最多生成一个
signature candidate，`active_signature` 固定为零。

对于识别出的调用，这个模块执行四项与呈现有关的工作：

1. 检查 token 连续关系是否构成 `receiver.name(...)`，并确认解析到的函数允许 postfix 调用。Method 形式
   会为 receiver 保留第零个语义参数位置。
2. 从左括号后开始切分 argument fragment，忽略嵌套括号和方括号内的逗号。每个非空 fragment 单独解析
   和推断；空 fragment 变成 `ArgumentObservation::Empty`，无法确定的非空 fragment 则可以保留为
   `Unknown` 类型。Observation list 会扩展到 cursor 所在 argument，让空的 active slot 也有对应记录。
   Receiver 类型则从整份源码恢复出的最小匹配 member-call 表达式推断。
3. 把这些 observation 传给 `analyzer::semantic::resolve_call_signature`。参数形状投影、generic binding 和
   return-type refinement 由这个共享 resolver 负责。IDE adapter 只消费它返回的
   `ResolvedFunctionSig`，不会重新实现这些规则。
4. 将解析后的 projection 转成 `DisplaySegment`，并把 cursor 所在 argument 映射到 active rendered
   parameter。

[`signature/render.rs`](../../../ide/src/signature/render.rs) 中的呈现 adapter 遇到声明为 generic 的 slot
时，只要存在 observed type 就会显示它，包括 `Unknown`；没有 observation 时才使用 resolver 给出的
expected type。对于非 generic slot，`Unknown` observation 不会覆盖 expected type，而兼容的 union
observation 可以将其缩窄。Adapter 还会把函数参数显示为其返回类型、在 optional slot 的显示类型后加
`?`、为已投影的 repeat slot 编号，并插入一个省略号。这些都是共享解析之后的呈现选择，不构成另一套
参数模型。

Postfix 调用的第一个 projected slot 会变成 `(condition: boolean).` 这样的 receiver 前缀；对应的
`DisplaySegment::Param` 没有 `param_index`，不能成为 active parameter。其余参数 segment 获得连续的
显示 index。Active-parameter mapping 会找到 `argument_index` 与 cursor 匹配的 projected slot；method
形式再减去 receiver slot。若不完整输入没有直接 mapping，则回退到最后一个 rendered parameter。
Ellipsis segment 永远不计入 active parameter。集中测试
[`test_completion_signature_help.rs`](../../../ide/src/tests/ide/test_completion_signature_help.rs) 覆盖嵌套逗号、
空参数、generic 与 union 显示、重复 slot、postfix receiver 和 fallback mapping。

[`build_signature_segments`](../../../ide/src/display.rs) 将名称、标点、分隔符、参数、省略号、箭头和返回
类型保留为结构化 segment。这个 crate 不把它们压平成一段 UI 字符串，也不决定颜色和排版；最终呈现由
下游 adapter 负责。

## Formatter 把 AST 与原始 trivia 合并

[`ide::format`](../../../ide/src/lib.rs) 会调用 [`edit.rs`](../../../ide/src/edit.rs) 中的 `ide_format`。它先
执行 `analyzer::analyze_syntax`；只要出现 lexer 或 parser diagnostic，就返回
`IdeError::FormatError`，本操作不涉及语义 diagnostic。成功时，formatter 生成一个覆盖完整源码的替换，
再走与调用方 edits 相同的校验和应用路径。

[`Formatter`](../../../ide/src/format.rs) 根据恢复出的 AST 渲染表达式，同时通过 `TokenQuery` 查询原始
token stream。AST 负责运算优先级和表达式嵌套，原始 trivia 则提供行注释和块注释。`used_comments` 防止
同一注释重复附着。尝试 inline layout 时，formatter 会先保存这个集合；如果表达式必须改用 multiline，
就回滚集合，避免一次失败的紧凑布局提前消耗注释。

Formatter 通过 `INDENT` 和 `MAX_WIDTH` 常量集中维护缩进与行宽策略。原本跨行的表达式通常继续走
multiline 路径；带末尾行注释的 binary expression 可以重新尝试 inline layout。单行表达式只有在所有
嵌套部分都能放进宽度时才保持 inline。调用、列表、分组、运算符、三元表达式和 member call 共用这套
递归布局。[`ide/tests/format`](../../../ide/tests/format) 下的 golden snapshot 覆盖注释附着和多行布局；
[`test_format_idempotence.rs`](../../../ide/src/tests/ide/test_format_idempotence.rs) 要求第二次格式化得到完全
相同的文本。

## 编辑应用将校验与修改分开

[`apply_edits`](../../../ide/src/edit.rs) 先按原始源码中的 `(start, end)` 排序调用方提供的 edit，再把完整
vector 交给 `validate_cursor` 和 `validate_sorted_non_overlapping_edits`。这两个 validator 会检查源码范围、
UTF-8 字符边界、range 方向和重叠，并在构造新文本之前把失败映射为 `IdeError` variant。

校验通过后，[`apply_text_edits_bytes_with_cursor`](../../../ide/src/text_edit.rs) 从源码末尾向前遍历 edit，
从而保持原始坐标有效。遍历过程中，位于 edit 之后的 cursor 会按字节长度差移动；位于替换区间内部的
cursor 则锚定到该区间起点。Helper 最终只构造一份 source 和 cursor，不向调用方返回中间结果。

## 测试沿实现边界组织

Crate 内 IDE 测试使用 `$0` 标记和
[`completion_dsl.rs`](../../../ide/src/tests/ide/completion_dsl.rs) 中的 builder，把源码、cursor、语义上下文、
候选应用和预期结果写在同一个 scenario 里。各 suite 对应维护者通常会修改的边界：

- `test_completion_position.rs` 检查调用与位置检测、替换 span、不完整输入和字符串内禁用补全；
- `test_completion_ranking.rs` 检查 query 过滤、fuzzy 与类型排序、receiver 过滤和 preferred index；
- `test_completion_signature_help.rs` 检查 observed argument 到结构化签名和 active parameter 的 adapter；
- `test_edit_ops.rs` 检查重叠拒绝、cursor 在合法 edit 后的移动、格式化时的语法错误拒绝，以及组合 help
  结果；
- `test_format_idempotence.rs` 检查 formatter 稳定性。

当前 focused edit suite 没有直接覆盖所有 `InvalidCursor` 或 `InvalidEditRange` 分支。修改这些 validator
时应该补充边界用例，不能只依赖 overlap test。

Integration test [`format_golden.rs`](../../../ide/tests/format_golden.rs) 按路径排序，将每个 `*.formula`
fixture 与相邻 `*.snap` 结果比较。Formatter 变更只有在逐一检查 source/output pair 后才应更新 snapshot。
运行完整 crate 测试：

```bash
cargo test -p ide
```

---
doc_id: how.analyzer-wasm
title: "analyzer_wasm 如何衔接 JavaScript 与 Rust"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# analyzer_wasm 如何衔接 JavaScript 与 Rust

[English](README.md)

`analyzer_wasm` crate 是 JavaScript 编辑器数据与 Rust `analyzer`、`ide` crate 之间的转换层。它在导出的
`Analyzer` 中保存语义配置，在每次调用边界转换源码坐标，再把内部结果序列化成 `dto::v1` namespace 下的
DTO。本文面向需要修改或调试这层桥接实现的维护者，说明 Current 实现。

这个 crate 不定义公式语义或 IDE 算法，也不负责稳定的外部 WASM 契约；后者由 `docs/specs/` 维护。下文的
方法名和数据流只说明当前实现如何兑现契约，不构成第二份 API specification。

## Facade 只保存配置，不保存文档

[`Analyzer`](../../../analyzer_wasm/src/lib.rs) 是通过 `wasm_bindgen` 导出的类型，内部保存 Analyzer
`Context` 和补全偏好数量。构造时先确认传入的 `JsValue` 是 object，且顶层只包含已识别的 key，再用
`serde_wasm_bindgen` 反序列化为 `dto::v1::AnalyzerConfig`。随后，构造器把配置属性转换成 Analyzer
property，将 `disabled_reason` 设为 `None`，安装 Rust 内置函数注册表，并确定默认偏好数量。

所有导出操作都只接收 `&self`。同一个 instance 会复用不可变的语义配置，但不会保存源码、parse tree、
diagnostic 或 edit history。每次调用 `analyze`、`format`、`apply_edits` 或 `help` 都要传入完整源码，并重新
生成 core result。

```text
JsValue / UTF-16 positions
            |
            v
 wasm_bindgen Analyzer
  | config state |
  +--------------+
            |
     deserialize inputs
     convert coordinates
            |
            v
     analyzer or ide
       byte positions
            |
            v
  stateless Converter
  core types -> dto::v1
            |
            v
 serde_wasm_bindgen::to_value
```

阅读时从顶部的 JavaScript 调用向下，直到序列化结果。Config box 是多次调用之间唯一保留的状态。图中有意
省略由下游 crate 负责的分析、补全、签名、格式化和编辑算法。

## 每个方法组合自己的边界流水线

各 export 共用序列化 helper，但校验顺序不同：

| 方法 | `lib.rs` 中的边界流水线 |
| --- | --- |
| `new` | 校验 object key -> 反序列化 config -> 构造 `Context` |
| `analyze` | 调用 `analyzer::analyze` -> 转换 diagnostic、token 和根类型 -> 序列化 |
| `format` | 校验并转换 cursor -> 调用 `ide::format` -> 按更新后源码转换 cursor -> 序列化 |
| `apply_edits` | 反序列化 edit -> 校验并转换 range -> 校验并转换 cursor -> 调用 `ide::apply_edits` -> 转换更新后的 cursor -> 序列化 |
| `help` | 宽松转换 cursor -> 调用 `ide::help` -> 转换 help DTO，包括编辑后文档中的 cursor -> 序列化 |

调试失败时，顺序很重要。以 `apply_edits` 为例：edit payload 无法反序列化、UTF-16 range 超出输入文档，
或 cursor 越界，都会让调用在进入 `ide` 之前失败。转换成字节坐标之后的排序、重叠校验、edit 应用和
cursor 重定位仍由 IDE 负责。

`analyze` 把 instance 中的 `Context` 直接交给 Analyzer，converter 不会重新解释 diagnostic 或推断类型。
`format` 和 `apply_edits` 也只在坐标转换后委托操作。`help` 会传入保存的偏好数量，但补全和签名机制仍在
`ide` 内部。

## 坐标转换会向 Unicode scalar 起点取整

Rust core span 和 cursor 使用 UTF-8 字节 offset；JavaScript 编辑器位置进入本 crate 时使用 UTF-16 code
unit offset。[`offsets.rs`](../../../analyzer_wasm/src/offsets.rs) 集中实现两个方向：

- `Converter::utf16_to_8_offset` 先把 offset 限制在 UTF-16 文档长度内，再遍历 Unicode scalar；如果位置
  落在某个 scalar 的 UTF-16 编码内部，则向该 scalar 的字节起点取整。
- `Converter::utf8_to_16_offset` 同样会限制 byte position，并在位置落入 scalar 的 UTF-8 编码内部时
  向起点取整，之后才累计 UTF-16 code unit。

以 `"😀a"` 为例，UTF-16 offset `1` 位于 emoji 的 surrogate pair 内部，会转换为 byte `0`；offset `2`
则转换到 emoji 后面的字节位置。Checked cursor 或 edit endpoint 位于 pair 内部时，也使用这条取整规则，
不会一律拒绝 scalar-interior position。

通用 converter 对所有输入都有结果，但导出方法会采用不同 wrapper：

- `help` 直接调用带 clamp 的 converter。超出文档的 offset 会变成文档末尾，scalar-interior offset 会向下
  取整。
- `format` 和 `apply_edits` 通过 `utf16_to_8_cursor` 处理 cursor。它先拒绝超过 UTF-16 长度的位置，再对
  scalar interior 使用相同的取整转换。
- `utf16_to_8_text_edits` 在坐标转换前拒绝反向 range 或 end 超过 UTF-16 长度的 range，再分别转换两个
  endpoint。每个 endpoint 都独立向下取整。转换后的 range 会交给 `ide::apply_edits` 校验字节区间顺序和
  重叠。

因此，位于 surrogate pair 内的 endpoint 可能让 range 收缩为空字节区间，也可能改变多个 range 转换后的
相对关系。Core edit validator 只能看到最终字节 range。`offsets.rs` 旁的单元测试固定了取整和越界路径；
Current WASM integration suite 只在合法 scalar 边界上测试 emoji 转换。如果修改 checked scalar-interior
处理，应补充直接覆盖这种输入的 integration case。

[`byte_span_to_utf16_span`](../../../analyzer_wasm/src/span.rs) 使用反向 converter 分别处理输出 span 的两个
endpoint。Analyzer 和 IDE span 通常已经位于合法字节边界；即使内部传来不合法 endpoint，取整也能让
boundary adapter 保持 panic-free。

转换坐标时使用的 source 必须与坐标所处的 lifecycle 一致：

- diagnostic、token、code action、replace span 和补全 edit range 使用原始 source；
- `format` 和 `apply_edits` 返回的 cursor 使用 IDE 返回的更新后 source；
- completion item 的目标 cursor 属于应用该 item edits 后的假想文档，因此 completion converter 会先构造
  更新后的文本，再转换 cursor。

## Converter 只适配结构，不增加语义

`Converter` 是零大小的 namespace，分别实现在
[`converter/analyze.rs`](../../../analyzer_wasm/src/converter/analyze.rs)、
[`converter/completion.rs`](../../../analyzer_wasm/src/converter/completion.rs) 和
[`converter/shared.rs`](../../../analyzer_wasm/src/converter/shared.rs) 中。每个 adapter 接收 core result，
逐字段构造对应 DTO。

Analyze adapter 创建一份 `SourceMap`，转换每个 diagnostic span 和所附 code-action edit，过滤 trivia token，
再把剩余 token kind 显式映射成 DTO string。Diagnostic 的 `line` 和 `col` 来自原始 byte span 起点处的
`SourceMap::line_col`；它们是 Analyzer 计算的展示坐标，不是另一轮 UTF-16 offset 转换。推断出的根类型
则使用 Rust display implementation 渲染。

Help adapter 保留 IDE item 顺序、preferred index、signature index 和结构化 display segment。Primary 和
additional edit range 都按原始 source 转换。当 item 存在 primary edit 时，`completion_item_view` 会确定
编辑后文档中的 byte cursor，计入 primary edit 之前的合法 additional edit，把排序后的 edit 应用到一份
临时 source，将目标 byte cursor 限制在结果长度内，最后才转换为 UTF-16。它不会重新实现 completion
ranking 或 active-parameter 逻辑。

`token_kind_string`、`completion_kind_view` 和 `display_segment_view` 等 enum converter 使用穷尽的 Rust
match。增加内部 enum variant 时，compiler 会直接暴露尚未处理的转换义务，而不会默默输出 fallback。

## DTO 同时驱动序列化和 TypeScript declaration

[`dto::v1`](../../../analyzer_wasm/src/dto/v1.rs) 将 wire-shaped Rust type 与 Analyzer、IDE 内部结构隔离。
输入类型 derive `Deserialize`，输出类型 derive `Serialize`，需要供 TypeScript 使用的类型还 derive
`ts_rs::TS`。Serde attribute 定义 `Property.type` rename、带 tag 的 `DisplaySegment` 表示，以及 config
未知字段拒绝等 wire 细节。`v1` 只是源码 namespace；本 crate 没有运行时版本协商。

DTO layer 和 `serde_wasm_bindgen` 是两个独立 seam：

- `from_value` 把 constructor input 转为 typed config，并把反序列化失败统一折叠成 constructor 的通用错误；
- `apply_edits` 直接反序列化 edit array，在开始坐标处理之前映射 malformed payload；
- `to_value` 序列化每个返回 DTO，并将 serializer failure 映射为 JavaScript `Error`。

Analyzer diagnostic 仍是 `AnalyzeResult` 内的数据；facade 不会把它转换成 thrown operation error。相比之下，
`format` 和 `apply_edits` 的坐标或 IDE operation failure 会经过 `operation_err`，根据 `IdeError` message 构造
`js_sys::Error`。已经执行到序列化阶段的方法仍可能在那里失败。稳定的外部错误面由 WASM spec 维护；本节
只说明 failure 的来源。

## Type generation 与 WASM packaging 是两条构建边界

Checked-in TypeScript DTO 文件和可执行 WASM package 来自不同工具。

[`export_ts`](../../../analyzer_wasm/src/bin/export_ts.rs) 对一份显式排序的 `dto::v1` 类型列表调用
`TS::decl()`，按需补上 `export`，再覆盖
[`wasm_dto.ts`](../../../examples/vite/src/analyzer/generated/wasm_dto.ts)。`just gen-ts` recipe 会运行这个
binary。`ts-rs` 反映 Rust field type，Serde 则控制 runtime 接受哪些输入；例如，Serde default 不会自动
让生成的 TypeScript property 变成 optional。修改输入 DTO 时，必须同时检查反序列化 attribute 和生成的
declaration。

另一方面，[`analyzer_wasm/Cargo.toml`](../../../analyzer_wasm/Cargo.toml) 将 library 同时构建为 `cdylib`
和 `rlib`。Vite 的 `wasm:build` recipe 调用 `wasm-pack build --target web`，把 JavaScript glue 和 `.wasm`
module 写进 example 中被忽略的 `src/pkg/` 目录。Wasm-bindgen artifact 不会替代 checked-in
`wasm_dto.ts`，导出 DTO declaration 也不会重新构建 WASM module。

目前没有自动 drift test 会重新生成 `wasm_dto.ts` 并与 checked-in 文件比较。DTO 变更应先运行
`just gen-ts`、检查 generated diff；binding surface 变化时，再单独构建 WASM consumer。

## 测试分别覆盖原生转换与 JavaScript 边界

Native test 在不启动 JavaScript runtime 的情况下检查纯 Rust 部分：

```bash
cargo test -p analyzer_wasm
```

`offsets.rs` 旁的单元测试覆盖 scalar-interior floor 和 checked out-of-range conversion；analyze converter test 固定
多行 diagnostic location。[`analyzer_wasm/tests/analyze.rs`](../../../analyzer_wasm/tests/analyze.rs) 中的
`wasm_bindgen_test` case 在 native 编译中不会执行，只通过 WASM test runner 运行：

```bash
wasm-pack test --node analyzer_wasm
```

这些 integration test 穿过真实 `JsValue` 边界，覆盖 config 拒绝、序列化后的 ASCII/中文/emoji span、
diagnostic action、format error、edit conversion、overlap error 和 preference 传递。如果问题只在 Node
出现，先从 `lib.rs` 判断失败阶段，再根据类型检查 `offsets.rs` 或对应 output converter。序列化 shape
变化还应检查 `dto/v1.rs` 和 generated TypeScript diff。

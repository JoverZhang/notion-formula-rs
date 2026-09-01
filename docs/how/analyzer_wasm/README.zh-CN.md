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

## 每个方法组合不同的内部调用路径

各 export 共用序列化 helper，但组合方式不同。下表用于阅读源码，不定义调用方可以传入什么，也不定义多个
失败条件同时出现时的外部结果；这些保证属于 WASM API specification。

| 方法 | `lib.rs` 中的当前调用 |
| --- | --- |
| `new` | `validate_config_keys` -> `from_value::<AnalyzerConfig>` -> 构造 `Context` |
| `analyze` | `analyzer::analyze` -> `Converter::analyze_output` -> `to_value` |
| `format` | `utf16_to_8_cursor` -> `ide::format` -> `utf8_to_16_offset` -> `to_value` |
| `apply_edits` | 反序列化 edit -> `utf16_to_8_text_edits` -> `utf16_to_8_cursor` -> `ide::apply_edits` -> 反向转换 cursor -> `to_value` |
| `help` | `utf16_to_8_offset` -> `ide::help` -> `Converter::help_output_view` -> `to_value` |

调试时，可以沿着对应行找到最先拥有可疑表示的 helper。以 `apply_edits` 为例，它会先准备 DTO edit 和 byte
coordinate，再把操作交给 IDE。转换后的排序、重叠校验、edit 应用和 cursor 重定位仍由 IDE 负责。多个输入
条件同时非法时调用方会看到哪个错误，由 WASM specification 定义，不由这张调用表定义。

`analyze` 把 instance 中的 `Context` 直接交给 Analyzer，converter 不会重新解释 diagnostic 或推断类型。
`format` 和 `apply_edits` 也只在坐标转换后委托操作。`help` 会传入保存的偏好数量，但补全和签名机制仍在
`ide` 内部。

## 坐标转换让 Rust 始终使用 Unicode scalar 边界

Rust core span 和 cursor 使用 UTF-8 字节 offset；JavaScript 编辑器位置进入本 crate 时使用 UTF-16 code
unit offset。[`offsets.rs`](../../../analyzer_wasm/src/offsets.rs) 集中实现两个方向。输入是否有效以及调用方
最终能观察到什么坐标行为，由 WASM specification 定义；本节只解释实现这些规则的循环：

- `Converter::utf16_to_8_offset` 先把 offset 限制在 UTF-16 文档长度内，再遍历 Unicode scalar；如果位置
  落在某个 scalar 的 UTF-16 编码内部，则向该 scalar 的字节起点取整。
- `Converter::utf8_to_16_offset` 同样会限制 byte position，并在位置落入 scalar 的 UTF-8 编码内部时
  向起点取整，之后才累计 UTF-16 code unit。

两个循环都逐个 Unicode scalar 累计位置。如果目标位置落在当前 scalar 的编码内部，循环会返回进入该
scalar 前已经累计的位置，避免从 scalar 中间切开。通用 converter 对所有输入都有结果，export method 则
通过不同 helper 组合这些转换：

- `help` 直接调用 `Converter::utf16_to_8_offset`；
- `format` 和 `apply_edits` 在进入 IDE 前通过 `utf16_to_8_cursor` 处理 cursor；
- `apply_edits` 通过 `utf16_to_8_text_edits` 处理 DTO range。这个 helper 分别转换 endpoint，生成 Analyzer
  byte edit，再交给 IDE 校验整个 batch。

超过文档末尾、range 方向反转，以及落在 Unicode scalar 编码内部的位置，对调用方呈现的具体行为以 WASM
specification 为准。

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

- `from_value` 是 constructor 把输入反序列化为 typed config 的入口；
- `apply_edits` 在坐标转换前反序列化 edit array；
- `to_value` 是各方法共用的 output serialization 入口；
- 调用 `operation_err` 的方法通过它把 `IdeError` 适配为 `js_sys::Error`。

这些 helper 的位置用于定位转换失败。哪些失败作为数据返回、哪些会抛出错误，以及 message 和可观察顺序，
都由 WASM specification 维护。

## Type generation 与 WASM packaging 是两条构建边界

签入仓库的 TypeScript DTO 文件和可执行 WASM package 来自不同工具。

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

`offsets.rs` 旁的单元测试覆盖 Unicode scalar 内部位置的向下取整，以及带校验的越界转换；analyze
converter 的测试固定多行 diagnostic 的位置。[`analyzer_wasm/tests/analyze.rs`](../../../analyzer_wasm/tests/analyze.rs) 中的
`wasm_bindgen_test` case 在 native 编译中不会执行，只通过 WASM test runner 运行：

```bash
wasm-pack test --node analyzer_wasm
```

这些 integration test 穿过真实 `JsValue` 边界，覆盖 config 拒绝、序列化后的 ASCII/中文/emoji span、
diagnostic action、format error、edit conversion、overlap error 和 preference 传递。如果问题只在 Node
出现，先从 `lib.rs` 判断失败阶段，再根据类型检查 `offsets.rs` 或对应 output converter。序列化 shape
变化还应检查 `dto/v1.rs` 和 generated TypeScript diff。

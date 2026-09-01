---
doc_id: specs.wasm-api
title: "WASM 与 JavaScript 边界保证什么？"
language: zh-CN
source_language: en
counterpart: ./wasm-api.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# WASM API

[English](wasm-api.md)

这份 Current 规格定义了 JavaScript 集成所面对的同步 WASM 边界：如何配置
`Analyzer`、它提供哪些方法、请求和结果经过序列化后的确切结构、位置如何跨越边界，
以及调用方可以区分哪些受控错误。

公式含义由公式语言、公式引用和内置函数规格定义；补全、签名帮助、格式化与编辑行为由
编辑器服务规格定义。本文只说明这些结果如何通过 WASM 边界，不重复相关算法。它不暴露
Rust evaluator API，也不规定演示应用的 UI 策略。

## 复用一个已配置的 Analyzer

模块导出一个有状态的 `Analyzer` 类，提供以下同步接口：

```ts
new Analyzer(config: AnalyzerConfig)

analyzer.analyze(source: string): AnalyzeResult
analyzer.format(source: string, cursor_utf16: number): ApplyResult
analyzer.apply_edits(
  source: string,
  edits: Array<TextEdit>,
  cursor_utf16: number,
): ApplyResult
analyzer.help(source: string, cursor_utf16: number): HelpResult
```

实例会保留属性 schema 和补全优选项数量上限，且没有方法可在创建后修改这些配置。它不会
保留源码、分析结果、编辑历史或光标位置；每次操作都接收完整源码。因此，多个文档如果共享
同一份配置，应用可以复用同一个实例。

这个边界不允许配置函数。每个实例都从 Rust 声明取得项目支持的规范内置函数集合。顶层的
`functions` 字段会被拒绝，不会扩充或替换该集合。导出接口的实现入口是
[`analyzer_wasm/src/lib.rs`](../../analyzer_wasm/src/lib.rs)。

## 区分运行时配置与生成的 TypeScript 类型

生成的类型声明如下：

```ts
type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };

type Property = {
  name: string;
  type: Ty;
};

type AnalyzerConfig = {
  properties: Array<Property>;
  preferred_limit: number | null;
};
```

生成的 TypeScript 类型要求 `AnalyzerConfig` 的两个字段都存在。运行时反序列化有意接受
更多写法：

| 字段 | 运行时接受方式 |
| --- | --- |
| `properties` | 可以省略，等价于 `[]` |
| `preferred_limit` | 可以省略，也可以传 `undefined` 或 `null`，此时采用默认值 `5` |
| `preferred_limit: 0` | 可以传入，并会禁用 `preferred_indices` |

如果提供 `properties`，它必须是数组；每个元素必须包含字符串 `name` 和有效的 `type`。
因此，显式传入 `properties: undefined` 是无效输入，并不等同于省略该字段。如果提供优选项
上限，它必须能反序列化成 WASM `usize` 范围内的非负整数。属性缺少必需字段、类型变体无效
或值的结构不正确，都会使整份配置被拒绝。

`AnalyzerConfig` 顶层的未知字段会被拒绝；单个 `Property` 对象中的额外未知字段目前会被
忽略。通过 schema 校验不代表重复属性名是受支持的输入。公式引用规格要求属性名唯一，
只有这样才能得到规格所定义的查找行为。

运行时 schema 定义在 [`dto/v1.rs`](../../analyzer_wasm/src/dto/v1.rs)。更严格的生成类型
提交在 [`wasm_dto.ts`](../../examples/vite/src/analyzer/generated/wasm_dto.ts)。TypeScript
集成应满足生成的类型；JavaScript 集成可以依赖上面列出的运行时省略与 `null` 行为。

## 所有 JavaScript 接口都使用 UTF-16 位置

共用的传输类型如下：

```ts
type Span = {
  start: number;
  end: number;
};

type TextEdit = {
  range: Span;
  new_text: string;
};

type ApplyResult = {
  source: string;
  cursor: number;
};
```

所有 span、编辑端点、输入光标和返回光标都以 UTF-16 code unit 计数。span 和编辑范围采用
左闭右开区间：包含 `start`，不包含 `end`。`ApplyResult.cursor` 指向它一同返回的更新后
`source`；输入编辑范围则指向调用时提供的原始源码。

如果一个位置落在占两个 UTF-16 code unit 的 Unicode scalar 内部，它会向下取整到该
scalar 的起点。这个规则适用于 `help`、`format`、`apply_edits` 的光标，也适用于编辑范围的
**两个**端点。因此，落在 scalar 内部的端点不会一概被拒绝；取整还可能把一个非空 UTF-16
范围折叠为内部的空范围。转换契约的实现入口是
[`offsets.rs`](../../analyzer_wasm/src/offsets.rs)。

超过源码末尾的位置按方法分别处理：

| 输入 | 超过末尾时的行为 |
| --- | --- |
| `help` 光标 | 截到源码末尾 |
| `format` 光标 | 抛出 `Invalid cursor` |
| `apply_edits` 光标 | 抛出 `Invalid cursor` |
| `apply_edits` 范围端点 | 抛出 `Invalid edit range` |

诊断的 `line` 和 `col` 是另一套面向人的坐标。两者都从 1 开始，且 `col` 按 Unicode
scalar value 而不是 UTF-16 code unit 计数。因此，一个 emoji 会让诊断列号增加 1，
但会在 span 中占 2 个单位。

## Analyze 把公式问题作为数据返回

`analyze` 返回以下确切字段：

```ts
type DiagnosticKind = "error";

type CodeAction = {
  title: string;
  edits: Array<TextEdit>;
};

type Diagnostic = {
  kind: DiagnosticKind;
  message: string;
  span: Span;
  line: number;
  col: number;
  actions: Array<CodeAction>;
};

type Token = {
  kind: string;
  text: string;
  span: Span;
};

type AnalyzeResult = {
  diagnostics: Array<Diagnostic>;
  tokens: Array<Token>;
  output_type: string;
};
```

词法、语法和语义问题都通过 `diagnostics` 返回，不会让 `analyze` 抛出错误。诊断顺序和
可选 action 的语义由编辑器服务规格定义。传输对象只包含 `kind`、`message`、`span`、
`line`、`col` 和 `actions`；内部诊断代码、label 和 note 不会跨越边界。

`tokens` 会排除注释与换行 trivia，但包含 `Eof` token。生成的 API 将 `Token.kind`
定义为开放的字符串，而不是封闭的 TypeScript union。`output_type` 始终是非空字符串；
推断失败或无法确定时用 `"unknown"` 表示。

对于受支持的字符串输入，`analyze` 唯一的受控失败是结果序列化失败，错误消息为
`Serialize error`。公式诊断的确切文案不是兼容性标识；下文列出的受控边界错误消息则是。
边界转换的实现入口是
[`converter/analyze.rs`](../../analyzer_wasm/src/converter/analyze.rs)。

## Help 返回补全和可选的签名数据

`help` 返回以下确切序列化结构：

```ts
type CompletionItemKind =
  | "FunctionGeneral"
  | "FunctionText"
  | "FunctionNumber"
  | "FunctionDate"
  | "FunctionPeople"
  | "FunctionList"
  | "FunctionSpecial"
  | "Builtin"
  | "Property"
  | "Operator";

type CompletionItem = {
  label: string;
  kind: CompletionItemKind;
  insert_text: string;
  primary_edit: TextEdit | null;
  cursor: number | null;
  additional_edits: Array<TextEdit>;
  detail: string | null;
  is_disabled: boolean;
  disabled_reason: string | null;
};

type CompletionResult = {
  items: Array<CompletionItem>;
  replace: Span;
  preferred_indices: Array<number>;
};

type DisplaySegment =
  | { kind: "Name"; text: string }
  | { kind: "Punct"; text: string }
  | { kind: "Separator"; text: string }
  | { kind: "Ellipsis" }
  | { kind: "Arrow"; text: string }
  | { kind: "Param"; name: string; ty: string; param_index: number | null }
  | { kind: "ReturnType"; text: string };

type SignatureItem = {
  segments: Array<DisplaySegment>;
};

type SignatureHelp = {
  signatures: Array<SignatureItem>;
  active_signature: number;
  active_parameter: number;
};

type HelpResult = {
  completion: CompletionResult;
  signature_help: SignatureHelp | null;
};
```

`completion` 始终存在，即使 `items` 为空；各数组字段也始终存在。没有可用的签名帮助时，
`signature_help` 没有值。生成的类型用 `null` 表示缺失的可选值，但当前运行时 serializer
仍会保留相应对象属性，只是把属性值写成 `undefined`。受影响的字段包括 `signature_help`、
`CompletionItem` 中所有可空字段以及 `Param.param_index`。JavaScript 和 TypeScript 调用方
都必须处理这项生成类型与运行时之间的差异。补全候选项、排序、编辑语义、签名帮助的可用
条件和 active parameter 行为均由编辑器服务规格定义。结果转换的实现入口是
[`converter/completion.rs`](../../analyzer_wasm/src/converter/completion.rs)。

`help` 接受不完整的公式源码，并采用上文所述的宽容光标处理方式。它唯一的受控失败是
`Serialize error`。

## Format 和 Apply edits 返回一份更新后的完整文档

两个修改源码的操作都返回 `ApplyResult`，不会修改 Analyzer 内部保留的文档状态。

`format(source, cursor_utf16)` 会先检查光标，再检查源码语法。因此，光标超过末尾时会返回
`Invalid cursor`，即使公式同时也无法格式化。成功时，它返回完整的格式化源码和该新源码
中的光标位置。存在词法或语法问题的源码会产生 `Format error`；格式布局和光标重定位由
编辑器服务规格定义。

`apply_edits(source, edits, cursor_utf16)` 要求 `edits` 能反序列化为 `Array<TextEdit>`。
它先按原始源码转换每个范围，再检查光标，最后校验并应用整批编辑。成功时，它返回完整的
更新后源码和重定位后的光标。原始坐标排序、重叠行为和光标重定位由编辑器服务规格定义。

## 依赖受控错误消息与校验顺序

构造函数以原始字符串 `Invalid analyzer config` 拒绝无效配置。各操作失败时会返回
JavaScript `Error` 对象，其 `message` 为以下值之一：

| 消息 | 含义 |
| --- | --- |
| `Invalid edits` | `edits` 无法反序列化为 `Array<TextEdit>` |
| `Invalid cursor` | 需要严格校验的光标超过原始源码末尾 |
| `Invalid edit range` | 编辑范围反向，或其结束位置超过源码末尾 |
| `Overlapping edits` | 转换后的原始文档范围互相重叠 |
| `Format error` | 源码存在词法或语法问题 |
| `Serialize error` | 结果无法序列化为 JavaScript 值 |

此表中的大小写和文字是 Current 边界契约的一部分。

校验优先级同样可观察：

| 操作 | 校验与执行顺序 |
| --- | --- |
| 构造函数 | 要求对象并拒绝顶层未知键 -> 反序列化配置 -> 构造 Analyzer |
| `analyze` | 分析并转换结果 -> 序列化 |
| `format` | 校验并转换光标 -> 格式化 -> 序列化 |
| `apply_edits` | 反序列化 edits -> 按输入顺序校验并转换编辑范围 -> 校验并转换光标 -> 排序、检查重叠并应用 -> 序列化 |
| `help` | 截断/取整光标 -> 计算 help -> 序列化 |

例如，格式错误的编辑 payload 会优先于 `apply_edits` 后续所有失败；无效范围优先于无效
光标；但在范围转换成功后，无效光标又会先于重叠检查报告。scalar 内部的取整发生在转换
阶段，早于重叠检查，因此用于检查重叠的范围可能和原始 UTF-16 端点不同。

确切错误消息和方法顺序定义在
[`analyzer_wasm/src/lib.rs`](../../analyzer_wasm/src/lib.rs)，导出边界行为由
[`analyzer_wasm/tests/analyze.rs`](../../analyzer_wasm/tests/analyze.rs) 覆盖。

## Evaluator 与 UI 行为不属于这个 API

WASM 模块只导出分析与编辑器服务。它不导出公式求值、prepared plan、row input、evaluator
结果或 Rust crate API，也不承诺补全组件、quick fix 选择、popover 布局、focus 或公式面板
identity 的任何行为。这些属于应用策略，不是这个边界的字段或行为。

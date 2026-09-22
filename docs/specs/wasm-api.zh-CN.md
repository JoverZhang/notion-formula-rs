---
doc_id: specs.wasm-api
title: "WASM API 与 Worker"
language: zh-CN
source_language: zh-CN
counterpart: ./wasm-api.md
implementation_status: planned
document_status: draft
translation_status: needs-update
last_verified: 2026-09-19
---

# WASM API 与 Worker

[English](wasm-api.md) · [Specification index](README.zh-CN.md)

> Planned Worker client 与 Current Analyzer 分节定义；当前 WASM 尚无 Engine 求值入口。

## Planned：薄客户端

```text
主线程 → FormulaEngineClient / FormulaDraftClient → Worker RPC → WASM → Rust
wrapper 只负责 RPC/session 路由、无损 DTO 转换、UTF-8 ↔ UTF-16、Result ↔ Promise、生命周期。
依赖分析、环检测、编译、求值、文本编辑留在 Rust；Worker/线程池数量不属于接口。
```

```ts
interface FormulaEngineClient {
  getProperty(id: PropertyId): Promise<PropertyState | null>;
  getProperties(): Promise<PropertyState[]>;
  getState(): Promise<FormulaEngineState>;
  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult>;
  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null>;
  // EvaluateInputError 拒绝 Promise；公式与行错误随 EvaluateResult 返回。
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  close(): Promise<void>;
}
interface FormulaDraftClient {
  getState(): Promise<FormulaDraftState>;
  help(cursor: number): Promise<CursorHelp>;
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult>;
  intoDefinition(): Promise<FormulaDefinition>;
  close(): Promise<void>;
}
```

```text
queue
  一个 Engine 及其全部 Draft client 共用一条 FIFO 队列，所有调用按入队顺序串行执行。
engine.close()
  幂等；拒绝新调用 → 等待已入队调用完成 → 释放 Engine 和关联 Draft → 终止 Worker。
draft.close()
  释放该草稿，即 discard。
draft.intoDefinition()
  消耗该草稿；仍需包装为 PropertyDefinition 的 Formula 分支并显式 upsert 才修改 Engine。
coordinates
  JS cursor/span 使用 UTF-16 code units；Rust 使用 UTF-8 bytes。
DTO
  上述名称对应 Rust 契约，不是在声明 Rust 内存布局可直接传输。
  新接口的具体 JS 编码、初始化入口和错误载荷待随实现细化；不能套用下节 Current DTO。
```

业务语义仅由 [Engine](formula-engine.zh-CN.md) 与 [Draft](ide.zh-CN.md) 定义。

## Current：同步 Analyzer

```ts
// 已初始化的生成包导出 Analyzer；default async initializer / initSync 的具体签名不作为稳定契约。
declare class Analyzer {
  constructor(config: AnalyzerConfig);
  analyze(source: string): AnalyzeResult;
  format(source: string, cursor_utf16: number): ApplyResult;
  apply_edits(source: string, edits: TextEdit[], cursor_utf16: number): ApplyResult;
  help(source: string, cursor_utf16: number): HelpResult;
  free(): void;
}
// 只保留固定 properties/preferred_limit，不保存 source、结果、编辑历史或 cursor。
// 同配置可复用到多份文档；无配置更新方法。
// host 存在 Symbol.dispose 时生成 glue 也提供对应析构方法。
// free/dispose 后不得再调用；产生何种失败不在受控契约内。

type Ty = "Number" | "String" | "Boolean" | "Date" | { List: Ty };
type Property = { name: string; type: Ty };
type AnalyzerConfig = { properties: Property[]; preferred_limit: number | null };
// 以上为生成的 TS 声明，两个字段均必填；JS 运行时接受范围见下。
```

```text
config
  properties 省略 → []；显式 undefined → 无效。
  支持 Array<Property>；serde 偶然接受其他 iterable 不构成稳定保证。
  preferred_limit 省略/undefined/null → 5；0 → 不产生 preferred_indices。
  指定值须为 WASM usize 范围内非负整数；property 缺字段、类型无效、形状错误 → 整份 config 失败。
  顶层未知字段拒绝（包括 functions）；Property 内额外字段忽略；property name 必须唯一。
  builtin 集合固定，不允许通过配置扩展或替换。TS 调用方应满足生成类型。
```

### Current DTO

```ts
type Span = { start: number; end: number };
type TextEdit = { range: Span; new_text: string };
type ApplyResult = { source: string; cursor: number };
type DiagnosticKind = "error";
type CodeAction = { title: string; edits: TextEdit[] };
type Diagnostic = {
  kind: DiagnosticKind; message: string; span: Span;
  line: number; col: number; actions: CodeAction[];
};
type Token = { kind: string; text: string; span: Span };
type AnalyzeResult = {
  diagnostics: Diagnostic[]; tokens: Token[]; output_type: string;
};
type CompletionItemKind =
  | "FunctionGeneral" | "FunctionText" | "FunctionNumber" | "FunctionDate"
  | "FunctionPeople" | "FunctionList" | "FunctionSpecial" | "Builtin" | "Property" | "Operator";
type CompletionItem = {
  label: string; kind: CompletionItemKind; insert_text: string;
  primary_edit: TextEdit | null; cursor: number | null; additional_edits: TextEdit[];
  detail: string | null; is_disabled: boolean; disabled_reason: string | null;
};
type CompletionResult = {
  items: CompletionItem[]; replace: Span; preferred_indices: number[];
};
type DisplaySegment =
  | { kind: "Name"; text: string } | { kind: "Punct"; text: string }
  | { kind: "Separator"; text: string } | { kind: "Ellipsis" }
  | { kind: "Arrow"; text: string }
  | { kind: "Param"; name: string; ty: string; param_index: number | null }
  | { kind: "ReturnType"; text: string };
type SignatureItem = { segments: DisplaySegment[] };
type SignatureHelp = {
  signatures: SignatureItem[]; active_signature: number; active_parameter: number;
};
type HelpResult = { completion: CompletionResult; signature_help: SignatureHelp | null };
```

```text
analyze
  公式问题作为 diagnostics 返回，不因公式无效而抛异常。
  不暴露内部 diagnostic code/labels/notes；kind 目前只有 "error"。
  tokens 去掉注释/换行，但保留 Eof；Token.kind 是开放字符串。
  output_type 始终为字符串；推断失败/未知为 "unknown"，不是 null。
help
  容忍残缺 source；items/additional_edits/preferred_indices 等数组始终存在。
  注意当前偏差：生成 TS 将 Option 写成 null，实际 serializer 保留字段但输出 undefined。
  涉及 signature_help、primary_edit、cursor、detail、disabled_reason、param_index。
format / apply_edits
  返回更新后的完整 source 与 cursor；不保存 source，也不修改 Analyzer 配置。
```

候选、排序、诊断顺序、格式和编辑语义归 [IDE 的 Current 章节](ide.zh-CN.md) 所有。

### Current 坐标与异常

```text
strings
  source/property name/new_text 等须为合法 Unicode，无孤立 surrogate。
  生成边界会将孤立 surrogate 替为 U+FFFD；依赖这种归一化的输入不受支持。
positions
  所有 JS cursor/span/edit endpoint 都是 UTF-16 code units，半开区间 [start,end)。
  输入 range 基于原 source；返回 cursor 基于返回的新 source。
  支持的数值为有限整数 0..=4_294_967_295。
  直接 cursor 参数的非法数值可能先被 ABI 强制转换；这种输入不受支持。
  edit DTO 内非法数值 → Invalid edits。
  落在 surrogate pair 内部 → 向下取到该 scalar 起点，cursor 和两个 endpoint 都如此。
  因而非空 UTF-16 range 可能折叠为空；不是一律拒绝。
past end
  help cursor → clamp 到文末
  format/apply_edits cursor → Invalid cursor
  apply_edits endpoint → Invalid edit range
diagnostic location
  line/col 从 1 开始；col 数 Unicode scalar，不是 UTF-16；emoji 在 col 中占 1，在 span 中占 2。

controlled failures
  constructor → 抛出 primitive string "Invalid analyzer config"
  方法 → Error("Invalid edits" | "Invalid cursor" | "Invalid edit range" |
               "Overlapping edits" | "Format error" | "Serialize error")
  range 反向/越界 → Invalid edit range；lexer/parser diagnostic → Format error。
  analyze/help 唯一受控异常是 Serialize error；公式/残缺源码问题作为数据返回。

validation order
  constructor → object/顶层字段校验 → deserialize → construct
  analyze     → analyze/转换 → serialize
  format      → cursor 校验/转换 → format → serialize
  apply_edits → edits deserialize → 按输入顺序校验/转换 ranges → cursor 校验/转换
              → 排序/重叠检查/应用 → serialize
  help        → cursor clamp/floor → help → serialize
  endpoint flooring 先于重叠检查；多个错误并存时由上述顺序决定先报告哪一个。
```

当前边界不暴露 evaluator、plan 或业务行，也不定义 demo UI 策略。
实现锚点：[导出](../../analyzer_wasm/src/lib.rs)、[DTO](../../analyzer_wasm/src/dto/v1.rs)、
[坐标](../../analyzer_wasm/src/offsets.rs)、
[生成的 TS](../../examples/vite/src/analyzer/generated/wasm_dto.ts)。

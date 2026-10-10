---
doc_id: specs.wasm-api
title: "WASM API 与 Worker"
language: zh-CN
source_language: en
counterpart: ./wasm-api.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-09
---

# WASM API 与 Worker

[English](wasm-api.md) · [Specification index](README.zh-CN.md)

Worker 客户端以无损 DTO 使用 [Engine](formula-engine.zh-CN.md) 和 [Draft](ide.zh-CN.md) 契约。分析、求值和编辑逻辑留在 Rust。

## Engine 与 Draft 客户端

```ts spec-file=packages/notion-formula/src/client.h.ts
import type {
  CompletionConfig,
  CursorHelp,
  DiagnosticId,
  EvaluateInput,
  EvaluateResult,
  ExpressionUpdate,
  FormulaDefinition,
  FormulaDraftState,
  FormulaEdit,
  FormulaEngineChangeResult,
  FormulaEngineState,
  FormulaSchema,
  PropertyDefinition,
  PropertyId,
  PropertyState,
  QuickFix,
  UpdateExpressionResult,
} from "./generated/wasm_dto.js";
import type { FormulaWorker } from "./rpc.js";

// 纯同步函数，无需初始化 Worker/WASM。为有效 Unicode 值加上双引号，
// 转义反斜杠、双引号、换行和 tab；保留其他字符。
// a"b -> "a\"b"；空字符串 -> ""。
export declare function encodeFormulaString(value: string): string;

// 纯同步函数；输入为词法分析器产出的一个完整、有效的 String token.text。
// 只解码一次：\n -> 换行，\t -> tab，\" -> 双引号，\\ -> 反斜杠。
// "a\"b" -> a"b；"" -> 空字符串。保留原始 Unicode 和控制字符。
export declare function decodeFormulaString(literal: string): string;

// Engine 及其全部 Draft 共用一条 FIFO 队列；调用失败不阻断后续调用。
// 每个请求入队时保存参数快照；后续突变不改变已入队的请求。
// 不可克隆的参数在对应 FIFO 位置以 INVALID_REQUEST 拒绝。
export interface FormulaEngineClient {
  getProperty(id: PropertyId): Promise<PropertyState | null>;
  getProperties(): Promise<PropertyState[]>;
  getState(): Promise<FormulaEngineState>;
  // 返回排序、去重后的传递 Input ID。Ready 目标的依赖集合完整；
  // NotReady 目标只返回已知引用。无效目标列表以 EVALUATE_INPUT 拒绝。
  requiredInputs(formulaIds: PropertyId[]): Promise<PropertyId[]>;
  // 所有 Draft 消耗或关闭之前，拒绝并返回 ACTIVE_DRAFTS。
  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult>;
  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null>;
  // EVALUATE_INPUT 拒绝 Promise；公式与行错误保留在 EvaluateResult 中。
  // 所有行使用调用方提供的 RuntimeContext；Engine 不读取系统时钟。
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  // 立即拒绝新调用，等待已入队调用完成，先释放 Draft 再释放 Engine，
  // 最后终止 Worker。重复 close 返回同一 Promise。
  close(): Promise<void>;
}

export interface FormulaDraftClient {
  // Diagnostic ID 不透明，在当前版本中稳定，并限定于此客户端。
  getState(): Promise<FormulaDraftState>;
  // UTF-16 cursor：surrogate pair 内向下取整，超出文末则截到文末。
  help(cursor: number, config: CompletionConfig): Promise<CursorHelp>;
  // 其他客户端或过期的 Diagnostic ID 返回 []；查询仍进入同一 FIFO。
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  // edits 和 cursor 基于原 source；返回的 cursor 基于新 source。
  // 保留 bigint base_version；过期编辑拒绝并返回 UPDATE_EXPRESSION。
  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult>;
  // 消耗 Draft 而不保存；通过 Engine.upsert({ Formula: ... }) 显式保存。
  intoDefinition(): Promise<FormulaDefinition>;
  // 丢弃 Draft；幂等，包括 Engine.close() 之后。
  close(): Promise<void>;
}

export interface FormulaClientOptions {
  // 可选的 Worker 注入；客户端拥有并负责终止所返回的 Worker。
  workerFactory?: () => FormulaWorker;
}

// 由 client.ts 的 createFormulaEngineClient 实现，WASM 与 Engine 初始化完成后
// resolve；初始化失败会释放 Worker。受控拒绝为 FormulaClientError，
// error.data.code 区分对应的类型化 payload；Worker 故障会结束所有 pending 调用。
export type CreateFormulaEngineClient = (
  schema: FormulaSchema,
  options?: FormulaClientOptions,
) => Promise<FormulaEngineClient>;
```

### 无损 DTO

Rust 声明生成 JavaScript 类型。DTO 使用 structured clone 而非 JSON；enum 采用 serde 输出的编码。可选结果与 unit payload 显式为 `null`；索引和长度保持普通 JavaScript `number`。字符串须为合法 Unicode，不含孤立 surrogate。

```rust out=analyzer_wasm/src/dto/engine.h.rs
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
pub use super::v1::{CompletionItem, SignatureItem, Span, TextEdit, Token};

pub type PropertyId = String;
pub type RowId = String;
/// JavaScript bigint；输入 number 会被拒绝。
pub type DraftVersion = u64;
/// 不透明的原生 ID；Worker 客户端为其添加自己的会话范围。
pub type DiagnosticId = String;
/// 有限整数 UTF-16 code units，范围 0..=4_294_967_295；range 为半开区间。
/// surrogate pair 内的位置在重叠检查前向下取到 scalar 起点。
pub type TextOffset = u32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ValueType { Number, String, Boolean, Date, Unknown, List(Box<ValueType>), Union(Vec<ValueType>) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FormulaSchema { pub properties: Vec<PropertyDefinition> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum PropertyDefinition { Input { id: PropertyId, ty: ValueType }, Formula(FormulaDefinition) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FormulaDefinition { pub id: PropertyId, pub expression: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum FormulaEngineState { AllReady, NotAllReady { cycle_path: Vec<PropertyId> } }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum PropertyState { Input { id: PropertyId, ty: ValueType }, Formula(FormulaState) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FormulaState { pub definition: FormulaDefinition, pub status: FormulaStatus }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum FormulaStatus { Ready { output_type: ValueType }, NotReady }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct FormulaEngineChangeResult { pub affected_formulas: Vec<PropertyId> }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
/// Number 保留 NaN、Infinity 和 signed zero；Date 使用 bigint 毫秒。
/// 普通嵌套 null 以 None 表示，序列化为 JavaScript null。
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    Date(#[serde(deserialize_with = "deserialize_i64_bigint")] i64),
    List(Vec<Option<Value>>),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// validity[i] 区分普通 null 与 values[i]；行错误单独返回。
pub struct ColumnData<T> { pub values: Vec<T>, pub validity: Vec<bool> }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub enum Column {
    Number(ColumnData<f64>), String(ColumnData<String>), Boolean(ColumnData<bool>),
    Date(#[serde(deserialize_with = "deserialize_date_column")] ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>), Union(ColumnData<Value>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ColumnKind { Number, String, Boolean, Date, List, Union }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// 每个请求使用调用方提供的一份时间与时区快照；now 严格要求 bigint。
pub struct RuntimeContext {
    #[serde(deserialize_with = "deserialize_i64_bigint")]
    pub now: i64,
    pub time_zone: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// columns 必须是 JavaScript Map；record 和其他 iterable 会被拒绝。
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>,
    #[serde(deserialize_with = "deserialize_columns")]
    #[ts(type = "Map<PropertyId, Column>")]
    pub columns: HashMap<PropertyId, Column>,
    pub runtime: RuntimeContext,
    pub formula_ids: Vec<PropertyId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
/// formulas 为 JavaScript Map，完整保留 "__proto__" 等 ID。
pub struct EvaluateResult {
    #[ts(type = "Map<PropertyId, { Ok: FormulaOutput } | { Err: FormulaEvaluationError }>")]
    pub formulas: HashMap<PropertyId, Result<FormulaOutput, FormulaEvaluationError>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct FormulaOutput { pub output_type: ValueType, pub column: Column, pub errors: Vec<RowError> }
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct RowError { pub row_index: u32, pub origin_formula_id: PropertyId, pub error: RuntimeError }
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub enum RuntimeError {
    InvalidValueType { expected: ValueType, actual: ValueType },
    InvalidValue { actual: Value, constraint: String },
    InvalidRegex { pattern: String, detail: String },
    InvalidDateText { text: String }, DateOutOfRange,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum FormulaEvaluationError { NotReady }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum FormulaEngineInitError { EmptyId, DuplicateId(PropertyId) }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum EngineChangeError { EmptyId }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum CreateDraftError { EmptyId }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub enum EvaluateInputError {
    InvalidNow { now: i64 }, InvalidTimeZone { time_zone: String },
    EmptyRowId { row_index: u32 }, DuplicateRowId { id: RowId }, EmptyFormulaIds,
    InvalidFormulaId { id: PropertyId }, DuplicateFormulaId { id: PropertyId },
    MissingInputs { ids: Vec<PropertyId> }, UnexpectedInputs { ids: Vec<PropertyId> },
    InvalidColumnType { id: PropertyId, expected: ColumnKind, actual: ColumnKind },
    InvalidColumnLength { id: PropertyId, expected: u32, values_len: u32, validity_len: u32 },
    InvalidValueType { id: PropertyId, row_index: u32, element_path: Vec<u32>, expected: ValueType, actual: ValueType },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CompletionConfig { pub preferred_limit: u32 }
#[derive(Serialize, TS)]
pub struct FormulaDraftState {
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    pub output_type: ValueType,
    pub diagnostics: Vec<ExpressionDiagnostic>,
    pub tokens: Vec<Token>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ExpressionDiagnostic { pub id: DiagnosticId, pub span: Span, pub message: String }
#[derive(Serialize, TS)]
pub struct CompletionResult { pub items: Vec<CompletionItem>, pub replace: Span, pub preferred_indices: Vec<u32> }
#[derive(Serialize, TS)]
pub struct SignatureHelp { pub signatures: Vec<SignatureItem>, pub active_signature: u32, pub active_parameter: u32 }
#[derive(Serialize, TS)]
pub struct CursorHelp { pub base_version: DraftVersion, pub completion: CompletionResult, pub signature_help: Option<SignatureHelp> }
#[derive(Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
/// 编辑基于原 source；base_version 必须是 bigint。
pub struct FormulaEdit {
    #[serde(deserialize_with = "deserialize_u64_bigint")]
    pub base_version: DraftVersion,
    pub edits: Vec<TextEdit>,
}
#[derive(Serialize, Deserialize, TS)]
pub enum ExpressionUpdate { Replace(String), Edits { edit: FormulaEdit, cursor: TextOffset } }
#[derive(Serialize, TS)]
pub struct QuickFix { pub title: String, pub edit: FormulaEdit }
#[derive(Serialize, TS)]
pub struct UpdateExpressionResult { pub state: FormulaDraftState, pub cursor: TextOffset }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub enum UpdateExpressionError { VersionMismatch, InvalidCursor, InvalidEditRange, OverlappingEdits }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct InvalidDtoPayload { pub operation: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct DraftClosedPayload { pub handle: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ActiveDraftsPayload { pub count: u32 }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EngineInitPayload { pub error: FormulaEngineInitError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EngineChangePayload { pub error: EngineChangeError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct CreateDraftPayload { pub error: CreateDraftError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct EvaluateInputPayload { pub error: EvaluateInputError }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct UpdateExpressionPayload { pub error: UpdateExpressionError }

/// 受控 WASM 错误以普通数据克隆穿过 Worker 边界。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FormulaClientErrorData {
    InvalidDto { message: String, payload: InvalidDtoPayload },
    EngineClosed { message: String, payload: () },
    DraftClosed { message: String, payload: DraftClosedPayload },
    ActiveDrafts { message: String, payload: ActiveDraftsPayload },
    EngineInit { message: String, payload: EngineInitPayload },
    EngineChange { message: String, payload: EngineChangePayload },
    CreateDraft { message: String, payload: CreateDraftPayload },
    EvaluateInput { message: String, payload: EvaluateInputPayload },
    UpdateExpression { message: String, payload: UpdateExpressionPayload },
    FormatError { message: String, payload: () },
    SerializeError { message: String, payload: () },
    /// Worker 崩溃或传输失败时由 JS host 返回。
    WorkerFailure { message: String, payload: () },
    /// Worker 返回不合法响应时由 JS host 返回。
    InvalidResponse { message: String, payload: () },
    /// RPC 请求不合法时由 JS host 返回。
    InvalidRequest { message: String, payload: () },
    /// 创建 Worker 或加载 WASM 失败时由 JS host 返回。
    InitializationError { message: String, payload: () },
}
```

### 同步 WASM 会话

初始化后的 WASM 包导出 `FormulaEngineSession`。Worker 拥有该分配，并将 Engine 与 Draft 调用路由至它。

```rust out=analyzer_wasm/src/engine_session.h.rs
use wasm_bindgen::prelude::*;

/// 拥有一个 Engine 及其 Draft handle；全部业务逻辑在 Rust 中执行。
/// 显式 close 幂等。host 在 close 后须用 free() 释放生成的分配，
/// free() 后不得再使用该分配。
#[wasm_bindgen]
#[spec::private_fields]
pub struct FormulaEngineSession {}

#[wasm_bindgen]
#[spec::header]
impl FormulaEngineSession {
    /// 从无损 FormulaSchema DTO 构建 Engine。
    #[wasm_bindgen(constructor)]
    pub fn new(schema: JsValue) -> Result<FormulaEngineSession, JsValue>;
    pub fn get_property(&self, id: String) -> Result<JsValue, JsValue>;
    pub fn get_properties(&self) -> Result<JsValue, JsValue>;
    pub fn get_state(&self) -> Result<JsValue, JsValue>;
    /// 接收 PropertyId[]，按 FormulaEngine::required_inputs 的语义返回 PropertyId[]。
    /// 数组格式错误返回 INVALID_DTO；公式选择错误返回 EVALUATE_INPUT。
    /// 存在活动 Draft 时仍可调用；Engine 关闭后返回 ENGINE_CLOSED。
    pub fn required_inputs(&self, formula_ids: JsValue) -> Result<JsValue, JsValue>;
    /// 任何 Draft handle 存在时，拒绝并返回 ACTIVE_DRAFTS。
    pub fn upsert(&mut self, property: JsValue) -> Result<JsValue, JsValue>;
    /// 任何 Draft handle 存在时，拒绝并返回 ACTIVE_DRAFTS。
    pub fn remove(&mut self, id: String) -> Result<JsValue, JsValue>;
    pub fn evaluate(&self, input: JsValue) -> Result<JsValue, JsValue>;
    pub fn create_draft(&mut self, formula: JsValue) -> Result<u32, JsValue>;
    pub fn draft_state(&self, handle: u32) -> Result<JsValue, JsValue>;
    /// UTF-16 转换前，检查 cursor 为有限的无符号 32 位整数。
    pub fn draft_help(&self, handle: u32, cursor: JsValue, config: JsValue) -> Result<JsValue, JsValue>;
    pub fn draft_quick_fixes(&self, handle: u32, diagnostic_id: String) -> Result<JsValue, JsValue>;
    pub fn draft_format_edits(&self, handle: u32) -> Result<JsValue, JsValue>;
    pub fn draft_update_expression(&mut self, handle: u32, update: JsValue) -> Result<JsValue, JsValue>;
    /// 消耗 Draft；不修改已保存的定义。
    pub fn draft_into_definition(&mut self, handle: u32) -> Result<JsValue, JsValue>;
    /// 丢弃 Draft；重复 close 及 Engine close 后的 close 均成功。
    pub fn draft_close(&mut self, handle: u32);
    /// 先释放所有 Draft，再释放 Engine；重复 close 成功。
    pub fn close(&mut self);
}
```

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
type Token = {
  kind: string;
  /** 原始源码拼写，包含引号和转义序列。 */
  text: string;
  span: Span;
};
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

同步 Analyzer 边界不暴露 evaluator、plan 或业务行，也不定义 demo UI 策略。
实现锚点：[导出](../../analyzer_wasm/src/lib.rs)、[DTO](../../analyzer_wasm/src/dto/v1.rs)、
[坐标](../../analyzer_wasm/src/offsets.rs)、
[生成的 TS](../../examples/vite/src/analyzer/generated/wasm_dto.ts)。

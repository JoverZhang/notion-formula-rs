---
doc_id: specs.formula-runtime
title: "FormulaEvaluator 与 FormulaAnalyzer 交互接口"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-runtime.md
implementation_status: planned
document_status: draft
translation_status: pending
last_verified: 2026-09-02
---

# FormulaEvaluator 与 FormulaAnalyzer 交互接口

[English](formula-runtime.md)

> 本文描述 Planned 接口。实现和英文版本完成前，它不是当前运行时契约。

本文回答一个问题：宿主如何计算一组公式，同时为单个未提交 expression 提供编辑器能力？Rust 是规范源，TypeScript 只提供 Worker Adapter。

```text
FormulaEvaluator
  无公式状态
  每次接收完整求值快照
  一次计算多个 target

FormulaAnalyzer
  持有一个 expression 的 compiled state
  重复响应 cursor 查询
  不会把草稿提交给 FormulaEvaluator
```

## 共享数据模型

```rust
pub struct PropertyId(String);
pub struct RowId(String);
pub struct TextOffset(usize);
pub struct DraftVersion(u64);
pub struct DiagnosticId(String);
```

### Schema 与类型

```rust
pub struct Schema {
    pub properties: Vec<PropertySchema>,
}

pub struct PropertySchema {
    pub id: PropertyId,
    pub ty: Type,
}

pub enum Type {
    Number,
    String,
    Boolean,
    Date,
    List(Box<Type>),
    Union(Vec<Type>),
}

pub enum InferredType {
    Known(Type),
    NullOnly,
}
```

```text
Schema rules
  - PropertyId 非空、唯一、区分大小写
  - 不执行 Unicode normalization
  - Type 不包含 Unknown 或 Null
  - Union 只包含明确的非 null 类型
  - 所有属性默认 nullable；nullability 不写进 Type
```

类型推断会移除 union 中的 null：

```text
Number | null           => Known(Number)
Number | String | null  => Known(Union([Number, String]))
null                    => NullOnly
```

`NullOnly` 是有效的 expression 结果，但不能写入 Schema。Analyzer Schema 中的公式字段必须已经得到 `Known(Type)`。

Evaluator 的 Schema 描述调用方提供的基础列。Analyzer 的 Schema 描述当前 expression 可以引用的全部字段，包括其他公式已推断出的字段类型。

### 公式和文本位置

```rust
pub struct FormulaDefinition {
    pub id: PropertyId,
    pub source: String,
}

pub struct Span {
    pub start: usize,
    pub end: usize,
}
```

公式没有 display name。`prop("x")` 精确引用 `PropertyId("x")`。Rust span 是 UTF-8 byte 半开区间；TypeScript Adapter 转为 UTF-16 code-unit 半开区间。

## FormulaEvaluator

### 接口

```rust
pub struct FormulaEvaluator { /* private */ }

impl FormulaEvaluator {
    pub fn new() -> Self;
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;
}

pub struct EvaluateInput {
    pub schema: Schema,
    pub formulas: Vec<FormulaDefinition>,
    pub row_ids: Vec<RowId>,
    pub columns: Vec<InputColumn>,
    pub runtime: RuntimeContext,
    pub targets: Vec<PropertyId>,
}
```

一次请求就是完整快照：

```text
schema + formulas + row_ids + columns + runtime + targets
                         ↓
               FormulaEvaluator::evaluate
                         ↓
            formula types + target columns
```

Evaluator 不提供 `upsert`、`remove`、`state`、`revision` 或 `applyChanges`。公式数组顺序不影响依赖分析；每次调用都会根据本次输入建立公式命名空间和依赖图。

### 请求约束

```text
formulas
  - ID 唯一
  - 不得与 Schema property ID 冲突
  - 缺失引用、语法/类型错误和依赖环直接返回 EvaluateError

targets
  - 必填、非空、唯一
  - 每个 ID 都必须属于 formulas
  - 返回顺序与 targets 输入顺序一致

row_ids
  - 非空字符串
  - 同一批次内唯一
  - 零行批次合法

columns
  - 只接受 Schema property
  - ID 唯一，顺序不影响语义
  - target 依赖闭包需要的基础列必须存在
  - Schema 中存在但本次不需要的列允许并忽略
  - 每列长度必须等于 row_ids.len()
```

Schema、公式集合、targets 或输入列违反规格时，Evaluator 在计算行之前直接返回 `Err(EvaluateError)`，不返回部分 target 结果。

### 列输入与运行时

```rust
pub struct InputColumn {
    pub id: PropertyId,
    pub data: Column,
}

pub struct RuntimeContext {
    pub evaluated_at_epoch_ms: i64,
    pub timezone_offset_minutes: i32,
}

pub struct ColumnData<T> {
    pub values: Vec<T>,
    pub validity: Validity,
}

pub enum Validity {
    AllValid,
    AllNull,
    Bitmap(Vec<bool>), // true = value, false = null
}

pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Date(i64),
    List(Vec<Option<Value>>),
}

pub enum Column {
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    Union(ColumnData<Value>),
    Null { len: usize },
}
```

`Bitmap` 长度必须等于 `values.len()`。null 或行错误位置的占位 value 不可读取。`Date` 是 UTC Unix epoch milliseconds。

RuntimeContext 在一次请求的全部行和公式中保持不变。`now()`、`today()` 等函数不得在计算期间读取系统时钟。`row_ids` 为 `id()` 提供逐行身份。

### 结果和错误

```rust
pub struct EvaluateResult {
    pub formula_types: Vec<FormulaType>,
    pub targets: Vec<TargetResult>,
}

pub struct FormulaType {
    pub id: PropertyId,
    pub output_type: InferredType,
}

pub struct TargetResult {
    pub id: PropertyId,
    pub output_type: InferredType,
    pub column: Column,
    pub errors: Vec<RowError>,
}

pub struct RowError {
    pub row_index: usize,
    pub origin_formula_id: PropertyId,
    pub code: RowErrorCode,
    pub message: String,
}
```

`formula_types` 包含全部公式并按 ID 确定性排序，供宿主构建 Analyzer Schema。`targets` 只包含请求的列。

```text
普通 null
  column validity = false
  没有 RowError

行错误
  column validity = false
  包含一个或多个 RowError

规格或公式错误
  整个调用返回 EvaluateError
  没有 EvaluateResult
```

同一 target、同一行可以有多个错误。错误保持确定性的求值遍历顺序，不按行重新排序。短路逻辑和条件表达式不执行未选中的分支。

第一版返回完整列，不提供流、分页、取消或 row provider。

## FormulaAnalyzer

### 生命周期和状态

```rust
pub struct FormulaAnalyzer { /* private compiled state */ }

impl FormulaAnalyzer {
    pub fn new(schema: Schema, expression: String) -> Result<Self, AnalyzerSchemaError>;
    pub fn state(&self) -> &FormulaAnalyzerState;
    pub fn query_cursor_help(&self, cursor: TextOffset) -> CursorHelp;
    pub fn query_quick_fixes(&self, diagnostic_id: DiagnosticId) -> Vec<QuickFix>;
    pub fn query_format_edits(&self) -> Result<FormulaEdit, FormatError>;
    pub fn apply_edits(
        &mut self,
        edit: FormulaEdit,
        cursor: TextOffset,
    ) -> Result<ApplyEditsResult, ApplyEditsError>;
}
```

Schema 在 Analyzer 生命周期内固定。Schema 改变时，宿主用当前 source 创建新的 Analyzer。

```rust
pub struct FormulaAnalyzerState {
    pub version: DraftVersion,
    pub source: String,
    pub output_type: Option<InferredType>,
    pub diagnostics: Vec<FormulaDiagnostic>,
    pub tokens: Vec<Token>,
}

pub struct FormulaDiagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<Span>,
}
```

无效 expression 仍会创建 Analyzer；问题放入 diagnostics，无法推断明确输出时 `output_type = None`。只有 Schema 本身无效才使 `new` 返回错误。

`state` 是公开状态。query 方法可以更新不可观察的私有缓存，但不得改变 `source`、`version` 或公开状态。

### Cursor help

```rust
pub struct CursorHelp {
    pub completions: Vec<Completion>,
    pub signature_help: Option<SignatureHelp>,
}
```

completion、postfix completion 和 signature help 共享一次 cursor 查询。diagnostics 不依赖 cursor，只存在于 `FormulaAnalyzerState`。

### Quick fix、format 和 edits

```rust
pub struct TextEdit {
    pub range: Span,
    pub new_text: String,
}

pub struct FormulaEdit {
    pub base_version: DraftVersion,
    pub edits: Vec<TextEdit>,
}

pub struct QuickFix {
    pub title: String,
    pub edit: FormulaEdit,
}

pub struct ApplyEditsResult {
    pub state: FormulaAnalyzerState,
    pub cursor: TextOffset,
}
```

`query_quick_fixes` 和 `query_format_edits` 只返回候选 edits。调用方可以直接丢弃，只有 `apply_edits` 修改 Analyzer。

```text
apply_edits
  1. base_version 必须等于当前 state.version
  2. 所有 range 和 cursor 必须有效
  3. edits 不得重叠，全部 range 均基于修改前 source
  4. 全部 edits 原子应用并重定位 cursor
  5. 重新编译 expression，推进 version
  6. 返回新 state 和 cursor
```

编辑后的 expression 可以无效；这仍是成功结果，错误出现在新 state 的 diagnostics 中。陈旧版本、非法 range、重叠 edits 或非法 cursor 返回 `ApplyEditsError`，Analyzer 保持不变。

format 只要求语法可格式化，不依赖 Schema 中的语义正确性。Quick fix 必须绑定产生对应 diagnostic 的 version。

## TypeScript Worker Adapter

```ts
interface FormulaEvaluator {
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  close(): Promise<void>;
}

interface FormulaAnalyzer {
  getState(): Promise<FormulaAnalyzerState>;
  queryCursorHelp(cursor: number): Promise<CursorHelp>;
  queryQuickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  queryFormatEdits(): Promise<FormulaEdit>;
  applyEdits(edit: FormulaEdit, cursor: number): Promise<ApplyEditsResult>;
  close(): Promise<void>;
}

declare function createFormulaEvaluator(): Promise<FormulaEvaluator>;

declare function createFormulaAnalyzer(
  schema: Schema,
  expression: string,
): Promise<FormulaAnalyzer>;
```

Evaluator 和 Analyzer 可以运行在不同 Worker 或线程池。具体数量不属于接口：

```text
Evaluator request
  彼此无公式状态依赖，可以并行调度

Analyzer session
  query 可以复用 compiled state
  applyEdits 必须按 session 顺序执行
```

Rust 错误在 TypeScript 中拒绝 Promise；formula diagnostics 和 row errors 属于成功结果。Rust 使用 UTF-8 offset，Adapter 对外使用 UTF-16 offset。

`close()` 幂等。开始关闭后拒绝新调用，等待已入队调用完成，Drop Rust 对象后再终止 Worker。Rust 接口只使用所有权和 `Drop`。

## 第一版不提供

```text
FormulaEvaluator
  - 持久公式集合或 mutation 方法
  - revision、事务或增量 evaluate
  - 流式结果与取消

FormulaAnalyzer
  - FormulaId 或向 Evaluator 提交草稿
  - 可变 Schema
  - 多 expression 文档
  - 增量编译性能保证
```

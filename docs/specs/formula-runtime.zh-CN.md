---
doc_id: specs.formula-runtime
title: "FormulaEngine Rust 交互接口"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-runtime.md
implementation_status: planned
document_status: draft
translation_status: pending
last_verified: 2026-09-03
---

# FormulaEngine Rust 交互接口

[English](formula-runtime.md)

> 本文描述 Planned 接口。实现和英文版本完成前，它不是当前运行时契约。

`FormulaEngine` 是唯一可以独立创建的入口。它持有权威状态，并创建可丢弃的公式草稿：

```text
FormulaEngine
  Schema + FormulaDefinition + 依赖图 + 编译结果
  │
  └─ create_draft(...) → FormulaDraft
                          独立 source 与编译结果
                          不修改 FormulaEngine
```

## 数据模型

```rust
pub struct PropertyId(String);
pub struct RowId(String);
pub struct TextOffset(usize);
pub struct DraftVersion(u64);
pub struct DiagnosticId(String);

pub struct Span {
    pub start: usize,
    pub end: usize,
}

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

pub struct FormulaDefinition {
    pub id: PropertyId,
    pub source: String,
}

pub struct FormulaDiagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<Span>,
}
```

```text
- PropertyId 非空、区分大小写，不执行 Unicode normalization
- property 与 formula 共享一个 ID 命名空间；formula 没有 display name
- Schema 中的 PropertyId 唯一，Type 必须明确
- Type 描述非 null 值的静态类型；所有 property 和 formula 均可为 null
- Span 和 TextOffset 是 UTF-8 byte 半开区间
```

## FormulaEngine

```rust
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    pub fn new(
        schema: Schema,
        formulas: Vec<FormulaDefinition>,
    ) -> Result<Self, FormulaEngineInitError>;

    pub fn state(&self) -> &FormulaEngineState;
    pub fn upsert_property(
        &mut self,
        property: PropertySchema,
    ) -> Result<ChangeResult, EngineChangeError>;
    pub fn remove_property(&mut self, id: &PropertyId) -> Option<ChangeResult>;
    pub fn upsert_formula(
        &mut self,
        formula: FormulaDefinition,
    ) -> Result<ChangeResult, EngineChangeError>;
    pub fn remove_formula(&mut self, id: &PropertyId) -> Option<ChangeResult>;
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;
    pub fn create_draft(
        &self,
        formula: FormulaDefinition,
    ) -> Result<FormulaDraft, CreateDraftError>;
}
```

### 初始化与状态

`new` 接收完整的初始 Schema 和公式集合；数组顺序不影响依赖分析。

```text
FormulaEngineInitError
  - property ID 重复
  - formula ID 重复
  - property 与 formula ID 冲突
  - Schema 包含不支持的 Type

不会导致初始化失败
  - 公式语法或类型错误
  - 依赖缺失
  - 循环依赖
```

Engine 保留不可执行的 definition，并在 state 中报告状态：

```rust
pub struct FormulaEngineState {
    pub schema: Schema,
    pub formulas: Vec<FormulaState>,
}

pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
    pub output_type: Option<Type>,
    pub diagnostics: Vec<FormulaDiagnostic>,
}

pub enum FormulaStatus {
    Ready,   // 当前公式和依赖均可执行
    Invalid, // 当前公式自身存在语法或类型错误
    Blocked, // 依赖缺失、不可执行，或者存在依赖环
}
```

`Ready` 必须带有明确的 `output_type`；`Type` 不包含 `Unknown` 或 `Null`。`state().formulas` 按 ID 确定性排序。依赖图、编译计划和缓存不公开。

### 修改与依赖更新

```rust
pub struct ChangeResult {
    pub affected_formulas: Vec<PropertyId>,
}
```

```text
upsert_property  新增或替换 property；重新分析其直接和间接依赖者
remove_property  删除 property；依赖者转为 Blocked
upsert_formula   新增或替换 formula；重新分析它及其直接和间接依赖者
remove_formula   删除 formula；依赖者转为 Blocked
```

所有依赖更新和重新分析都在方法返回前完成。

```text
- 删除不存在的 ID：None
- upsert 完全相同的值：空 affected_formulas
- affected_formulas：按 ID 排序
- upsert_formula：包含被 upsert 的公式
- remove_formula：只包含删除后仍存在的依赖者
- ID 冲突：EngineChangeError，不修改 Engine
- 公式内容无效：保存 definition，在 state 中标记 Invalid 或 Blocked
```

依赖缺失可在后续修改后恢复：

```text
upsert A（A 依赖尚不存在的 B） → A = Blocked
upsert B                         → 重新分析 A → A = Ready
```

## Evaluate

```rust
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>,
    pub columns: Vec<InputColumn>,
    pub runtime: RuntimeContext,
    pub targets: Vec<PropertyId>,
}

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
}
```

Schema、公式、依赖图和编译结果来自 Engine，不在请求中重复传入。

```text
targets
  - 必填、非空、唯一，且每个 ID 都是 formula
  - 返回顺序与输入顺序一致
  - 只有各 target 的依赖闭包必须为 Ready
  - 任一 target 不可执行时，整个请求返回 EvaluateError

row_ids
  - 每个 ID 非空，批次内唯一；零行批次合法

columns
  - 只接受 Schema property；ID 唯一，顺序无关
  - 必须提供 target 依赖闭包所需的列；允许提供并忽略其他 Schema 列
  - variant 必须匹配 property Type
  - values.len() 和 Bitmap 长度必须等于 row_ids.len()

runtime
  - 在本次请求的所有行和公式中保持不变
  - Date 使用 UTC Unix epoch milliseconds
```

与 targets 无关的 `Invalid` 或 `Blocked` 公式不影响求值。全空列仍使用对应 `Column` variant，并设置 `AllNull`；null 或行错误位置的占位 value 不可读取。

### 结果

```rust
pub struct EvaluateResult {
    pub targets: Vec<TargetResult>,
}

pub struct TargetResult {
    pub id: PropertyId,
    pub output_type: Type,
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

```text
普通 null               → validity = false，无 RowError
行错误                  → validity = false，包含 RowError
请求或 target 状态错误  → EvaluateError，无 EvaluateResult
```

同一 target、同一行可以有多个错误，并保持确定性的求值遍历顺序。第一版总是返回 requested targets 的完整列。

## FormulaDraft

`FormulaDraft` 只能由 Engine 创建。它以当前 Engine 为上下文分析一份未提交的 definition；source 可以与 Engine 中同 ID 的 definition 不同。

```rust
pub struct FormulaDraft { /* private */ }

impl FormulaDraft {
    pub fn state(&self) -> &FormulaDraftState;
    pub fn help(&self, cursor: TextOffset) -> CursorHelp;
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;
    pub fn apply_edits(
        &mut self,
        edit: FormulaEdit,
        cursor: TextOffset,
    ) -> Result<ApplyEditsResult, ApplyEditsError>;
    pub fn into_definition(self) -> FormulaDefinition;
}

pub struct FormulaDraftState {
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    pub output_type: Option<Type>,
    pub diagnostics: Vec<FormulaDiagnostic>,
    pub tokens: Vec<Token>,
}

pub struct CursorHelp {
    pub completions: Vec<Completion>,
    pub signature_help: Option<SignatureHelp>,
}

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
    pub state: FormulaDraftState,
    pub cursor: TextOffset,
}
```

```text
创建
  - 使用创建时的 Schema、其他公式类型和依赖图快照
  - definition ID 可以对应现有 formula，也可以是尚未提交的新 ID
  - definition ID 与 property 冲突时返回 CreateDraftError
  - 用 draft source 替换同 ID 公式的依赖，再检查类型和循环
  - 无效 source 仍创建成功；问题进入 diagnostics
  - 无法推断明确类型时 output_type = None

只读方法
  - state：diagnostics 与 tokens，不依赖 cursor
  - help：一次返回 completion、postfix completion 和 signature help
  - quick_fixes / format_edits：只返回候选 FormulaEdit
  - diagnostic ID 不属于当前 state 时，quick_fixes 返回空列表

apply_edits
  - 是唯一修改 draft 的方法
  - base_version 必须等于 state.version
  - range 和 cursor 必须有效；edits 不重叠，且都基于修改前 source
  - 原子应用全部 edits，重定位 cursor，重新编译并推进 version
  - 成功返回新 state 与 cursor；编辑后的 source 可以无效
  - 失败返回 ApplyEditsError，draft 保持不变

循环依赖
  - 直接或间接引用自身时产生 diagnostic
  - 会形成循环的 completion 仍返回，但标记为 disabled
  - 新公式必须先分配一个不与 property 冲突的 ID
```

`format_edits` 只要求语法可格式化，不要求语义有效。Quick fix 的 `base_version` 将它绑定到产生该 diagnostic 的 draft 版本。

### Commit 与 discard

```rust
let mut draft = engine.create_draft(formula)?;
draft.apply_edits(edit, cursor)?;

if save {
    engine.upsert_formula(draft.into_definition())?; // commit
} else {
    drop(draft); // discard
}
```

Draft 不随 Engine 的后续修改更新。commit 时，`upsert_formula` 根据 Engine 的最新权威状态重新分析 definition；调用方以返回的 `ChangeResult` 和新 Engine state 为准。

## 第一版不提供

```text
FormulaEngine  持久化业务行；公开依赖图、revision 或缓存；批量事务；流或分页
FormulaDraft   隐式修改或自动跟随 Engine；多 expression 文档；增量编译性能保证
```

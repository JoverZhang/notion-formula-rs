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

```rust
// 本文面向 FormulaEngine 的 Rust 调用方。
// 目标是维护一组 property 和 formula、重复求值，并编辑一份可丢弃的公式草稿。
//
// FormulaEngine
//   Schema + FormulaDefinition + 依赖图 + 编译结果
//   │
//   └─ create_draft(...) → FormulaDraft
//                           独立 source 与编译结果
//                           不修改 FormulaEngine
```

## 数据模型

```rust
/// property 和 formula 共享的 ID。
/// 非空、区分大小写，不执行 Unicode normalization。
pub struct PropertyId(String);

pub struct RowId(String);

/// 当前 source 中的 UTF-8 byte offset。
pub struct TextOffset(usize);

pub struct DraftVersion(u64);
pub struct DiagnosticId(String);

/// 当前 source 中的 UTF-8 byte 半开区间 `[start, end)`。
pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub struct Schema {
    /// 每个 PropertySchema.id 必须唯一。
    pub properties: Vec<PropertySchema>,
}

pub struct PropertySchema {
    pub id: PropertyId,

    /// 必须是明确的公开 Type。
    pub ty: Type,
}

/// 非 null 值的静态类型。
/// property 和 formula 默认均可为 null，因此公开类型不包含 Null。
/// Unknown 仅可作为内部推断状态，不属于公开 Type。
pub enum Type {
    Number,
    String,
    Boolean,
    Date,
    List(Box<Type>),
    Union(Vec<Type>),
}

/// 公式的完整持久化定义；不另设 display name。
pub struct FormulaDefinition {
    /// formula 的唯一 ID。
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

## FormulaEngine

```rust
/// 唯一可以独立创建的入口，也是 Schema 和 FormulaDefinition 的权威状态。
///
/// Engine 不持久化业务行，也不公开依赖图、revision 或编译缓存。
/// 第一版不提供批量事务 mutation、流、分页、取消或 row provider。
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    /// 使用完整 Schema 和公式集合创建 Engine。
    /// formulas 的顺序不影响依赖分析。
    ///
    /// 以下结构错误返回 FormulaEngineInitError：
    /// - property ID 重复；
    /// - formula ID 重复；
    /// - property 与 formula ID 冲突；
    /// - Schema 包含不支持的 Type。
    ///
    /// formula 的语法错误、类型错误、依赖缺失或循环依赖不会导致初始化失败。
    /// Engine 保留这些 definition，并通过 state 报告其状态。
    pub fn new(
        schema: Schema,
        formulas: Vec<FormulaDefinition>,
    ) -> Result<Self, FormulaEngineInitError>;

    /// 返回当前权威状态。
    pub fn state(&self) -> &FormulaEngineState;

    /// 新增或替换 property，并在返回前重新分析其直接和间接依赖者。
    /// 完全相同的 property 是 no-op，返回空 affected_formulas。
    /// ID 已属于 formula 时返回 EngineChangeError，且 Engine 保持不变。
    pub fn upsert_property(
        &mut self,
        property: PropertySchema,
    ) -> Result<ChangeResult, EngineChangeError>;

    /// 删除 property，并在返回前将其直接和间接依赖者转为 Blocked。
    /// ID 不存在时返回 None。
    pub fn remove_property(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    /// 新增或替换 formula，并在返回前重新分析它及其直接和间接依赖者。
    /// 完全相同的 definition 是 no-op，返回空 affected_formulas。
    /// ID 已属于 property 时返回 EngineChangeError，且 Engine 保持不变。
    ///
    /// formula 内容无效不属于 EngineChangeError：Engine 保存 definition，
    /// 并在 state 中将它标记为 Invalid 或 Blocked。
    ///
    /// 缺失依赖可以在后续 mutation 后恢复：
    ///
    /// upsert A（A 依赖尚不存在的 B） → A = Blocked
    /// upsert B                         → 重新分析 A → A = Ready
    pub fn upsert_formula(
        &mut self,
        formula: FormulaDefinition,
    ) -> Result<ChangeResult, EngineChangeError>;

    /// 删除 formula，并在返回前将其直接和间接依赖者转为 Blocked。
    /// ID 不存在时返回 None。
    pub fn remove_formula(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    /// 计算 targets 及其依赖，无需调用方自行安排公式顺序。
    /// Schema、formula、依赖图和编译结果均来自 self，不在 EvaluateInput 中重复传入。
    ///
    /// 只有 target 的依赖闭包必须为 Ready；无关的 Invalid 或 Blocked formula
    /// 不影响本次求值。输入不合法或任一 target 不可执行时返回 EvaluateError，
    /// 不返回部分 EvaluateResult。
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;

    /// 基于当前 Engine 快照创建可独立编辑和丢弃的公式草稿。
    /// formula.id 可以对应现有 formula，也可以是尚未提交的新 ID。
    /// formula.id 已属于 property 时返回 CreateDraftError。
    ///
    /// Draft 保存当前 Schema、其他公式类型和依赖图，用自己的 source 替换同 ID
    /// 公式的依赖，再执行类型分析和循环依赖检查。无效 source 仍可创建 Draft，
    /// 问题通过 FormulaDraftState.diagnostics 返回。
    pub fn create_draft(
        &self,
        formula: FormulaDefinition,
    ) -> Result<FormulaDraft, CreateDraftError>;
}
```

### 状态

```rust
pub struct FormulaEngineState {
    pub schema: Schema,

    /// 按 FormulaDefinition.id 确定性排序。
    pub formulas: Vec<FormulaState>,
}

pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,

    /// status == Ready 时必须为 Some(Type)。
    pub output_type: Option<Type>,

    pub diagnostics: Vec<FormulaDiagnostic>,
}

pub enum FormulaStatus {
    /// 当前公式和依赖均可执行。
    Ready,

    /// 当前公式自身存在语法或类型错误。
    Invalid,

    /// 依赖缺失、依赖不可执行，或者存在依赖环。
    Blocked,
}
```

### 修改结果

```rust
pub struct ChangeResult {
    /// 按 PropertyId 确定性排序。
    ///
    /// upsert_formula 时包含被 upsert 的公式；remove_formula 时只包含
    /// 删除后仍然存在的依赖者。no-op 时为空。
    pub affected_formulas: Vec<PropertyId>,
}
```

## Evaluate

```rust
pub struct EvaluateInput {
    /// 为每一行提供稳定身份。每个 RowId 非空且批次内唯一；允许为空数组。
    pub row_ids: Vec<RowId>,

    /// 只接受 Schema property，ID 唯一，顺序不影响语义。
    /// 必须提供 targets 依赖闭包所需的列；允许提供并忽略其他 Schema 列。
    pub columns: Vec<InputColumn>,

    /// 在本次请求的所有行和 formula 中保持不变。
    pub runtime: RuntimeContext,

    /// 必填、非空、唯一，且每个 ID 都必须属于 formula。
    /// EvaluateResult.targets 保持这里的输入顺序。
    pub targets: Vec<PropertyId>,
}

pub struct InputColumn {
    /// 必须是 Schema 中的 property ID。
    pub id: PropertyId,

    /// variant 必须匹配对应 PropertySchema.ty。
    pub data: Column,
}

pub struct RuntimeContext {
    /// 本次请求固定使用的 UTC Unix epoch milliseconds。
    pub evaluated_at_epoch_ms: i64,

    pub timezone_offset_minutes: i32,
}

pub struct ColumnData<T> {
    /// 长度必须等于 EvaluateInput.row_ids.len()。
    /// validity 为 false 的位置是占位值，不得读取。
    pub values: Vec<T>,

    pub validity: Validity,
}

pub enum Validity {
    AllValid,

    /// values 仍须为每一行提供对应 Column variant 的占位值。
    AllNull,

    /// 长度必须等于 ColumnData.values.len()。
    /// true 表示有效值，false 表示 null 或行错误。
    Bitmap(Vec<bool>),
}

pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),

    /// UTC Unix epoch milliseconds。
    Date(i64),

    List(Vec<Option<Value>>),
}

/// 每个 variant 对应一个 Type；全空列也不得丢失 variant。
pub enum Column {
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),

    /// UTC Unix epoch milliseconds。
    Date(ColumnData<i64>),

    List(ColumnData<Vec<Option<Value>>>),
    Union(ColumnData<Value>),
}
```

### 结果

```rust
pub struct EvaluateResult {
    /// 顺序与 EvaluateInput.targets 一致。
    /// 第一版总是返回所有 requested targets 的完整列。
    pub targets: Vec<TargetResult>,
}

pub struct TargetResult {
    pub id: PropertyId,

    /// 明确的非 null 静态类型。
    pub output_type: Type,

    /// 普通 null 和行错误都将对应位置标记为无效。
    pub column: Column,

    /// 普通 null 不产生 RowError；行错误产生一个或多个 RowError。
    /// 同一行可以有多个错误，顺序由确定性的求值遍历决定，不按行重新排序。
    pub errors: Vec<RowError>,
}

pub struct RowError {
    /// 对应 EvaluateInput.row_ids 中的位置。
    pub row_index: usize,

    /// 实际产生错误的 formula，不一定等于 requested target。
    pub origin_formula_id: PropertyId,

    pub code: RowErrorCode,
    pub message: String,
}
```

## FormulaDraft

```rust
/// FormulaDraft 只能由 FormulaEngine 创建。
///
/// 它持有创建时的 Engine 快照，只编辑一个未提交的 definition。source 可以与
/// Engine 中同 ID 的 definition 不同；Draft 不随 Engine 的后续 mutation 更新。
/// Draft 的任何操作都不会隐式修改 Engine。
///
/// 第一版不支持多 expression 文档，也不保证增量编译性能。
pub struct FormulaDraft { /* private */ }

impl FormulaDraft {
    /// 返回当前 definition、diagnostics、tokens 和推断类型。
    pub fn state(&self) -> &FormulaDraftState;

    /// 一次返回 completion、postfix completion 和 signature help。
    /// 会形成循环依赖的 completion 仍然返回，但标记为 disabled。
    pub fn help(&self, cursor: TextOffset) -> CursorHelp;

    /// 返回当前 diagnostic 的候选修复，不直接修改 Draft。
    /// diagnostic_id 不属于当前 state 时返回空列表。
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// 返回格式化 edits，不直接修改 Draft。
    /// 只要求 source 在语法上可格式化，不要求语义有效。
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// 原子应用 edit；这是唯一修改 Draft 的方法。
    ///
    /// 要求：
    /// - edit.base_version == state.version；
    /// - range 和 cursor 有效；
    /// - edits 不重叠，且全部 range 都基于修改前的 source。
    ///
    /// 成功时应用全部 edits、重定位 cursor、重新编译并推进 version。
    /// 编辑后的 source 可以无效；diagnostics 通过新 state 返回。
    /// 失败时返回 ApplyEditsError，Draft 保持不变。
    pub fn apply_edits(
        &mut self,
        edit: FormulaEdit,
        cursor: TextOffset,
    ) -> Result<ApplyEditsResult, ApplyEditsError>;

    /// 消耗 Draft，产生可传给 FormulaEngine::upsert_formula 的 definition。
    pub fn into_definition(self) -> FormulaDefinition;
}

pub struct FormulaDraftState {
    /// 每次成功 apply_edits 后推进。
    pub version: DraftVersion,

    pub definition: FormulaDefinition,

    /// 无法推断明确类型时为 None。
    pub output_type: Option<Type>,

    /// 不依赖 cursor；直接或间接引用自身时包含循环依赖 diagnostic。
    pub diagnostics: Vec<FormulaDiagnostic>,

    pub tokens: Vec<Token>,
}

pub struct CursorHelp {
    pub completions: Vec<Completion>,
    pub signature_help: Option<SignatureHelp>,
}

pub struct TextEdit {
    /// 基于 apply_edits 前的 source。
    pub range: Span,
    pub new_text: String,
}

pub struct FormulaEdit {
    /// 必须等于当前 FormulaDraftState.version。
    pub base_version: DraftVersion,

    /// 所有 range 基于同一个旧 source，且不得重叠。
    pub edits: Vec<TextEdit>,
}

pub struct QuickFix {
    pub title: String,

    /// base_version 绑定到产生该 diagnostic 的 Draft 版本。
    pub edit: FormulaEdit,
}

pub struct ApplyEditsResult {
    pub state: FormulaDraftState,
    pub cursor: TextOffset,
}
```

### Commit 与 discard

```rust
let mut draft = engine.create_draft(formula)?;
draft.apply_edits(edit, cursor)?;

if save {
    // Engine 根据最新权威状态重新分析 definition。
    // commit 结果以 ChangeResult 和新的 FormulaEngineState 为准。
    let change = engine.upsert_formula(draft.into_definition())?;
    let state = engine.state();
} else {
    // 未提交 source 从未进入 Engine。
    drop(draft);
}
```

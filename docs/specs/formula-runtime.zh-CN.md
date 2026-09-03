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
// FormulaEngine 是唯一可以独立创建的入口：
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

// 数据模型约束
// - PropertyId 非空、区分大小写，不执行 Unicode normalization。
// - property 与 formula 共享一个 ID 命名空间；formula 没有 display name。
// - Schema 中的 PropertyId 唯一，Type 必须明确。
// - Type 描述非 null 值的静态类型；所有 property 和 formula 均可为 null。
// - Type 不包含 Unknown 或 Null。
// - Span 和 TextOffset 使用 UTF-8 byte offset；Span 是 [start, end) 半开区间。
```

## FormulaEngine

```rust
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    /// 使用完整 Schema 和公式集合创建 Engine。
    ///
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

    /// 新增或替换 property，并重新分析其直接和间接依赖者。
    pub fn upsert_property(
        &mut self,
        property: PropertySchema,
    ) -> Result<ChangeResult, EngineChangeError>;

    /// 删除 property，并将依赖者转为 Blocked。
    /// ID 不存在时返回 None。
    pub fn remove_property(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    /// 新增或替换 formula，并重新分析它及其直接和间接依赖者。
    pub fn upsert_formula(
        &mut self,
        formula: FormulaDefinition,
    ) -> Result<ChangeResult, EngineChangeError>;

    /// 删除 formula，并将依赖者转为 Blocked。
    /// ID 不存在时返回 None。
    pub fn remove_formula(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    /// 计算 targets 及其依赖，无需调用方自行安排公式顺序。
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;

    /// 基于当前 Engine 快照创建可独立编辑和丢弃的公式草稿。
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
    pub formulas: Vec<FormulaState>,
}

pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
    pub output_type: Option<Type>,
    pub diagnostics: Vec<FormulaDiagnostic>,
}

pub enum FormulaStatus {
    Ready,   // 当前公式和依赖均可执行；output_type 必须为 Some(Type)。
    Invalid, // 当前公式自身存在语法或类型错误。
    Blocked, // 依赖缺失、不可执行，或者存在依赖环。
}

// 状态约束
// - formulas 按 PropertyId 确定性排序。
// - 依赖图、编译计划和缓存属于私有状态。
```

### 修改结果

```rust
pub struct ChangeResult {
    pub affected_formulas: Vec<PropertyId>,
}

// 修改约束
// - 依赖更新和重新分析必须在方法返回前完成。
// - upsert 完全相同的值是 no-op，返回空 affected_formulas。
// - affected_formulas 按 PropertyId 确定性排序。
// - upsert_formula 的结果包含被 upsert 的公式。
// - remove_formula 的结果只包含删除后仍存在的依赖者。
// - property 与 formula ID 冲突时返回 EngineChangeError，且 Engine 保持不变。
// - formula 内容无效不属于 mutation error；保存 definition，并标记 Invalid 或 Blocked。
//
// 缺失依赖可以在后续 mutation 后恢复：
//
// upsert A（A 依赖尚不存在的 B） → A = Blocked
// upsert B                         → 重新分析 A → A = Ready
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

// 请求约束
// - Schema、formula、依赖图和编译结果来自 Engine，不在请求中重复传入。
// - targets 必填、非空、唯一，且每个 ID 都是 formula。
// - 只有各 target 的依赖闭包必须为 Ready。
// - 任一 target 不可执行时，整个请求返回 EvaluateError。
// - 与 targets 无关的 Invalid 或 Blocked formula 不影响本次求值。
// - row_ids 中每个 ID 非空且批次内唯一；零行批次合法。
// - columns 只接受 Schema property；ID 唯一，顺序无关。
// - 必须提供 target 依赖闭包所需的列；允许提供并忽略其他 Schema 列。
// - Column variant 必须匹配 property Type。
// - values.len() 以及 Bitmap 长度必须等于 row_ids.len()。
// - 全空列仍使用对应 Column variant，并设置 Validity::AllNull。
// - null 或行错误位置的占位 value 不可读取。
// - RuntimeContext 在本次请求的所有行和 formula 中保持不变。
// - Date 使用 UTC Unix epoch milliseconds。
```

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

// 结果约束
// - targets 顺序与 EvaluateInput.targets 一致。
// - 第一版总是返回 requested targets 的完整列。
// - 普通 null：validity = false，不产生 RowError。
// - 行错误：validity = false，并产生一个或多个 RowError。
// - 请求或 target 状态错误：返回 EvaluateError，不返回 EvaluateResult。
// - 同一 target、同一行可以有多个错误。
// - RowError 保持确定性的求值遍历顺序，不按行重新排序。
```

## FormulaDraft

```rust
/// FormulaDraft 只能由 FormulaEngine 创建。
///
/// 它使用创建时的 Engine 快照分析一份未提交的 definition。
/// source 可以与 Engine 中同 ID 的 committed definition 不同。
/// Draft 的任何操作都不会修改 Engine。
pub struct FormulaDraft { /* private */ }

impl FormulaDraft {
    /// 返回 diagnostics、tokens、推断类型和当前 definition。
    pub fn state(&self) -> &FormulaDraftState;

    /// 一次返回 completion、postfix completion 和 signature help。
    pub fn help(&self, cursor: TextOffset) -> CursorHelp;

    /// 返回当前 diagnostic 的候选修复；未知 ID 返回空列表。
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// 返回格式化 edits，不直接修改 draft。
    /// source 无法完成语法格式化时返回 FormatError。
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// 原子应用 edits；这是唯一修改 draft 的方法。
    pub fn apply_edits(
        &mut self,
        edit: FormulaEdit,
        cursor: TextOffset,
    ) -> Result<ApplyEditsResult, ApplyEditsError>;

    /// 消耗 draft，产生可传给 FormulaEngine::upsert_formula 的 definition。
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

// 创建约束
// - definition ID 可以对应现有 formula，也可以是尚未提交的新 ID。
// - definition ID 与 property 冲突时返回 CreateDraftError。
// - Draft 保存创建时的 Schema、其他公式类型和依赖图快照。
// - Draft 用自己的 source 替换同 ID 公式的依赖，再检查类型和循环。
// - 无效 source 仍创建成功；问题进入 diagnostics。
// - 无法推断明确类型时 output_type = None。
// - diagnostics 不依赖 cursor。
//
// 循环依赖约束
// - 直接或间接引用自身时产生 diagnostic。
// - 会形成循环的 completion 仍返回，但标记为 disabled。
// - 新公式必须先分配一个不与 property 冲突的 ID。
//
// FormulaEdit 约束
// - quick_fixes 和 format_edits 只返回候选 FormulaEdit，调用方可以丢弃。
// - QuickFix.edit.base_version 绑定到产生该 diagnostic 的 draft 版本。
// - apply_edits 要求 base_version == state.version。
// - range 和 cursor 必须有效；edits 不得重叠，且都基于修改前 source。
// - apply_edits 原子应用全部 edits，重定位 cursor，重新编译并推进 version。
// - 编辑后的 source 可以无效；成功结果通过新 state 返回 diagnostics。
// - 失败时返回 ApplyEditsError，draft 保持不变。
```

### Commit 与 discard

```rust
let mut draft = engine.create_draft(formula)?;
draft.apply_edits(edit, cursor)?;

if save {
    // commit：Engine 根据最新权威状态重新分析 definition。
    let change = engine.upsert_formula(draft.into_definition())?;
    let state = engine.state();
} else {
    // discard：未提交 source 从未进入 Engine。
    drop(draft);
}

// Draft 不随 Engine 的后续 mutation 更新。
// commit 结果以 ChangeResult 和新的 FormulaEngineState 为准。
```

## 第一版不提供

```rust
// FormulaEngine
// - 持久化业务行；
// - 公开依赖图、revision 或编译缓存；
// - 批量事务 mutation；
// - 流、分页、取消或 row provider。
//
// FormulaDraft
// - 隐式修改 FormulaEngine；
// - 自动跟随 Engine state 更新；
// - 多 expression 文档；
// - 增量编译性能保证。
```

---
doc_id: specs.formula-runtime
title: "FormulaEngine：编译与执行"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-runtime.md
implementation_status: planned
document_status: draft
translation_status: synced
last_verified: 2026-09-18
---

# FormulaEngine：编译与执行

[English](formula-runtime.md) · [规格索引](README.zh-CN.md)

> Planned：本文是 Rust 接口草案，不是当前已发布 API。错误枚举的成员尚待细化。

## 入口

```rust
// Engine 持有 Schema、公式、依赖分析和编译结果；不保存业务行。
// 不公开依赖图、revision 或缓存；v1 不提供批量事务、流、分页、取消、row provider。
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    // formulas 的输入顺序不影响依赖分析。
    // 重复 ID、property/formula ID 冲突、不支持的 Schema 类型 → Err。
    // 公式语法/类型错误、缺失依赖、依赖环 → 保留 definition，在 state 中报告。
    pub fn new(schema: Schema, formulas: Vec<FormulaDefinition>)
        -> Result<Self, FormulaEngineInitError>;
    pub fn state(&self) -> &FormulaEngineState;

    // 返回前重新分析公式自身及直接、间接依赖者；相同内容为 no-op。
    // ID 已属于另一种定义 → Err，Engine 不变。
    pub fn upsert_property(&mut self, property: PropertySchema)
        -> Result<ChangeResult, EngineChangeError>;
    pub fn upsert_formula(&mut self, formula: FormulaDefinition)
        -> Result<ChangeResult, EngineChangeError>;
    // 无效 source 仍保存；它不属于 EngineChangeError。

    // 不存在 → None；存在 → 删除并将仍依赖它的公式转为 Blocked。
    pub fn remove_property(&mut self, id: &PropertyId) -> Option<ChangeResult>;
    pub fn remove_formula(&mut self, id: &PropertyId) -> Option<ChangeResult>;

    // 使用 self 的编译状态，只要求 targets 的依赖闭包全部 Ready。
    // 无关的 Invalid/Blocked 公式不影响求值。
    // 输入不合法或任一 target 不可执行 → Err，不返回部分结果。
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateError>;

    // 创建独立编辑快照；不修改 Engine。细节由 FormulaDraft 规格维护。
    pub fn create_draft(&self, formula: FormulaDefinition)
        -> Result<FormulaDraft, CreateDraftError>;
}

// 先插入 A（依赖尚不存在的 B） → A: Blocked
// 再插入可执行的 B             → 重新分析 A → A: Ready
// Rust 生命周期由所有权/Drop 管理；Worker close() 见 WASM API。
```

[FormulaDraft](formula-draft.zh-CN.md) 定义编辑与提交；
[WASM API](formula-runtime-wasm.zh-CN.md) 定义 Worker 和关闭行为。

## 定义与编译状态

```rust
pub struct PropertyId(String); // property/formula 共用；非空、区分大小写、不做 Unicode 归一化
pub struct RowId(String);
pub struct DiagnosticId(String);
pub struct Span { pub start: usize, pub end: usize } // UTF-8 byte，[start, end)

pub struct Schema {
    pub properties: Vec<PropertySchema>, // ID 唯一
}
pub struct PropertySchema {
    pub id: PropertyId,
    pub ty: Type,
}
pub enum Type { // 非 null 值的明确静态类型；所有字段默认 nullable
    Number, String, Boolean, Date,
    List(Box<Type>),
    Union(Vec<Type>),
    // 无公开 Null/Unknown；Unknown 只允许作为内部推断状态。
}
pub struct FormulaDefinition {
    pub id: PropertyId, // 不再维护独立 name
    pub source: String,
}
pub struct FormulaDiagnostic {
    pub id: DiagnosticId,
    pub code: DiagnosticCode, // 枚举成员待细化；message 不作为机器判断依据
    pub message: String,
    pub span: Option<Span>,
}

pub struct FormulaEngineState {
    pub schema: Schema,
    pub formulas: Vec<FormulaState>, // 按 definition.id 确定性排序
}
pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
    pub output_type: Option<Type>, // Ready 必须为 Some(Type)
    pub diagnostics: Vec<FormulaDiagnostic>,
}
pub enum FormulaStatus {
    Ready,   // 自身及依赖可执行
    Invalid, // 自身语法或类型错误
    Blocked, // 依赖缺失、不可执行或存在环
}
pub struct ChangeResult {
    // 按 ID 确定性排序；no-op 为空。
    // upsert_formula 包含自身；remove_formula 只包含删除后仍存在的依赖者。
    pub affected_formulas: Vec<PropertyId>,
}
```

## 列式求值

```rust
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>, // 每个 ID 非空、批次内唯一；允许零行
    pub columns: Vec<InputColumn>, // ID 唯一，只接受 Schema property，列顺序无关
    // 必须提供 targets 依赖闭包需要的列；允许附带并忽略其他 Schema 列。
    pub runtime: RuntimeContext, // 本次请求的所有行和公式共用同一快照
    pub targets: Vec<PropertyId>, // 必填、非空、唯一，且必须是 formula ID
}
pub struct InputColumn {
    pub id: PropertyId,
    pub data: Column, // variant 必须匹配 PropertySchema.ty
}
pub struct RuntimeContext {
    pub evaluated_at_epoch_ms: i64, // UTC Unix epoch milliseconds
    pub timezone_offset_minutes: i32,
}

pub struct ColumnData<T> {
    pub values: Vec<T>, // 长度 == row_ids.len()；无效位置的占位值不得读取
    pub validity: Validity,
}
pub enum Validity {
    AllValid,
    AllNull, // values 仍需为每行保留有类型的占位值
    Bitmap(Vec<bool>), // 长度 == values.len()；false 表示 null 或行错误
}
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    Date(i64), // UTC Unix epoch milliseconds
    List(Vec<Option<Value>>),
}
pub enum Column { // 全空列也保留 variant
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    Union(ColumnData<Value>),
}

pub struct EvaluateResult {
    pub targets: Vec<TargetResult>, // 按 input.targets 顺序返回全部完整列
}
pub struct TargetResult {
    pub id: PropertyId,
    pub output_type: Type, // 明确的非 null 静态类型
    pub column: Column, // 普通 null、行错误均标记为无效
    pub errors: Vec<RowError>, // 普通 null 不报错；同一行可有多个错误
    // 错误按确定性的求值遍历顺序返回，不额外按行排序。
}
pub struct RowError {
    pub row_index: usize, // 对应 input.row_ids 中的位置
    pub origin_formula_id: PropertyId, // 实际出错的公式，可以是 target 的依赖
    pub code: RowErrorCode, // 枚举成员待细化
    pub message: String,
}
```

表达式、引用和逐行运算规则见[公式文法](formula-language.zh-CN.md)；
函数签名见[builtin](builtin-functions.zh-CN.md)。这两份文档的 Current 行为不代表本接口已经实现。

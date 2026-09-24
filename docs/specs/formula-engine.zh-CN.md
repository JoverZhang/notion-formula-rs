---
doc_id: specs.formula-engine
title: "FormulaEngine：编译与求值"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-engine.md
implementation_status: current
document_status: draft
translation_status: synced
last_verified: 2026-09-24
---

# FormulaEngine：编译与求值

[English](formula-engine.md)

> Current：定义管理、依赖分析与状态查询。Planned：求值与 FormulaDraft。

**目录**

- [类型定义](#类型定义)
- [FormulaEngine API](#formulaengine-api)

## 类型定义

下面的定义与状态类型属于 Current；求值数据类型仍属于 Planned。

```rust
use std::collections::HashMap;
```

**当前 Schema 类型**

```rust out=formula_engine/src/formula_engine.h.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaSchema {
    pub properties: Vec<PropertyDefinition>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyDefinition {
    Input { id: PropertyId, ty: ValueType },
    Formula(FormulaDefinition),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaDefinition {
    pub id: PropertyId,
    /// 公式定义，可通过 prop("id") 按 ID 引用同一 FormulaSchema 中其它 Property。
    pub expression: String,
}

/// 由调用方指定的稳定 ID。
///
/// - Input 与 Formula 共用同一命名空间；Engine 校验 ID 非空及 FormulaSchema 内唯一。
/// - 不限制字符串格式；区分大小写，不做 Unicode 归一化。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::From)]
#[from(String, &str)]
pub struct PropertyId(pub String);

/// Input 的声明类型与 Formula 的推断类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    /// 数值规则见 [Planned Number](formula-language.zh-CN.md#planned-number)。
    Number,
    String, Boolean, Date,
    /// 静态类型未确定；可用于 Input 声明和推断结果，包括嵌套类型。
    /// 未知部分允许所有 Value 类型，具体操作在运行时校验实际类型。
    Unknown,
    List(Box<ValueType>),
    Union(Vec<ValueType>),
}

```

**计划中的运行时值类型**

```rust
/// 用于存放运行时数据
/// 列式结构，内部使用 bitmap 标记无效位
pub enum Column {
    Number(ColumnData<f64>),
    String(ColumnData<String>),
    Boolean(ColumnData<bool>),
    Date(ColumnData<i64>),
    List(ColumnData<Vec<Option<Value>>>),
    /// 可承载 Union 或 Unknown；每个非 null 值保留实际类型。
    Union(ColumnData<Value>),
}
pub struct ColumnData<T> {
    /// 长度等于本次求值的行数；null 位置仅保留占位值，不得读取。
    pub values: Vec<T>,
    /// 参考 [Arrow NullBuffer](https://arrow.apache.org/rust/arrow_buffer/buffer/struct.NullBuffer.html)。
    /// 长度等于 values.len()；普通 null 和行错误均标为 null。
    pub validity: NullBuffer,
}
pub enum Value {
    Number(f64), String(String), Boolean(bool),
    /// UTC Unix 毫秒时间戳。
    Date(i64),
    List(Vec<Option<Value>>),
}

```

**当前状态与变更类型**

```rust out=formula_engine/src/formula_engine.h.rs
#[derive(Debug, PartialEq, Eq)]
pub enum FormulaEngineState<'a> {
    /// 无环且所有公式均为 Ready。
    AllReady,
    /// 即使有公式尚未就绪，其他公式仍可保持 Ready。
    NotAllReady {
        /// 有环时返回一条路径；前一个 ID 直接依赖后一个 ID，首尾 ID 相同。
        /// - empty: 无环路，仍有未 Ready 的公式。
        /// - [A, A]: 自引用 A -> A
        /// - [A, B, C, A]: 环路 A -> B -> C -> A
        cycle_path: &'a [PropertyId],
    },
}

/// 字段定义与状态的独立快照，不随 Engine 后续更新而变化。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyState {
    Input { id: PropertyId, ty: ValueType },
    Formula(FormulaState),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormulaState {
    pub definition: FormulaDefinition,
    pub status: FormulaStatus,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormulaStatus {
    /// 公式自身及其依赖都可以执行。
    /// output_type 可包含 Unknown；例如 [] 为 Ready，类型为 List(Unknown)。
    Ready { output_type: ValueType },
    NotReady,
}

/// FormulaEngine::upsert / remove 的变更结果。
#[derive(Debug, PartialEq, Eq)]
pub struct FormulaEngineChangeResult {
    /// 受本次变更影响的 Formula ID，包括直接、间接依赖者。
    /// - 仅包含变更后仍存在的 Formula。
    /// - upsert 新增或修改 Formula 时，也包含它自身。
    /// - 定义内容没有发生变化时为空。
    pub affected_formulas: Vec<PropertyId>,
}

/// FormulaEngine::new 接收的定义不合法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaEngineInitError {
    EmptyId,
    DuplicateId(PropertyId),
}

/// FormulaEngine::upsert 接收的定义不合法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineChangeError {
    EmptyId,
}
```

**计划中的求值请求与结果类型**

```rust
/// FormulaEngine::evaluate() 的参数
pub struct EvaluateInput {
    /// 输入列和结果列中的值均按此顺序排列。
    /// ID 非空且在本批次内唯一；允许零行。
    pub row_ids: Vec<RowId>,
    /// 按 ID 与 Engine 中的全部 Input 一一对应，包括本次求值未使用的列。
    /// evaluate() 运行时校验列及嵌套值的类型，以及 values 和 validity 的长度。
    pub columns: HashMap<PropertyId, Column>,
    pub runtime: RuntimeContext,
    /// 请求返回结果的 Formula ID；依赖公式由 Engine 自动求值。
    /// 非空且不重复；每个 ID 均须指向 Engine 中的 Formula。
    pub formula_ids: Vec<PropertyId>,
}
#[derive(derive_more::From)]
#[from(String, &str)]
pub struct RowId(pub String);

/// 一次求值中，所有行和公式共用这份时间与时区快照。
pub struct RuntimeContext {
    /// now() 使用的 UTC Unix 毫秒时间戳，由调用方提供；Engine 不读取时钟。
    /// 实时求值取请求开始时的时间；测试或重放可传固定值。
    pub evaluated_at_epoch_ms: i64,
    /// 本地时间减 UTC 的分钟数：UTC+08:00 = 480，UTC-05:00 = -300。
    /// 取 evaluated_at_epoch_ms 时业务/用户时区的偏移，供 today() 和日期操作使用。
    /// 整次求值使用固定偏移，不随被计算日期应用夏令时规则。
    pub timezone_offset_minutes: i32,
}

pub struct EvaluateResult {
    /// 与 input.formula_ids 一一对应，包括求值失败的公式。
    pub formulas: HashMap<PropertyId, Result<FormulaOutput, FormulaEvaluationError>>,
}
/// 所有行都失败时，仍以 Ok(FormulaOutput) 返回，错误记录在 errors 中。
pub struct FormulaOutput {
    /// 本次求值时的输出类型。
    pub output_type: ValueType,
    pub column: Column,
    /// 同一行可有多个错误。
    /// 相同定义和输入下，错误顺序一致；不保证按 row_index 排序。
    pub errors: Vec<RowError>,
}
/// 单行求值失败，对应结果位置标为 null；其他行继续求值。
/// 依赖错误只沿实际执行的分支传播，不改变 FormulaStatus。
pub struct RowError {
    /// input.row_ids 的下标。
    pub row_index: usize,
    /// 最初出错的 Formula ID，可能指向请求公式的依赖。
    pub origin_formula_id: PropertyId,
    pub error: RuntimeError,
}
/// constraint 和 detail 仅用于展示。
pub enum RuntimeError {
    /// 运行时值的类型不适用于当前操作。
    InvalidValueType {
        expected: ValueType,
        actual: ValueType,
    },
    /// 类型正确，但值违反函数约束；正则和日期问题使用下方的具体分类。
    InvalidValue {
        actual: Value,
        /// 例如“repeat 的次数必须非负”。
        constraint: String,
    },
    InvalidRegex {
        pattern: String,
        /// 正则编译失败的说明。
        detail: String,
    },
    /// 日期文本无法解析。
    InvalidDateText {
        text: String,
    },
    /// 日期运算超出支持范围。
    DateOutOfRange,
}
/// 公式无法开始求值时返回；单行求值错误见 RowError。
pub enum FormulaEvaluationError {
    /// 对应 Formula 的 FormulaStatus 为 NotReady。
    NotReady,
}
```

## FormulaEngine API

定义和状态方法属于 Current。下方的求值与 Draft 方法仍属于 Planned。

### 计划中的求值示例

```rust
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// // 1. Define schema
/// let schema = FormulaSchema {
///     properties: vec![
///         PropertyDefinition::Input { id: "text".into(), ty: ValueType::String },
///         PropertyDefinition::Input { id: "number".into(), ty: ValueType::Number },
///         PropertyDefinition::Formula(FormulaDefinition {
///             id: "formula".into(),
///             expression: r#"repeat(prop("text"), prop("number"))"#.into(),
///         }),
///     ],
/// };
///
/// // 2. Create the engine
/// let engine = FormulaEngine::new(schema).expect("valid definitions");
/// assert!(matches!(engine.state(), FormulaEngineState::AllReady));
///
/// // 3. Capture one runtime snapshot (this example uses UTC+08:00)
/// let since_epoch = SystemTime::now()
///     .duration_since(UNIX_EPOCH)
///     .expect("system clock is before Unix epoch");
/// let runtime = RuntimeContext {
///     evaluated_at_epoch_ms: since_epoch.as_millis().try_into().expect("timestamp exceeds i64"),
///     timezone_offset_minutes: 8 * 60, // Use the business/user time zone's offset.
/// };
///
/// // 4. Build input columns in row_ids order
/// // row_id    text    number
/// // row-1     "ha"    2
/// // row-2     "go"    3
/// let input = EvaluateInput {
///     row_ids: vec!["row-1".into(), "row-2".into()],
///     columns: HashMap::from([
///         (
///             "text".into(),
///             Column::String(ColumnData {
///                 values: vec!["ha".into(), "go".into()],
///                 validity: NullBuffer::new_valid(2),
///             }),
///         ),
///         (
///             "number".into(),
///             Column::Number(ColumnData {
///                 values: vec![2.0, 3.0],
///                 validity: NullBuffer::new_valid(2),
///             }),
///         ),
///     ]),
///     runtime,
///     formula_ids: vec!["formula".into()],
/// };
///
/// // 5. Evaluate the requested formula
/// let result = engine.evaluate(&input).expect("valid request");
///
/// // 6. Read the result column in row_ids order
/// let Some(Ok(output)) = result.formulas.get(&PropertyId::from("formula")) else {
///     panic!("expected a computed formula");
/// };
/// let Column::String(column) = &output.column else {
///     panic!("expected a string column");
/// };
/// assert_eq!(column.values, ["haha", "gogogo"]);
/// assert!(output.errors.is_empty());
/// ```
```

### 当前的定义管理方法

```rust out=formula_engine/src/formula_engine.h.rs
#[spec::private_fields]
pub struct FormulaEngine {}

#[spec::header]
impl FormulaEngine {
    /// allows:
    /// - 任意定义顺序。
    /// - 保存存在语法/类型错误、缺失依赖、未就绪依赖或依赖环的公式定义。
    ///
    /// errors:
    /// - ID 为空。
    /// - ID 重复。
    pub fn new(schema: FormulaSchema) -> Result<Self, FormulaEngineInitError>;

    pub fn property(&self, id: &PropertyId) -> Option<PropertyState>;
    pub fn properties(&self) -> Vec<PropertyState>;
    pub fn state(&self) -> FormulaEngineState<'_>;

    /// 原子更新 PropertyDefinition。
    /// 返回前重新分析该 ID 的直接、间接公式依赖者及新的 Formula 定义。
    ///
    /// no-op: 定义内容没有发生变化。
    ///
    /// allows:
    /// - 同一 ID 的 Input 与 Formula 互换。
    /// - 保存存在语法/类型错误、缺失依赖、未就绪依赖或依赖环的公式定义。
    ///
    /// error: ID 为空串。
    pub fn upsert(&mut self, property: PropertyDefinition)
        -> Result<FormulaEngineChangeResult, EngineChangeError>;

    /// 删除后，仍依赖该 ID 的公式变为 NotReady。
    pub fn remove(&mut self, id: &PropertyId) -> Option<FormulaEngineChangeResult>;
}
```

### 计划中的求值与 Draft 方法

```rust
impl FormulaEngine {

    /// 输入校验通过即返回 Ok(EvaluateResult)，包括所有请求的公式都失败的情况。
    /// 公式与行错误随结果返回；其他公式继续求值。
    ///
    /// allows: formula_ids 中存在 NotReady 的公式。
    ///
    /// errors: 入参校验失败，不执行任何公式。
    /// - EvaluateInputError::MissingInputs：缺少 Input 列，报告全部缺失的 ID，去重并按 ID 排序。
    /// - 多余列，或 ID、类型、长度不合法。
    pub fn evaluate(&self, input: &EvaluateInput) -> Result<EvaluateResult, EvaluateInputError>;

    /// 基于 Engine 分析候选公式；编辑结果不写入 Engine。
    ///
    /// allows:
    /// - 新 ID；在 Draft 的分析中替换同 ID 的 Input 或 Formula。
    /// - 存在语法/类型错误、依赖缺失或不可执行、依赖环的公式定义。见 FormulaDraft::state()
    ///
    /// error: ID 为空串。
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    /// // 1. Create an engine with a saved formula
    /// let mut engine = FormulaEngine::new(FormulaSchema {
    ///     properties: vec![PropertyDefinition::Formula(FormulaDefinition {
    ///         id: "formula".into(),
    ///         expression: "1 + 1".into(),
    ///     })],
    /// })?;
    ///
    /// // 2. Create a draft from the saved definition
    /// let Some(PropertyState::Formula(saved)) = engine.property(&"formula".into()) else {
    ///     panic!("expected a saved formula");
    /// };
    /// let mut draft = engine.create_draft(saved.definition)?;
    /// assert_eq!(draft.state().definition.expression, "1 + 1");
    ///
    /// // 3. Edit the draft; the engine keeps the saved definition
    /// draft.update_expression(ExpressionUpdate::Replace("1 + 2".into()))?;
    /// assert_eq!(draft.state().definition.expression, "1 + 2");
    /// assert!(draft.state().diagnostics.is_empty());
    /// assert!(matches!(
    ///     engine.property(&"formula".into()),
    ///     Some(PropertyState::Formula(saved)) if saved.definition.expression == "1 + 1"
    /// ));
    ///
    /// // 4. Finish editing and save the definition
    /// let definition = draft.into_definition();
    /// engine.upsert(PropertyDefinition::Formula(definition))?;
    /// let Some(PropertyState::Formula(saved)) = engine.property(&"formula".into()) else {
    ///     panic!("expected a saved formula");
    /// };
    /// assert_eq!(saved.definition.expression, "1 + 2");
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_draft(&self, formula: FormulaDefinition)
        -> Result<FormulaDraft<'_>, CreateDraftError>;
}
```

- [FormulaDraft](ide.zh-CN.md)

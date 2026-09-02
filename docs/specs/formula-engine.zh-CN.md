---
doc_id: specs.formula-engine
title: "FormulaEngine 交互接口"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-engine.md
implementation_status: planned
document_status: draft
translation_status: pending
last_verified: 2026-09-02
---

# FormulaEngine 交互接口

[English](formula-engine.md)

> 本文描述尚未实现的 Planned 接口。对应实现和英文版本完成前，本文不是当前运行时契约。

本文回答一个问题：宿主系统如何注册一组公式，并针对调用方提供的一批行计算指定公式列？这份接口让 Rust 集成者和浏览器 Adapter 作者共享同一套状态、错误与顺序语义，避免两端分别定义公式生命周期。

核心模型只有两类状态：

```text
FormulaEngine 持久状态
  Schema
  已提交的 FormulaDefinition
  依赖图
  分析结果与执行计划

每次 evaluate 的临时输入
  row_ids
  基础属性 columns
  RuntimeContext
  targets
```

`FormulaEngine` 不保存业务行数据，也不保存编辑器草稿。

## 接口总览

Rust 接口是规范源。下列定义省略派生 trait、构造辅助函数和私有字段：

```rust
pub struct FormulaEngine { /* private */ }

impl FormulaEngine {
    pub fn new(schema: Schema) -> Result<Self, SchemaError>;

    pub fn state(&self) -> FormulaEngineState;

    pub fn upsert_formula(
        &mut self,
        formula: FormulaDefinition,
    ) -> Result<UpsertFormulaResult, UpsertFormulaError>;

    pub fn remove_formula(
        &mut self,
        id: &PropertyId,
    ) -> Option<RemoveFormulaResult>;

    pub fn evaluate(
        &self,
        input: &EvaluateInput,
        targets: &[PropertyId],
    ) -> Result<EvaluateResult, EvaluateRequestError>;
}
```

浏览器接口是运行于 Dedicated Worker 中的异步 Adapter：

```ts
interface FormulaEngine {
  getState(): Promise<FormulaEngineState>;

  upsertFormula(
    formula: FormulaDefinition,
  ): Promise<UpsertFormulaResult>;

  removeFormula(
    id: PropertyId,
  ): Promise<RemoveFormulaResult | null>;

  evaluate(
    input: EvaluateInput,
    targets: readonly PropertyId[],
  ): Promise<EvaluateResult>;

  close(): Promise<void>;
}

declare function createFormulaEngine(
  schema: Schema,
): Promise<FormulaEngine>;
```

Adapter 只负责 RPC、队列、生命周期和无损 DTO 转换。公式分析、依赖维护和求值语义必须由 Rust 实现。

## Engine 拥有什么

```text
FormulaEngine owns
  ✓ 固定 Schema
  ✓ 已提交公式及其原始 source
  ✓ 缺失引用和依赖关系
  ✓ FormulaState
  ✓ 私有分析结果、执行计划和缓存

FormulaEngine does not own
  ✗ 基础属性的业务数据
  ✗ 行的长期存储
  ✗ 公式显示名称
  ✗ 编辑器中的未提交 source
  ✗ FormulaAnalyzer 的草稿状态
  ✗ 公共 revision
```

`new` 创建一个公式集合为空、Schema 固定的 Engine。第一版不支持修改 Schema；需要新 Schema 时，宿主创建新的 Engine。

## 标识符与 Schema

### 标识符

```rust
pub struct PropertyId(String);
pub struct RowId(String);

pub enum IdentifierError {
    Empty,
}

impl PropertyId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError>;
}

impl RowId {
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError>;
}
```

两种标识符遵守相同的文本规则，但属于不同命名空间：

```text
有效值        非空 Unicode 字符串
比较方式      区分大小写，逐值精确比较
规范化        Engine 不执行 Unicode normalization
显示名称      不属于标识符，也不由 Engine 保存
```

`PropertyId` 同时标识基础属性和公式输出。两者共享一个命名空间，不得重名。公式中的 `prop("x")` 精确引用 `PropertyId("x")`。

第一版不提供 rename。修改公式 ID 等价于：

```rust
engine.remove_formula(&old_id);
engine.upsert_formula(FormulaDefinition {
    id: new_id,
    source,
});
```

### Schema

```rust
pub struct Schema {
    pub properties: Vec<PropertySchema>,
}

pub struct PropertySchema {
    pub id: PropertyId,
    pub ty: Type,
}
```

Schema 只声明宿主提供的基础属性。公式不出现在 `Schema.properties` 中。

```text
Schema invariants
  - PropertyId 唯一且有效
  - Type 是可执行的具体类型
  - Type 不包含 Union、Null 或 Unknown
  - properties 的输入顺序保留在 state() 中
  - 属性查询和 evaluate 输入不依赖数组顺序
  - 所有基础属性列都允许顶层 null；Schema 不另设 nullable
```

违反这些约束时，`FormulaEngine::new` 返回一个 `SchemaError`，Engine 不会被创建：

```rust
pub enum SchemaError {
    DuplicatePropertyId { id: PropertyId },
    UnsupportedPropertyType { id: PropertyId, ty: Type },
}
```

空属性 ID 在构造 `PropertyId` 时已经返回 `IdentifierError`，不能进入 Schema。

## 类型、公式和状态

### 公共类型

```rust
pub enum Type {
    Number,
    String,
    Boolean,
    Date,
    List(Box<Type>),
    Union(Vec<Type>),
    Null,
}
```

`Unknown` 只允许出现在分析器内部，不得出现在 `Schema`、`FormulaState.output_type` 或求值结果的公共类型中。

```text
Ready formula       output_type = Some(Type)
Invalid formula     output_type = None
Blocked formula     output_type = None
```

纯 `null` 公式的输出类型是 `Some(Type::Null)`，仍属于 `Ready`。

### 已提交公式

```rust
pub struct FormulaDefinition {
    pub id: PropertyId,
    pub source: String,
}
```

Engine 原样保留 `source`。相同 ID 下，只有 source 的逐字节内容也相同时，upsert 才是 no-op。

空 source、语法错误、类型错误、缺失引用和依赖环都不是结构性写入失败。Engine 保存该定义，并通过 `FormulaState` 表示它当前是否可执行。

### FormulaState

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
    Ready,
    Invalid,
    Blocked,
}
```

状态判定顺序固定为：

```text
1. 当前公式自身存在语法或语义错误
     => Invalid

2. 当前公式自身可分析，但依赖缺失、Invalid、Blocked 或处于依赖环
     => Blocked

3. 当前公式及其依赖均可执行
     => Ready
```

`state()` 返回完全拥有数据的快照，不返回 Engine 内部引用。`formulas` 按 `PropertyId` 的确定性顺序排列。快照不包含依赖图、执行计划、缓存或 revision。

### Diagnostic

```rust
pub struct FormulaDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Option<Span>,
}

pub struct DiagnosticCode(String);

pub struct Span {
    pub start: usize,
    pub end: usize,
}
```

```text
code       稳定、可供程序判断
message    面向人类，不保证逐字稳定
span       [start, end) 半开区间
Rust span  UTF-8 byte offset
TS span    UTF-16 code-unit offset
```

没有自然文本位置的图诊断，例如依赖环，可以返回 `span = None`。同一 Engine 状态下，diagnostics 的内容和顺序必须确定。

## 写入公式

### upsert_formula

```rust
pub struct UpsertFormulaResult {
    pub state: FormulaState,
    pub changed: bool,
    pub affected_dependents: Vec<FormulaState>,
}
```

`state` 是被写入公式的新状态。`affected_dependents` 不包含它本身，而包含所有需要重新分析的下游公式，包括间接依赖者。

```text
upsert same id + byte-identical source
  changed               = false
  state                 = existing state
  affected_dependents   = []

upsert new or changed source
  changed               = true
  state                 = committed state after recomputation
  affected_dependents   = downstream states after recomputation
```

即使某个下游公式的 `FormulaState` 字段最终没有变化，只要上游 source 的变化可能改变它的列值，它仍属于 `affected_dependents`。列表按与 `FormulaEngineState.formulas` 相同的 ID 顺序排列。

一次 upsert 是一个原子提交：

```text
structural validation fails
  => Err(UpsertFormulaError)
  => Engine unchanged

structural validation succeeds
  => definition stored
  => graph and affected states recomputed
  => Ok(UpsertFormulaResult)
```

公式 ID 与 Schema 中的基础属性冲突是 upsert 的结构性错误：

```rust
pub enum UpsertFormulaError {
    BasePropertyIdCollision { id: PropertyId },
}
```

空公式 ID 在构造 `PropertyId` 时已经返回 `IdentifierError`，不能进入 `FormulaDefinition`。公式内容错误通过 `FormulaState::Invalid` 或 `FormulaState::Blocked` 表达。

### remove_formula

```rust
pub struct RemoveFormulaResult {
    pub affected_dependents: Vec<FormulaState>,
}
```

```text
remove existing formula
  => Some(RemoveFormulaResult)

remove unknown id
  => None

remove base-property id
  => None
```

删除已有公式后，所有直接和间接下游公式都会重新分析。失去该依赖的公式通常转为 `Blocked`。返回列表不包含已删除公式，并按确定性 ID 顺序排列。

### 没有批量写入接口

公开接口只有 `upsert_formula` 和 `remove_formula`：

```rust
// Not public API
// fn apply_changes(changes: &[FormulaChange]);
```

Engine 可以在内部批量更新图和缓存，但调用方看不到 `FormulaChange`。多次公开写入是多个独立提交，不提供事务、回滚或 compare-and-swap。

### 缺失依赖可以自动恢复

以下顺序是合法的：

```rust
engine.upsert_formula(FormulaDefinition {
    id: PropertyId::new("A")?,
    source: "prop(\"B\") + 1".into(),
})?;

// A is Blocked: B does not exist.

engine.upsert_formula(FormulaDefinition {
    id: PropertyId::new("B")?,
    source: "41".into(),
})?;

// B is Ready.
// A is reanalyzed automatically and becomes Ready.
```

删除 `B` 后，`A` 自动回到 `Blocked`。因此 Engine 必须保留指向尚不存在 ID 的依赖边。

依赖环也通过状态表示：

```text
A -> B
B -> A

A.status = Blocked
B.status = Blocked
```

引用环中以及依赖该环的公式都不可执行，但这些定义仍保存在 Engine 中。

## 批量求值

### 请求

```rust
pub struct EvaluateInput {
    pub row_ids: Vec<RowId>,
    pub columns: Vec<InputColumn>,
    pub runtime: RuntimeContext,
}

pub struct InputColumn {
    pub id: PropertyId,
    pub data: Column,
}

pub struct RuntimeContext {
    pub evaluated_at_epoch_ms: i64,
    pub timezone_offset_minutes: i32,
}
```

`columns` 是这一批行的基础属性值，不是 Schema、公式定义或公式计算结果。

```rust
let result = engine.evaluate(
    &EvaluateInput {
        row_ids,
        columns: base_property_columns,
        runtime: RuntimeContext {
            evaluated_at_epoch_ms,
            timezone_offset_minutes,
        },
    },
    &[target_a, target_b],
)?;
```

`RuntimeContext` 对一次请求中的全部行、依赖和 target 固定不变。`now()`、`today()` 等能力不得隐式读取求值期间的系统时钟。

`row_ids` 同时用于：

```text
  - 确定批次行数和结果行顺序
  - 为 id() 等按行能力提供身份
  - 让宿主把返回列映射回业务行
```

Engine 不会在结果中重复返回 row IDs。

### targets

`targets` 是必填的独立参数：

```text
targets
  - 必须非空
  - 不得重复
  - 每个 ID 都必须是已注册公式
  - 输入顺序就是 TargetResult 输出顺序
```

Engine 计算每个 target 的完整依赖闭包，但只返回调用方明确请求的 target。

`Invalid` 或 `Blocked` 是已注册公式，因此可以作为 target；它们产生 `TargetResult::Unavailable`。未知 ID、基础属性 ID 或重复 ID 使整个请求失败。

### 必需输入列

对于所有 `Ready` target，Engine 取其依赖闭包需要的基础属性并集：

```text
required columns = union(base properties required by every Ready target closure)
```

输入规则如下：

```text
  - 缺少任一 required column：整个请求失败
  - Schema 中存在但本次不需要的额外列：允许并忽略
  - 未在 Schema 中声明的列：整个请求失败
  - 公式 ID 对应的列：整个请求失败
  - 重复列 ID：整个请求失败
  - 列顺序：不影响语义
  - 每列长度：必须等于 row_ids.len()
  - Bitmap 长度：必须等于 row_ids.len()
  - 列数据类型：必须符合 Schema
```

零行批次合法。此时所有提供的列和 Bitmap 长度也必须为零，结果包含对应的空列。

### Column

下列类型定义描述稳定的公共数据模型，不公开 evaluator 内部的 `KernelColumn`、共享存储或执行 mask：

```rust
pub struct ColumnData<T> {
    pub values: Vec<T>,
    pub validity: Validity,
}

pub enum Validity {
    AllValid,
    AllNull,
    Bitmap(Vec<bool>), // true = valid, false = null
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

pub enum ColumnKind {
    Number,
    String,
    Boolean,
    Date,
    List,
    Union,
    Null,
}
```

```text
ColumnData invariants
  values.len() = batch row count
  Bitmap(bits) => bits.len() = values.len()
  null/error row 的 values 占位值不可读取，具体内容不属于契约
  Date = UTC Unix epoch milliseconds
```

逻辑类型到物理列的选择如下：

```text
Number | Null                 => Column::Number + validity
String | Null                 => Column::String + validity
Boolean | Null                => Column::Boolean + validity
Date | Null                   => Column::Date + validity
List<T> | Null                => Column::List + validity
Number | String | ...         => Column::Union + validity
Null                          => Column::Null { len }
```

因此，顶层 null 不需要 `Value::Null`。`Column::Null` 仍必须携带 `len`，因为没有 `values` 可以表达它与多少输入行对齐。

### 结果

```rust
pub struct EvaluateResult {
    pub targets: Vec<TargetResult>,
}

pub enum TargetResult {
    Evaluated {
        id: PropertyId,
        column: Column,
        errors: Vec<RowError>,
    },
    Unavailable {
        id: PropertyId,
        diagnostics: Vec<FormulaDiagnostic>,
    },
}

pub struct RowError {
    pub row_index: usize,
    pub origin_formula_id: PropertyId,
    pub code: RowErrorCode,
    pub message: String,
}

pub struct RowErrorCode(String);
```

```text
Evaluated
  column.len() = input.row_ids.len()
  普通 null     => validity 为 false，errors 中没有对应错误
  求值错误      => validity 为 false，errors 中存在对应 RowError

Unavailable
  target 是 Invalid 或 Blocked
  不返回占位 Column
  不阻止同一请求中的其他 Ready target 被计算
```

`row_index` 指向输入 `row_ids` 的同一位置。`origin_formula_id` 标识错误实际产生于 target 依赖闭包中的哪条公式；外层 `TargetResult.id` 已经标识正在计算的 target。

同一 target、同一行可以产生多个错误。错误按确定性的求值遍历顺序返回，不按行号重新排序。普通二元运算和 eager builtin 保留所有已执行分支产生的错误；短路逻辑、条件表达式和受控 builtin 不得执行被 mask 排除的分支。

## 请求失败与局部失败

请求契约错误在计算任何 target 前返回一个确定的 `EvaluateRequestError`：

```rust
pub enum EvaluateRequestError {
    EmptyTargets,
    DuplicateTarget { id: PropertyId },
    UnknownTarget { id: PropertyId },
    DuplicateRowId { id: RowId },
    DuplicateColumn { id: PropertyId },
    UnknownColumn { id: PropertyId },
    WrongColumnType {
        id: PropertyId,
        expected: Type,
        actual: ColumnKind,
    },
    WrongColumnLength {
        id: PropertyId,
        expected: usize,
        actual: usize,
    },
    MissingColumn { id: PropertyId },
}
```

一次请求同时违反多条规则时，只返回一个错误。验证顺序固定为：

```text
1. targets 非空、唯一、已注册
2. row_ids 唯一
3. columns ID 唯一且属于 Schema
4. columns 类型和长度正确
5. required columns 全部存在
```

同一阶段内按调用方输入顺序选择第一个错误。缺失列按 Schema 顺序选择第一个错误。

三种失败层级不得混用：

```text
EvaluateRequestError
  整个调用返回 Err
  没有 EvaluateResult

FormulaStatus::Invalid / Blocked
  调用成功
  对应 target 返回 Unavailable

RowError
  调用成功
  对应 target 返回 Evaluated
  只有相关行无值
```

## 顺序、原子性和并发

Rust 方法是同步接口。调用方传入借用数据，Engine 返回拥有数据的结果。`evaluate` 不取得输入列所有权。

第一版不提供：

```text
  - 流式结果
  - evaluate 取消
  - Engine revision
  - 原子多公式事务
  - 并行调用保证
```

浏览器 Adapter 将所有 `FormulaEngine` 调用放入同一 FIFO 队列：

```text
await upsertFormula(A)
await upsertFormula(B)
await evaluate(batch, [A])

=> evaluate 必须看到 A 和 B 均已提交的状态
```

每个操作完成后才开始下一个操作。Promise 的完成顺序与入队顺序一致。后写入的相同 ID 覆盖先写入的定义；由于没有 revision，第一版不检测陈旧写入。

Engine 可以使用私有缓存，但缓存命中、失效方式和是否存在都不属于接口。缓存不得改变结果、错误或顺序。

## Browser Worker Adapter

### 值映射

Rust 名称使用 `snake_case`，TypeScript 使用 `camelCase`。枚举映射为带 `kind` 的 discriminated union。转换必须保留 Rust 接口的全部信息：

```text
Rust                         TypeScript
PropertyId / RowId           string
i64 epoch milliseconds       bigint
usize row index / length     number
Vec<T>                       readonly T[]
Option<T>                    T | null
Result<T, E>::Ok              fulfilled Promise<T>
Result<T, E>::Err             rejected Promise with structured E
UTF-8 Span                   UTF-16 Span
```

TS 调用方仍然拥有传入的数组和缓冲区。第一版不使用会 detach 调用方 `ArrayBuffer` 的 Transferable 输入协议。

### Promise 结果

```text
createFormulaEngine
  invalid PropertyId                   => reject
  SchemaError                         => reject

upsertFormula
  invalid PropertyId                   => reject
  UpsertFormulaError                  => reject
  Invalid / Blocked committed state   => resolve

removeFormula
  invalid PropertyId                   => reject
  unknown or base-property id         => resolve null

evaluate
  EvaluateRequestError                => reject
  Unavailable target                  => resolve
  RowError                            => resolve

any method
  Worker crash / serialization error  => reject
```

### close

浏览器接口只提供 `close()`，不提供 `terminate()`：

```text
Open
  ├─ close() ───────────────> Closing ── Rust value dropped ──> Closed
  └─ Worker failure ────────> Crashed

Closing
  └─ Worker failure ────────> Crashed
```

```text
close invariants
  - 幂等；重复调用返回同一个 Promise
  - close 开始后拒绝新调用
  - 已入队和正在执行的调用继续完成
  - Worker 确认 Rust FormulaEngine 已 drop 后，Adapter 才终止 Worker
  - Worker crash 会拒绝所有 pending 调用和 close Promise
  - Closed 与 Crashed 都是永久状态
```

Rust 端只使用所有权和 `Drop`，不增加对应的 `close` 方法。

## FormulaAnalyzer 不属于本接口

编辑草稿和计算已提交公式是两个不同模块：

```text
FormulaEngine
  输入：FormulaDefinition（已提交）
  负责：身份、依赖图、状态、批量求值

FormulaAnalyzer
  输入：formula source（未提交）
  负责：单表达式分析、帮助、格式化和文本编辑
```

`FormulaAnalyzer` 不得因为格式化或应用文本编辑而隐式调用 `FormulaEngine::upsert_formula`。

下面仅记录后续独立 spec 可以采用的 Exploratory 形状，不构成本篇 Planned 契约：

```rust
pub struct FormulaAnalyzer { /* read-only analysis context */ }

pub struct FormulaDraft {
    pub source: String,
    pub output_type: Option<Type>,
    pub diagnostics: Vec<FormulaDiagnostic>,
}

impl FormulaAnalyzer {
    pub fn analyze(&self, source: &str) -> FormulaDraft;
}
```

这里不需要 formula ID，因为该抽象只分析一个表达式，不判断它在已提交依赖图中的自引用或依赖环。缓存与此无关：第一版可以每次重新分析，未来也可以把私有解析结果附着在 draft 上而不改变接口。

将草稿保存到 Engine 必须是宿主的显式动作：

```rust
let draft = analyzer.analyze(editor_source);

// Discard: do nothing.

// Save:
engine.upsert_formula(FormulaDefinition {
    id: selected_formula_id,
    source: draft.source,
})?;
```

FormulaAnalyzer 的完整结果、编辑操作、上下文更新和 Worker 调度由独立规范定义。

## 设计取舍与复议条件

```text
选择                         当前收益                         需要复议的信号
固定 Schema                  状态和输入契约简单               宿主需要不停机增删基础属性
无 revision                  写入接口小                       出现多个并发写入者或覆盖冲突
单 FIFO Worker               调用顺序可直接推导               长求值明显阻塞交互调用
完整列结果                   错误和对齐语义简单               单批数据无法合理放入内存
稳定公共 Column              Rust 与 TS 共用数据模型          新后端需要不同的零拷贝 ABI
FormulaAnalyzer 独立         草稿不会污染已提交计算状态       编辑器必须预检依赖环
```

这些信号允许扩展接口，但不得静默改变第一版行为。

## 第一版明确不提供的能力

```text
  - 可变 Schema
  - 公式 rename
  - display name 管理
  - 公共依赖图
  - 公共缓存或执行计划
  - revision / optimistic concurrency control
  - applyChanges / FormulaChange
  - evaluate 默认 targets
  - row provider 或求值回调
  - 流式、分页或增量列结果
  - 调用取消
  - FormulaAnalyzer 草稿编辑契约
```

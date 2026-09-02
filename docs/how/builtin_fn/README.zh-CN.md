---
doc_id: how.builtin-fn
title: "Builtin 声明如何变成调用点签名"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Builtin 声明如何变成调用点签名

[English](README.md)

本文面向需要新增 builtin 或修改签名模型的维护者，说明一条 Rust 声明如何变成
语义分析和编辑器服务共用的 catalog metadata 与调用点签名。

`builtin_fn` crate 负责声明 catalog、语义签名类型、参数投影、泛型绑定和返回类型
细化。面向使用者的调用保证、编辑器呈现和按行求值不属于这里，分别由 builtin
specification、IDE guide 与 editor-services specification，以及 evaluator guide 维护。

## Catalog 从 Rust 源码开始

所有生产声明都位于
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs)。每个 category function
包含一次 `builtin_functions!` 调用，`builtin_categories()` 再按 catalog 顺序组合这些
函数。

```text
builtins.rs declarations
        |
        v
builtin_fn_macros: parse, validate, expand
        |
        v
BuiltinCategory -> BuiltinCatalogEntry
        |                    |
        | supported          | #[unsupported]
        v                    v
FunctionSig             metadata only
        |
        v
resolve_call_signature -> ResolvedFunctionSig
```

上图止于 AST inference、IDE presentation 和 evaluator dispatch 之前。这些消费者使用
生成后的模型，但各自维护自身行为。

`builtin_categories()` 返回全部声明，包括带有 `#[unsupported]` 的条目。
`builtins_functions()` 保持相同顺序，只留下具有 `FunctionSig` 的条目。因此，完整
inventory 只有一个编写来源：`builtin_fn/src/builtins.rs`。本文不会复制或渲染完整的
名称与签名清单。

`BuiltinSigParser` 可以为需要字符串格式的调用方解析独立签名，但它不是生产 registry
的数据来源。

## 声明 DSL 保存结构，而不是格式化文本

一段 category declaration 描述函数名、泛型、参数布局、返回类型、可选的支持状态，
以及 unsupported 条目的说明。例如：

```rust,ignore
builtin_functions! {
    category: General;

    ifs<T: Variant>(
        repeat(min = 1) {
            condition: boolean,
            value: () -> T,
        },
        else: () -> T,
    ) -> T;
}
```

DSL 支持：

- `number`、`string`、`boolean`、`date`、`null` 和 `any` 这些 primitive type；
- 已声明的 generic name、用 `|` 表示的 union、带有 `[]` suffix 的 list type、
  lambda type、grouped type 和 `Ident<T>` binder type；
- 写作 `name?: Type` 的 optional fixed parameter；
- 至多一个显式的 `repeat(min = N) { ... }` group；
- 当普通泛型替换无法表达返回类型时使用的 `#[resolver(path)]`；
- catalog-only declaration 使用的 `#[unsupported]`，并且必须附带至少一行 doc comment。

macro 会把一个 supported declaration 同时降低为 `BuiltinCatalogEntry` 和
`FunctionSig`。规范签名与 detail string 都由解析后的 shape 生成，声明不能另设 detail
override。Unsupported declaration 仍会生成有序的 catalog metadata，但
`implementation` 字段为 `None`。

解析、局部校验和展开由
[`builtin_fn_macros`](../builtin_fn_macros/README.zh-CN.md) 负责；`builtin_fn` 负责展开
结果中的数据结构。

## `ParamShape` 明确表达参数位置

每个 `FunctionSig` 都包含
`ParamShape { head, repeat, tail, repeat_min_groups }`。参数在 DSL 中的位置决定其区域：

- `repeat` 之前的普通参数进入 `head`；
- block 中的参数组成一个 repeating group；
- block 之后的普通参数进入 `tail`。

该模型直接支持五种布局，无需根据名称猜测 repetition：

| 布局 | 代表声明 |
| --- | --- |
| fixed only | `flat` 或 `substring` |
| repeat only | `concat` |
| head + repeat | `splice` |
| repeat + tail | `ifs` |
| head + repeat + tail | synthetic `caseOf` contract fixture |

对于 repeating signature，精确匹配的调用满足：

```text
total = head.len + repeat.len * groups + tail_used
groups >= repeat_min_groups
```

`tail_used` 表示这次 projection 选中的 tail slot 数量。生产 DSL 要求 repeat 后的所有
tail parameter 都是 required，因此其中 `tail_used == tail.len`。如果直接构造带 optional
tail slot 的 `ParamShape`，则可能选用更少的 slot。

`min` 计算完整 group 的数量，而不是单个参数的数量；它必须是不带 suffix 的非负整数
literal。Repeat group 不能为空，成员不能 optional，并且一个声明只能出现一次
`repeat`。存在 repeat 时，固定的 `head` 和 `tail` 参数也必须 required，否则 repeating
group 与 tail 的边界无法唯一确定。

没有 repeat 时，optional fixed parameter 只能构成一个连续 suffix；macro 会拒绝在
optional parameter 之后再出现 required parameter。Repeat member 使用 `condition` 这类
logical base name，带数字或旧式 `N` suffix 的名称会被拒绝。消费者根据
`ResolvedParamSlot::repeat_group` 生成 group number，而不是把编号写入声明。

降低结果和布局矩阵由
[`builtin_fn/tests/macro_dsl.rs`](../../../builtin_fn/tests/macro_dsl.rs) 与
[`builtin_fn/tests/equivalence.rs`](../../../builtin_fn/tests/equivalence.rs) 覆盖。共享的
shape projection 位于
[`builtin_fn/src/param_shape.rs`](../../../builtin_fn/src/param_shape.rs) 和
[`builtin_fn/src/resolution.rs`](../../../builtin_fn/src/resolution.rs)。

## 类型模型保留参数之间的关系

Generic 会按照声明顺序获得确定的 `GenericId`。两种 binding kind 决定 observation
如何累积：

| Kind | 绑定行为 |
| --- | --- |
| `Plain` | Unknown observation 不参与绑定；不同的 concrete observation 合成确定性 union。 |
| `Variant` | Concrete observation 合成确定性 union；只要出现 unknown，整个 binding 就成为 unknown。 |

省略 kind 等同于 `Plain`。Union normalization 会递归展开 nested union、去重，并按稳定
的 type order 排列。DSL 中的 `any` 会降低为隐藏的 `Plain` generic，从而继续使用普通
binding 机制。

Lambda type 同时保留 parameter type 与 binding origin。`current` 会降低为
`LambdaParam::Current`；其他 lambda parameter name 会降低为
`LambdaParam::ParamRef(name)`。当前声明约定用这个名称引用另一个参数，但 macro lowering
不会校验目标是否存在。`let` 的声明遵循这一约定：

```rust,ignore
let<T, U>(ident: Ident<T>, value: T, body: (ident: T) -> U) -> U;
```

`Ident<T>` 标记携带 identifier 的参数，`body` 则使用 `ident` 作为 reference name。
Analyzer 进行 staged lambda inference 时，会在 resolved argument 中查找同名 parameter，
找到 identifier expression 就采用其实际 spelling，否则回退到 reference name 本身。
查找与回退逻辑位于
[`analyzer/src/analysis/infer.rs`](../../../analyzer/src/analysis/infer.rs)；`builtin_fn` 只保存
类型与未经校验的 reference name。

相关模型位于
[`builtin_fn/src/types.rs`](../../../builtin_fn/src/types.rs)、
[`builtin_fn/src/signature.rs`](../../../builtin_fn/src/signature.rs) 和
[`builtin_fn/src/type_hints.rs`](../../../builtin_fn/src/type_hints.rs)。

## 调用点解析返回一份无隐藏状态的快照

`resolve_call_signature()` 接收 `FunctionSig` 和按语义顺序排列的参数。每个参数有两种
observation：

- `Empty`：语法上已有 argument slot，但没有 expression；
- `Typed(Ty)`：已有 expression；推断不出类型时也会明确写作 `Ty::Unknown`。

Input slice 的末尾表示这个 argument slot 尚不存在。区分这些状态后，不完整的编辑器
输入和完整的语义输入就能使用同一个 resolver。

解析过程依次：

1. 将 observed count 投影到 fixed 或 repeating shape；
2. 把投影位置映射到 `ParamRef::Head`、`Repeat` 或 `Tail`；
3. 根据 typed observation 绑定 generic；
4. 实例化声明中的 parameter type 和 return type；
5. 如果存在 custom return resolver，则调用它；
6. 比较 observation 与实例化后的 parameter type；
7. 一并返回 validity、projection、逐参数状态和 return type。

参数数量符合 shape 时得到 `ShapeValidity::Valid`，其他数量则产生具体的
`CallShapeError`。Fixed shape 始终投影全部已声明的 fixed slot，无论某个 slot 是否已有
observation。Repeating shape 在数量合法时采用精确 split，数量不合法时采用大于等于
observed count 的最小 completable count。没有对应 projected parameter 的多余参数标为
`Unmapped`；empty 与 unknown argument 标为 `Indeterminate`，避免把 partial call 误报为
类型不匹配。

Projection 保持为语义数据。每个 `ResolvedParamSlot` 包含 logical `ParamRef`、可选的
one-based repeat-group number、可选的 source argument index，以及实例化后的 expected
type；它不保存 rendered label 或 active editor parameter。重复调用 resolver 不会复用
hidden state。

对于 repeating shape，`resolve_repeat_tail_used()` 会寻找一个 split，使 middle 恰好由完整
group 组成，并满足 `repeat_min_groups`。它会保留 required tail prefix；如果外部直接构造的
shape 存在多个可行 split，则优先采用最大的 tail count。生产声明不允许 repeat 搭配
optional tail，因此其精确 split 唯一。Observed count 没有精确 split 时，projection 会选择
大于等于它的最小可行 count，并且只用这个 count 构造 completion slot，不会为源码中不
存在的参数虚构 observation。

[`builtin_fn/tests/resolution.rs`](../../../builtin_fn/tests/resolution.rs) 覆盖精确、不完整
和非法 shape、generic binding、staged observation 以及五种布局。

`FunctionSig` 和 `ParamShape` 不包含 postfix-capability flag，只公开 signature shape。
某个签名是否支持 postfix syntax，由 Analyzer 负责判断。

### Custom resolver 只能细化返回类型

`#[resolver(path)]` 关联一个 `SigResolver`，输入为：

```rust,ignore
pub struct ResolverInput<'a> {
    pub arguments: &'a [ArgumentObservation],
    pub default_return_ty: &'a Ty,
}
```

普通 shape projection 与 generic substitution 先执行，resolver 再给出最终
`return_ty`。它不能改变 parameter mapping、expected argument type、validity、catalog
metadata 或 generic declaration。Partial snapshot 也会调用 resolver，因此在 observation
为空或 unknown 时，resolver 必须回退到 `default_return_ty`。

`flat` 是生产代码中的例子。它递归收集第一个 list argument 中所有非 list leaf，并将
它们规范化为结果的 element union。实现与针对性测试分别位于
[`builtin_fn/src/builtins.rs`](../../../builtin_fn/src/builtins.rs) 和
[`builtin_fn/tests/resolution.rs`](../../../builtin_fn/tests/resolution.rs)。

## 每层只校验自己看得见的不变量

没有一个校验点能够看见全部不变量：

- `builtin_fn_macros` 校验一次 category invocation 内的 syntax、attribute、generic、
  supported type、parameter name、optional placement 和 repeat layout。只要存在 local
  error，就不会生成 partial `BuiltinCategory`。
- `ParamShape::new` 与 `FunctionSig::new_builtin` 保护由生产 DSL 之外的调用方直接构造的
  语义结构。违反这些 programmer invariant 会 panic，不会变成可恢复的 call result。
- 普通 Rust type checking 检查 resolver path 是否存在，并验证其类型为 `SigResolver`。
- 遍历 `builtin_categories()` 的测试负责单次 macro invocation 看不到的 invariant，包括
  cross-category name uniqueness、category order、support status 和 registry inclusion。
- Analyzer、IDE 和 evaluator 的测试只覆盖各自新增的行为，不重新定义 declaration model。

主要的 whole-catalog check 位于
[`builtin_fn/tests/builtins.rs`](../../../builtin_fn/tests/builtins.rs)。DSL 的 compile-pass
与 compile-fail coverage 位于
[`builtin_fn/tests/ui/`](../../../builtin_fn/tests/ui/)，而 procedural macro 自身的 recovery
和 diagnostic boundary 由
[`builtin_fn_macros` guide](../builtin_fn_macros/README.zh-CN.md) 说明。

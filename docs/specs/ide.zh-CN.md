---
doc_id: specs.ide
title: "FormulaDraft：编辑器能力"
language: zh-CN
source_language: zh-CN
counterpart: ./ide.md
implementation_status: planned
document_status: draft
translation_status: synced
last_verified: 2026-09-23
---

# FormulaDraft：编辑器能力

[English](ide.md) · [Specification index](README.zh-CN.md)

> Planned：FormulaDraft 尚未实现。补全、参数提示和文本编辑沿用末节的 Current IDE 行为。

## FormulaDraft

```rust spec=formula_draft.h.rs
pub use analyzer::{Span, TextEdit, Token};
pub use ide::{
    CompletionConfig, CompletionItem, CompletionKind, CompletionResult,
    DisplaySegment, SignatureHelp, SignatureItem,
};

/// 共享借用 Engine 进行分析；同 ID 的依赖关系按正在编辑的定义计算。
/// 同一 Engine 可同时创建多个 Draft；保存前须结束所有 Draft 的借用。
#[spec::private_fields]
pub struct FormulaDraft<'engine> {}

#[spec::header]
impl FormulaDraft<'_> {
    pub fn state(&self) -> &FormulaDraftState;

    /// 同时查询 completion、postfix completion 和 signature help。
    /// 会形成依赖环的候选仍返回，但标为 disabled。
    /// CompletionConfig::default() 的 preferred_limit 为 5；0 禁用 preferred_indices。
    pub fn help(&self, cursor: TextOffset, config: CompletionConfig) -> CursorHelp;

    /// 返回 diagnostic 附带的修改建议；未知或非当前版本的 ID 返回空列表。
    pub fn quick_fixes(&self, diagnostic_id: &DiagnosticId) -> Vec<QuickFix>;

    /// 返回整段 expression 的替换 edit；lexer/parser diagnostic 阻止格式化，语义错误不阻止。
    pub fn format_edits(&self) -> Result<FormulaEdit, FormatError>;

    /// 原子更新 expression，返回前更新 output_type、diagnostics 和 tokens。
    ///
    /// - allow: 无效 expression；通过 state() 查看 diagnostics。
    /// - error: Edits 的版本、区间、cursor 或非重叠校验失败，Draft 不变。
    pub fn update_expression(&mut self, update: ExpressionUpdate)
        -> Result<UpdateExpressionResult, UpdateExpressionError>;

    /// 不自动提交；保存时将返回的定义交给 FormulaEngine::upsert。
    pub fn into_definition(self) -> FormulaDefinition;
}
```

[FormulaEngine](formula-engine.zh-CN.md) 定义共享类型、`create_draft` 入口及编辑、保存示例。
[Token 与 Span](formula-language.zh-CN.md#词法结构) 由 language spec 定义。
Completion 沿用 [IDE 返回结构](../../ide/src/lib.rs)与[候选类型](../../ide/src/completion/mod.rs)；
SignatureHelp 沿用[签名结构](../../ide/src/signature/mod.rs)和[显示片段](../../ide/src/display.rs)。

```rust spec=formula_draft.h.rs
/// UTF-8 字节偏移。
#[derive(derive_more::From)]
pub struct TextOffset(pub usize);

#[derive(Clone, Copy)]
pub struct DraftVersion(pub u64);

pub struct FormulaDraftState {
    /// Replace 改变文本或 Edits 成功时加 1。
    pub version: DraftVersion,
    pub definition: FormulaDefinition,
    /// 推断不出明确类型时为 None，不返回 Unknown/Null。
    pub output_type: Option<ValueType>,
    /// 语法与语义诊断，不依赖 cursor；包含直接和间接自引用问题。
    pub diagnostics: Vec<ExpressionDiagnostic>,
    /// 当前 definition.expression 的词法 tokens，保留注释、换行和 Eof。
    pub tokens: Vec<Token>,
}
pub struct ExpressionDiagnostic {
    pub id: DiagnosticId,
    /// 当前 Draft 的 expression 中的位置。
    pub span: Span,
    pub message: String,
}
pub struct DiagnosticId(pub String);

pub struct CursorHelp {
    /// 应用本次补全的 edits 时，使用此版本作为 FormulaEdit.base_version。
    pub base_version: DraftVersion,
    pub completion: CompletionResult,
    pub signature_help: Option<SignatureHelp>,
}
pub struct FormulaEdit {
    /// 必须等于当前 state.version，防止旧 edits 应用到已修改的 expression。
    pub base_version: DraftVersion,
    /// 所有区间均基于修改前的 expression，且不得重叠。
    pub edits: Vec<TextEdit>,
}
pub enum ExpressionUpdate {
    /// 覆盖当前 expression，不要求 base_version。
    Replace(String),
    Edits {
        edit: FormulaEdit,
        /// 修改前的 expression 坐标，按 edits 重定位。
        cursor: TextOffset,
    },
}
pub struct QuickFix {
    pub title: String,
    /// version 绑定到产生 diagnostic 时的 Draft。
    pub edit: FormulaEdit,
}
pub struct UpdateExpressionResult {
    pub state: FormulaDraftState,
    /// Replace 返回新 expression.len()，空串为 0；Edits 返回重定位后的坐标。
    pub cursor: TextOffset,
}

/// expression 存在 lexer/parser diagnostic，无法格式化。
pub struct FormatError;

pub enum UpdateExpressionError {
    VersionMismatch,
    /// 越界或不在 UTF-8 字符边界上。
    InvalidCursor,
    /// 区间倒置、越界或端点不在 UTF-8 字符边界上。
    InvalidEditRange,
    OverlappingEdits,
}
```

补全的 primary/additional edits 合为一个 `FormulaEdit`，交给 `update_expression()` 应用。
补全的光标使用 `CompletionItem.cursor`，缺省时位于 primary edit 插入文本之后；
再计入 primary edit 之前的 additional edits，得到编辑后的坐标供编辑器定位。

## Current：IDE 行为

当前服务按请求处理 source，没有持久化 FormulaDraft。此节拥有编辑行为；
JS 序列化、坐标校验和异常由 [WASM API](wasm-api.zh-CN.md) 定义。

```text
diagnostics
  顺序 = parser ++ lexer ++ semantic；不做全局排序/去重，单个阶段可合并同 span。
  parser 可附带 recovery actions；lexer/semantic 当前不附带；从不自动应用。
  message 是人类可读文本，不是稳定错误码。

completion：候选集合
  expression 起点 / 空参数位置                 → properties、not/true/false、支持的函数
  identifier/not/true/false 内部或识别出的前缀末尾 → 同上
  完整 identifier/literal/右括号后，且非前缀     → == != >= > <= < + - * /、可 postfix 的函数
  receiver.                                  → receiver 类型兼容的 postfix 函数
  receiver 类型 unknown                      → 全部 postfix 候选
  string 内部                                → 无补全；仍可能有 signature help
  全文只有横向空白                            → cursor=0 才给起点候选；换行不走此特例
  % ^ && || 虽是语言运算符，但不在 after-atom 候选中。

completion：匹配与编辑
  前缀识别用小写前缀比较，却以原拼写判断完整匹配；完整混合大小写名称仍可能走前缀路径。
  前缀补全替换整个 identifier；普通插入使用 cursor 处空区间。
  函数插入括号、cursor 在括号内；not/true/false 带尾随空格；property 插入 prop("Name")。
  property 的 cursor 在调用后；名称直接插入，不转义引号/反斜杠，可能产生无效 source。
  disabled property 保留原因，但无 primary edit/cursor，且不进入 preferred。

completion：排序与 preferred
  已知具体参数类型只重排、不过滤；unknown/泛型/无参数映射时跳过此步骤。
  同 kind 连续分桶：enabled 优先，再按类型匹配；桶按最佳匹配排序，同分用固定 kind 顺序。
  后续 query 排序可改变以上顺序：仅处理非空、规范化后含 ASCII 字母/数字/_/空白的查询；
  比较忽略 ASCII 大小写和 _：exact > contains > ordered subsequence。
  同分按紧凑程度、kind、原顺序确定；函数的 () 和 postfix 的前导 . 不参与查询匹配。
  普通 expression 保留不匹配项；after-dot 过滤不匹配项。
  含非 ASCII 非空白字符 → 跳过 query 排序，保留此前顺序，preferred=[]。
  preferred_indices 指向最终列表，按最终顺序，最多 preferred_limit 个 enabled 匹配函数/property。
  无查询、limit=0 或无匹配 → preferred=[]。

signature help
  只看最内层未闭合 (，前面须是已知函数，cursor 在 ( 之后。
  内层 grouping/未知函数会抑制外层回退；开括号前/离开调用后无 help；缺右括号仍可用。
  返回单个 signature，active_signature=0；参数推断影响显示，unknown/泛型可保留。
  postfix receiver 单独处理，不占显示参数索引。
  顶层逗号决定 active_parameter；忽略嵌套逗号，空参数仍选槽位，repeat 参数投影到显示槽位。
  无映射 → 最后一个显示参数；零参数 → 0。这个回退目前没有专门的回归测试。

format
  任何 lexer/parser diagnostic → 失败；semantic diagnostic 不阻止格式化。
  全量、确定性；已覆盖语法保持幂等。缩进 2 空格，二元/三元/逗号采用常规空格，结尾一个换行。
  保留附着的注释；允许内联且缩进+渲染 UTF-8 字节数 <= 80 才内联，否则多行。
  原子表达式不受该宽度判断约束；不提供格式选项。
  Current 返回完整 source，并按整文替换 edit 重定位 cursor：内部位置通常归零，文末随新文末移动。
  Planned format_edits 返回 edits；不能将 Current 的返回结构直接当作它的结构。

apply edits
  所有 range 基于原 source；按 (start,end) 稳定排序，再逆序应用。
  非空区间不可重叠；允许相邻区间、同位置零宽插入，后者保留调用方顺序。
  区间无效/重叠 → 整次失败。
  edit.end <= cursor       → 按长度差平移
  start < cursor < end     → 落在 replacement 起点
  cursor == start          → 留在 replacement 前
  edit 在 cursor 后         → 不影响 cursor
  因此 cursor 处的插入使它位于插入文本之后；replacement 起点则仍在替换文本之前。
```

实现锚点：[help](../../ide/src/lib.rs)、[signature](../../ide/src/signature/mod.rs)、
[format](../../ide/src/format.rs)、[edit](../../ide/src/edit.rs)、
[字节编辑](../../ide/src/text_edit.rs)。候选排序不定义 UI 的展示、自动提交或选中策略。

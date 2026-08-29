# Project Glossary / 项目术语表

This shared bilingual reference defines project concepts once. Architecture documents use
the English and Chinese terms below consistently while preserving code identifiers exactly.

本双语参考统一定义项目概念。架构文档应稳定使用下列中英文术语，并保持代码标识符原样不变。

| English term | 中文术语 | Code identifier or example | Definition / 定义 | Avoid / 避免使用 |
| --- | --- | --- | --- | --- |
| formula | 公式 | `prepare_formula` | Formula source and its analyzed or prepared representation; not a persisted formula record with an ID. / 公式源码及其经过分析或准备后的表示；不是带 ID 的持久化公式记录。 | equation / 方程 |
| source | 源码 | `source: &str` | The UTF-8 formula text supplied to the analyzer. / 提供给 analyzer 的 UTF-8 公式文本。 | content when formula text is intended / 指代公式文本时使用“内容” |
| token | token | `Token` | A syntax unit emitted by the lexer. / lexer 产生的语法单元。 | word / 单词 |
| trivia | 非语义 token | comments, newlines | Tokens retained for source fidelity but ignored by semantic analysis. / 为保持源码保真而保留、但不参与语义分析的 token。 | whitespace when comments are also included / 包含注释时称为“空白” |
| diagnostic | 诊断 | `Diagnostic` | A syntax or semantic problem tied to a source span. / 关联到源码 span 的语法或语义问题。 | exception / 异常 |
| code action | 代码操作 | `CodeAction` | A diagnostic-attached edit that can implement a quick fix. / 附着在诊断上、可用于快速修复的编辑操作。 | fix when no edit is provided / 未提供编辑时称为“修复” |
| span | span | `Span` | A half-open source range `[start, end)`, measured in the coordinate system named by its interface. / 半开源码区间 `[start, end)`，计量单位由所在接口规定。 | interval without coordinates / 未说明坐标单位的“区间” |
| semantic analysis | 语义分析 | `SemanticMap` | Type inference and call validation performed after parsing. / 语法分析后执行的类型推断与调用校验。 | parsing / 语法分析 |
| execution plan | 执行计划 | `ExecPlan` | The evaluator-owned IR lowered from an analyzed expression. / evaluator 从已分析表达式降级得到并持有的 IR。 | AST |
| row batch | 行批次 | `RowBatch` | The ordered rows evaluated together under one prepared formula and runtime snapshot. / 在同一已准备公式和运行时快照下共同求值的有序行集合。 | table / 表 |
| execution mask | 执行掩码 | `Mask` | The rows active for a particular control-flow step. / 某个控制流步骤中需要执行的行集合。 | null bitmap / null 位图 |
| validity | 有效性 | `Validity` | Whether each successful row carries a non-null value. / 每个成功行是否包含非 null 值。 | success state / 成功状态 |
| row success | 行成功状态 | `ok` | Whether evaluation succeeded for each row; independent of `Mask` and `Validity`. / 每一行是否求值成功；独立于 `Mask` 和 `Validity`。 | validity / 有效性 |
| input slot | 输入槽位 | `InputSlot` | A dense identity local to one prepared input layout. / 仅在一个已准备输入布局中有效的稠密标识。 | property ID / 属性 ID |
| required-column manifest | 必需列清单 | `RequiredColumn[]` | The complete, deduplicated property dependencies callers must prepare before evaluation. / 调用方在求值前必须准备的、完整且去重的属性依赖。 | runtime lookup list / 运行时查找列表 |

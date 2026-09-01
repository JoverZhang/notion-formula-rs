# 文档重构 Review Goal

这份计划回答一个问题：完成 notion-formula-rs 的文档重构后，reviewer 应该看到什么，才能确认新的文档结构已经建立，并且旧结构没有继续充当第二个事实来源。

本文是本次交付的临时 review goal，不是系统当前行为的说明，也不授权立即修改 `docs/intent/` 或 `docs/specs/`。实施这些 human-controlled 文档时，agent 仍须获得用户对相应工作的明确授权。

## 交付结果

重构完成后，读者沿同一条路径逐渐深入，但三个层级维护不同类型的事实：

- `intent/` 简短说明为什么建设这个系统，以及目标、非目标、边界和重新评估条件。
- `specs/` 从受支持使用者的角度说明系统当前保证什么。
- `how/` 说明当前 Rust 实现如何兑现这些保证。

层级之间使用链接，不换一种说法重复同一事实。查询资料、维护方法和历史记录保留在正交目录中。

## 目标结构

最终目录以此结构为基准。Specs 的具体拆分可在契约审计后细化，但不得改为按 crate 一一对应。

```text
README.md
README.zh-CN.md
AGENTS.md
CLAUDE.md
DOCUMENTATION.md
DOCUMENTATION.zh-CN.md
DOCUMENTATION_RESTRUCTURE_PLAN.md
GLOSSARY.md

docs/
├── manifest.toml
├── README.md
├── README.zh-CN.md
├── intent/
│   ├── README.md
│   └── README.zh-CN.md
├── specs/
│   ├── README.md
│   ├── README.zh-CN.md
│   ├── formula-language.*
│   ├── formula-references.*
│   ├── builtin-functions/
│   │   └── README.*
│   ├── editor-services.*
│   └── wasm-api.*
├── how/
│   ├── README.md
│   ├── README.zh-CN.md
│   ├── analyzer/
│   │   └── README.*
│   ├── analyzer_wasm/
│   │   └── README.*
│   ├── builtin_fn/
│   │   └── README.*
│   ├── builtin_fn_macros/
│   │   └── README.*
│   ├── evaluator/
│   │   └── README.*
│   ├── ide/
│   │   └── README.*
│   └── examples/
│       └── vite/
│           └── README.*
├── contributing/
│   ├── testing.*
│   ├── coding.*
│   └── changelogs.*
├── changelogs/
│   └── <entries and counterparts>
├── _templates/
│   └── changelog-entry.*
└── assets/
```

`.*` 表示相邻的英文 `.md` 与简体中文 `.zh-CN.md`。它不表示把两种语言交错写在一个文件里。`contributing/coding.*` 是否保留，由 Contributing 小节定义的审计决定。

### 三个阅读入口

- 根 `README.md` 是项目落地页，只保留项目简介、最短使用入口和文档链接。
- `docs/README.md` 是读者导航，只回答不同读者从哪里开始。
- `docs/intent/README.md` 是项目设计意图的唯一权威位置。

### Intent

初始 intent 只有一篇简短的 `README`。它回答系统解决什么问题、服务谁、明确不做什么、为什么采用当前边界，以及什么变化会触发重新评估。

Intent 不按 crate 建文档，也不描述接口或实现过程。

### Specs

Specs 是 living specifications：直接维护系统当前有效的使用者契约，不记录历史演进，也不描述尚未实现的目标行为。

当前受支持的使用者包括：

- 编写和使用公式的人；
- 通过 WASM/JavaScript 边界集成分析和编辑器能力的人。

Rust crate consumer 暂不属于稳定 API 使用者。Rust 中的 `pub` 不自动形成 specification。只有显式承诺给使用者或集成方的行为、schema、错误、顺序、兼容性和失败边界才进入 specs。

第一轮契约审计至少覆盖以下主题：

- 公式语言和可观察的求值语义；
- formula ID、formula name、引用、重命名、缺失和歧义行为；
- builtin 调用规则；
- 诊断、补全、签名帮助和格式化的可观察行为；
- WASM 请求、响应、配置、offset 单位和错误边界。

这些主题可以在审计后合并或拆分。验收标准是每项使用者契约只有一个明确的权威文档，而不是文件名与本计划完全一致。

Specs PR 必须在 PR 描述或 review record 中维护一次性的 contract inventory，不把核验过程写进长期正文：

| Surface or claim | Evidence | Spec owner or explicit exclusion |
|---|---|---|
| `<user-visible behavior>` | `<code, test, schema, or runtime observation>` | `<spec path or exclusion reason>` |

Inventory 至少从 WASM exports、公式语法与运算符、Rust builtin declarations、IDE 输出、formula reference 行为以及对外 evaluator 输入和结果边界逐项展开。只有每个发现的使用者 surface 都被分配给一个 spec，或以明确理由排除，契约审计才算完成。

### How

`how/README.md` 用一条简短的端到端路径说明各阶段由哪个 crate 负责、跨 crate 数据在哪里转换，并链接到各 crate 文档。

具体实现文档准确对应源码目录名。每个 crate 默认只保留一篇 `README`；只有一个机制确实回答独立的源码阅读问题时，才增加更深的文档。

内部 crate API、IR、trait、算法、数据结构、调试入口和跨 crate 协作都属于 how，不属于 specs。跨 crate 事实优先由真正拥有它的 crate 维护，其他文档链接到该位置。确实没有单一 owner 的系统级实现机制才由 `how/README.md` 维护。

### Contributing 与 glossary

`docs/contributing/` 说明贡献者如何正确修改和验证项目，维护 testing、project-specific coding 和 changelog writing 规范。它不描述产品契约或实现机制。

实施前必须审计仓库中已经生效的 coding rules，包括 `AGENTS.md`、Cargo workspace lints、rustfmt 或 Clippy 配置、`justfile` 以及现有 contributor guidance。PR 或 review record 必须列出需要保留的项目特有规则及其权威来源，或者明确记录“不需要独立 coding guide”的结论。只有前一种结果才建立 `contributing/coding.*`；不得用通用 Rust 建议或对工具配置的逐项转述填充它。

本次重构不建立 `operations/`。只有项目以后出现部署、监控、故障处理或发布操作，并确实需要 runbook 时，才另行决定是否增加该目录。

项目只有一份规范术语表，因此使用根目录 `GLOSSARY.md`，不为单个文件建立 `reference/` 目录。它由 `docs/README.*` 链接，供整个仓库共同引用。

## 文档权限与 code review

`DOCUMENTATION.*`、`docs/intent/` 和 `docs/specs/` 是 human-controlled 区域：

- human 决定意图、契约、边界和取舍；
- agent 只有在用户明确要求相应修改时才能编辑；
- agent 因代码变更发现这些文档需要调整时，必须先询问用户；
- 未获授权的必要修改是 code review 阻塞问题；
- 翻译授权只允许保持语义的翻译，不允许借机改变技术结论。

授权根据用户实际命令判断，不要求 PR 关键字、固定 prompt 或额外证明字段。

`how/` 可以在相关实现任务范围内由 agent 同步维护。Code review 使用以下触发规则：

- 目标、非目标或系统边界变化时更新 intent；
- 外部行为、契约、状态、错误或兼容性变化时更新 specs；
- crate 结构、关键算法、数据流或调试入口发生实质变化时更新 how；
- 已有文档中的事实没有变化时不做形式性修改。

Front matter 中不使用 `owners` 控制编辑权限。目录决定内容层级，`DOCUMENTATION.md` 决定权限，用户命令决定本次授权。

## DOCUMENTATION.md

根目录的 `DOCUMENTATION.md` 与 `DOCUMENTATION.zh-CN.md` 是唯一文档维护标准。两份文件都保持精简，只维护：

1. `intent/`、`specs/`、`how/`、`contributing/`、changelogs 和 glossary 的职责；
2. human-controlled 与 agent-maintainable 的权限边界；
3. 代码变更触发哪一层文档更新；
4. 双语编辑和翻译状态；
5. human writing 标准。

Human writing 至少要求：

- 从读者问题出发，使用直接、具体的句子；
- 一个事实只维护在一个权威位置，其他位置使用链接；
- 根据代码、测试、schema 或明确的人类决策陈述事实；
- 不从代码结构反推并虚构设计意图；
- 将少量源码和测试入口放在相关结论附近，不设置统一的“核验地图”；
- 将本次如何核验记录在 PR，而不是长期文章中；
- 中英文保持相同技术语义，但分别写成自然的中文和英文；
- 删除无助于回答核心问题的模板化章节、空泛总结和历史沉积。

`AGENTS.md` 保持很短：它要求文档工作先读 `DOCUMENTATION.md`，并标明 `.agent/` 是 legacy。不得读取、迁移或修改 `.agent/` 中的任何内容。

## 双语策略

除明确例外外，正式 Markdown 最终都有相邻的英文和简体中文版本。

- 每轮编辑根据任务选择一种 source language，不默认中文或英文先行。
- 先核验并完成 source language，再单独翻译 counterpart 是合法工作流。
- 翻译保持范围、保证、限制、确定程度、错误行为、示例和生命周期状态一致。
- `last_verified` 只在技术事实重新核验后更新；移动文件、润色或翻译本身不更新日期。

翻译状态只有三种：

- `synced`：两个版本存在并且语义同步；
- `pending`：counterpart 尚未创建；
- `needs-update`：两个版本都存在，但其中一个已经落后。

`pending` 和 `needs-update` 是中间 PR 可接受的可见翻译债务，不让 `just docs-check` 失败。结构错误、无效 metadata、错误配对和断链仍然失败。整个 review goal 完成时，除明确例外外，translation debt 必须清零。

明确例外包括：

- 根 `GLOSSARY.md` 只维护规范英文术语；
- crate 和 example 根目录的 README 是中性跳转文件；
- `AGENTS.md`、`CLAUDE.md` 和本 review goal 是控制文件；
- 图片、schema 和其他无自然语言资产由两种语言共享；
- `.agent/` 是 ignored legacy。

## Manifest 与 Markdown 分类

`docs/manifest.toml` 是双语覆盖范围的机器可读来源。它只记录路径类别，不重复文档 front matter 中的 `doc_id`、source language、counterpart 或翻译状态。

预期结构如下：

```toml
bilingual_files = [
  "README.md",
  "DOCUMENTATION.md",
]

bilingual_directories = [
  "docs",
]

english_only_files = [
  "GLOSSARY.md",
]

neutral_redirect_files = [
  "analyzer/README.md",
  "analyzer_wasm/README.md",
  "builtin_fn/README.md",
  "builtin_fn_macros/README.md",
  "evaluator/README.md",
  "ide/README.md",
  "examples/vite/README.md",
]

control_files = [
  "AGENTS.md",
  "CLAUDE.md",
  "DOCUMENTATION_RESTRUCTURE_PLAN.md",
]

ignored_directories = [
  ".agent",
]
```

`bilingual_files` 使用无 `.zh-CN` 后缀的基准路径，同时覆盖其相邻 counterpart。`bilingual_directories` 覆盖目录中的所有 Markdown；exact exception 优先于目录默认值。

迁移期间允许 manifest 临时增加一个逐文件的 `legacy_files` 列表：

```toml
legacy_files = [
  "docs/design/demo-vite.md",
  "docs/signature-help.md",
]
```

它只用于登记尚未迁入新结构的现有文件。Checker 把它们列为 migration debt，并继续检查本地链接。每迁移一个文件就删除一个条目；不得用目录规则或通配符扩大 legacy 范围，也不得把新文件加入该列表。最终交付时 `legacy_files` 必须为空并从 manifest 删除。

Checker 按以下顺序分类：

1. 跳过工具自身的环境目录以及 manifest 中的 ignored directories。
2. 发现仓库中的全部 Markdown。
3. 先匹配 English-only、neutral redirect、control file 和临时 legacy file 这些 exact exceptions。
4. 再匹配 bilingual files 和 bilingual directories。
5. 未匹配任何类别的 Markdown 是错误。
6. 同一路径出现在多个 exact exception 类别中也是错误。

类别决定验证规则：

- bilingual document 使用完整 metadata，并按 `.md` / `.zh-CN.md` 命名和 `doc_id` 配对；
- `synced` 和 `needs-update` 必须存在两个版本；
- `pending` 必须声明预期 counterpart，并允许它尚不存在；
- English-only glossary 保留身份、语言、生命周期和核验日期，不要求 counterpart 或 translation status；
- neutral redirect 和 control file 不使用 front matter；
- 临时 legacy file 不要求补写即将废弃的 metadata，但始终形成 migration debt；
- 除 ignored legacy 外，所有类别都继续接受本地 Markdown 链接检查。

Checker 最后分别输出 errors、translation debt 和 migration debt。只有 errors 导致非零退出码，但最终 review 不允许保留 migration debt。

`scripts/check-docs.mjs` 应按照一个工作流组织：加载 manifest、发现文档、分类、验证、报告。多步骤入口用一行注释说明最终 outcome，顶层 action calls 之间保留空行。I/O、front matter 解析、链接解析和错误格式化放在有明确意图的 action 中，不把入口写成零散 helper 的调用堆栈。

`scripts/check-docs.test.mjs` 使用测试文件内嵌的目录树 fixtures。每个 case 应让路径和文件内容同时可读，不创建外部 fixture 目录，也不使用无法说明用途的占位文件名。测试至少覆盖：

- 完整 synced pair；
- 英文或中文 source 的 pending pair；
- needs-update pair；
- English-only glossary；
- neutral redirect 和 control file；
- ignored legacy；
- 临时 migration legacy 及其债务报告；
- 未分类 Markdown；
- 重复分类；
- 缺失或错误 counterpart；
- 无效 metadata；
- 断开的本地链接；
- translation debt 不导致失败。

## 现有内容迁移

迁移不是目录搬家。每篇旧文档先按事实类型拆分，确认新 owner 后再删除旧文件。

| 现有内容 | 处理方式 |
|---|---|
| 根 `README.md` | 精简为项目落地页，并增加中文 counterpart |
| `docs/README.*` | 重写为纯导航 |
| `docs/design/README.*` | 目标和取舍进入 intent；语言范围进入 specs；流水线和模块图进入 `how/README.*` |
| `docs/design/contracts.*` | 使用者可观察保证进入 specs；内部跨 crate 规则进入相应 how |
| analyzer 设计文档与 crate README | 合并为 `how/analyzer/README.*` |
| evaluator 设计文档与 crate README | 合并为 `how/evaluator/README.*` |
| builtin-fn 设计文档与两个 crate README | 拆入 builtin spec、`how/builtin_fn/README.*` 和 `how/builtin_fn_macros/README.*` |
| IDE 设计文档与 crate README | 合并为 `how/ide/README.*`；使用者行为进入 editor-services spec |
| `docs/signature-help.md` | `ParamShape` 机制进入 builtin_fn how；IDE 呈现和 active parameter 逻辑进入 ide how |
| WASM 设计文档与 crate README | 使用者边界进入 wasm spec；实现进入 `how/analyzer_wasm/README.*` |
| demo 设计文档与 example README | 合并为 `how/examples/vite/README.*` |
| `docs/design/testing.md` | 迁入 `contributing/testing.*` |
| `docs/changelog_entry_guidelines.md` | 迁入 `contributing/changelogs.*` |
| `docs/glossary.md` | 迁入根目录 English-only `GLOSSARY.md` |
| `docs/changelogs/` | 保留历史内容并逐步补齐 counterpart |
| `docs/assets/` | 继续共享；最终未被引用的资产才删除 |
| `docs/design/drift-tracker.md` | 删除；已解决内容不进入 current docs |
| `docs/builtin_functions/` | 在 builtin spec 和 how 落位后删除整个目录及其历史 prototype |
| design contract 与 module README templates | 删除；只保留并精简 changelog template |

每个 crate 根 `README.md` 保留为普通 Markdown 跳转文件，不使用 filesystem symlink，也不保留 API、命令、设计摘要或测试清单。保留文件可以继续满足 Cargo 自动识别 package README 的现有行为。`examples/vite/README.md` 使用同样的中性跳转方式。

## Builtin functions

完整 builtin 清单只存在于 `builtin_fn/src/builtins.rs` 的 Rust 声明中：

- `specs/builtin-functions/README.*` 说明使用者可依赖的调用规则、类型、错误和边界；
- `how/builtin_fn/README.*` 说明声明 DSL、签名模型和解析机制；
- `how/evaluator/README.*` 说明 evaluator 如何从同一 catalog 建立执行契约；
- specs 不复制或渲染完整名称和签名清单；
- unsupported declarations 不进入当前使用者规格。

删除 Markdown catalog 后，同时删除只为它存在的渲染和同步设施：

- `docs/builtin_functions/`；
- `builtin_fn/src/catalog_render.rs`；
- `builtin_fn/src/bin/builtin_catalog.rs`；
- `builtin_fn/tests/catalog_render.rs`；
- `analyzer/tests/builtin_spec_sync.rs`；
- 对应 exports、recipes、命令和文档引用。

Evaluator contract generation 仍然从 Rust builtin declarations 工作，不属于删除范围。

## 渐进交付

每个阶段可以由一个或多个 PR 完成。中文和英文可以分开交付，但必须准确标记翻译状态。

### 1. 建立维护规则和 checker

- 创建 `DOCUMENTATION.*` 和 `docs/manifest.toml`。
- 更新 `AGENTS.md` 的文档入口和 `.agent/` legacy 提示。
- 重构 `check-docs.mjs` 及其内嵌目录树 fixtures。
- 建立目标目录入口，但不填充未经授权的 intent/specs 内容。

完成条件：仓库中的每个 Markdown 都有唯一类别；尚未迁移的旧文件被逐项登记为 migration debt；结构错误失败；翻译债务和迁移债务可见但不失败。

### 2. 迁移 How

- 合并六个 crate README 与现有实现文档。
- 建立系统级 `how/README.*`。
- 迁移 Vite example 文档。
- 将源码旁 README 收缩为中性跳转。

完成条件：每个 crate 有一个可独立阅读的实现入口；旧文档不再是实现事实的第二个 owner。

### 3. 建立 Intent 与 Specs

- 获得相应 human 授权。
- 写一篇精简 intent。
- 审计代码、测试、schema 和现有契约，完成 contract inventory 并确定最终 spec 边界。
- 先完成并核验每轮 source language，再建立或更新 counterpart。

完成条件：contract inventory 中的每个 surface 都指向唯一 spec 或明确 exclusion；当前与未来没有混写；任何规范修改都处于明确授权范围内。

### 4. 迁移正交内容

- 将 English-only glossary 迁到根目录。
- 将 testing 和 changelog maintenance 文档迁入 `contributing/`。
- 完成 Contributing 小节定义的 coding rules audit。
- 保留并精简 changelog template。
- 补齐历史 changelog counterpart。

完成条件：术语、贡献规范和历史记录没有被强行塞入 intent/specs/how；所有应双语的正交文档均为 `synced`；coding rules audit 在 PR 或 review record 中有明确结论。

### 5. 清理旧结构

- 更新 Markdown 链接以及 code span 和普通文本中的旧路径。
- 完成“Builtin functions”和迁移表中列出的删除范围。

完成条件：旧路径不再被引用；manifest 不再包含 `legacy_files`；没有仍然有效的事实随旧文件丢失；产品运行时行为未改变。

### 6. 最终核验

- 运行 `just docs-check` 和受代码删除影响的 Rust checks。
- 检查 crate package README 仍指向根跳转文件。
- 确认不存在 `pending`、`needs-update` 或 migration debt。
- 对照本 review goal 检查每项阻塞条件。

完成条件：所有结构和代码检查通过；除明确例外外，全部正式文档均为 `synced`；reviewer 没有未解决的阻塞项。

## Review 阻塞条件

出现以下任一情况，文档重构不能视为完成：

- 违反“文档权限与 code review”中的 human-controlled 规则；
- contract inventory 未完整闭合，或同一技术事实仍有多个权威正文；
- specs 与 how 的边界、Current-only 规则或 crate-independent 组织方式没有落实；
- 任一 Markdown 未分类，或仍有 translation debt、migration debt、无效 metadata、错误配对或断链；
- crate 和 example 根 README 仍有跳转以外的实质内容；
- “Builtin functions”与迁移表中的删除和保留边界没有完成；
- checker workflow 或内嵌目录树 fixtures 不满足本计划定义；
- 任何非目标被纳入本次重构。

## 非目标

本次重构不负责：

- 将 Rust crate API 承诺为稳定公共规格；
- 建立 docs site 或新的发布工具链；
- 改变公式、分析、IDE、WASM 或 evaluator 的产品行为；
- 修改 CI。

本文件在重构期间保持为 control file。只有用户确认整个 review goal 已完成后，才另行决定是否删除；它不会迁入正式文档结构。

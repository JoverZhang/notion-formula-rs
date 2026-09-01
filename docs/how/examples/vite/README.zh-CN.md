---
doc_id: how.examples.vite
title: "Vite 示例如何协调 CodeMirror 与 WASM"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# Vite 示例如何协调 CodeMirror 与 WASM

[English](README.md)

Vite 示例通过浏览器调用 stateful WASM Analyzer。它展示一个应用怎样协调分析、编辑器辅助、CodeMirror
呈现和两个互不干扰的公式面板，同时把这些职责留在 Rust crate 之外。本文沿着 Current TypeScript 实现，
帮助维护者修改或调试示例。

本文提到的面板 ID、debounce 时长、分组行、chip、焦点、popover 布局和错误处理都属于
**example policy**，不是稳定的产品契约。用户可见的保证由 editor-services 和 WASM API specification
负责，对应的仓库路径是 `docs/specs/editor-services.*` 与 `docs/specs/wasm-api.*`。本文只解释 demo 如何
使用这些接口，不会重新定义 Analyzer、IDE 或 WASM 算法。

## 启动时共用一个边界 client，用两个 ID 区分面板

[`main.ts`](../../../../examples/vite/src/main.ts) 先挂载页面框架和静态表格，再创建一个 `AppVM`，并为
`FORMULA_IDS` 中的每个成员创建一个 `FormulaPanelView`。Current 示例使用 `f1` 和 `f2`。它们只是 demo
面板在 DOM、view model 和 debug bridge 中的标识，不是公式领域里可持久化的 identity。

每个面板都有独立的 CodeMirror instance 和初始示例源码。随后，`AppVM.start()` 初始化唯一的 WASM
`Analyzer`，把应用状态设为 ready，并安排分析 view model 中已经记录的源码。Current 启动流程里这些记录仍
为空，所以入口会在初始化完成后通过 `setSource` 提交两份示例源码。此后，待分析状态以 view model 为准，
不再只取自初始 DOM。

[`app/context.ts`](../../../../examples/vite/src/app/context.ts) 统一定义 `PROPERTY_SCHEMA` 以及由它构造的
`ANALYZER_CONFIG`。入口把这份配置交给共享 Analyzer，panel 则把同一份 property schema 用作 chip allowlist。

```text
main.ts
  |
  +-- root layout + theme toggle
  +-- static table view
  +-- AppVM -------------------------- one configured WASM Analyzer
  |      |
  |      +-- FormulaState f1
  |      +-- FormulaState f2
  |
  +-- FormulaPanelView f1 ------------ CodeMirror instance f1
  +-- FormulaPanelView f2 ------------ CodeMirror instance f2
```

[`app/types.ts`](../../../../examples/vite/src/app/types.ts) 中的 `FormulaState` 保存源码、diagnostic、token、
output type 和分析状态。`AppVM` 负责这些分析数据，并为每个 formula ID 保存一个 80 ms timer。Panel 只管理
selection、completion row、chip decoration 和 popover visibility 等临时呈现状态。两类状态分开后，一个
面板的焦点变化不会覆盖另一个面板的分析结果。

表格不会执行公式。[`table_view.ts`](../../../../examples/vite/src/ui/table_view.ts) 只渲染固定的示例行；
`main.ts` 根据每份公式的 analysis diagnostic 中是否存在 error，在对应公式列显示 `<error>` 或
`<pending>`。这段占位内容由 demo 决定，不是 evaluator result。

## wasm_client 是唯一直接导入 wasm-pack 产物的模块

[`analyzer/wasm_client.ts`](../../../../examples/vite/src/analyzer/wasm_client.ts) 是示例中唯一 import
wasm-pack glue 的 module。它在 module 作用域保存一个 `initPromise` 和一个 `Analyzer`：

- 浏览器初始化让 wasm-pack 自行加载 module URL；
- Vitest 使用的 Node 初始化会读取 `.wasm` 字节，再交给 wasm-pack，因为 Node 无法 fetch 生成的
  `file:` URL；
- 后续调用会复用首次创建的 promise 和同一个 Analyzer。初始化失败时，rejected 状态的 promise 也会保留，
  所以当前 client 无法重试。

初始化完成后，`analyze`、`format`、`apply_edits` 和 `help` 都直接转发。初始化前调用会抛出
`WASM analyzer is not initialized`。DTO shape 来自
[`analyzer/generated/wasm_dto.ts`](../../../../examples/vite/src/analyzer/generated/wasm_dto.ts)，
`src/pkg/` 则由 wasm-pack 生成。Client 只做 host 侧适配，不重新实现 Rust operation。

Help 还经过一层 demo adapter。`buildCompletionState` 把缺失的 completion 或 signature field 转为空值，并
丢弃类型不是 number 的 preferred index；row planner 再检查整数和边界。外层函数会捕获 `help` 抛出的
错误，并清空 completion 与 signature data。当前面板仍然显示时，completion 外框会保留并显示
`No suggestions`，原有 diagnostic 内容也可能继续显示。这项降级行为不属于 WASM contract。

## 一次编辑会推动两条独立更新环路

[`formula_panel_view.ts`](../../../../examples/vite/src/ui/formula_panel_view.ts) 把每个 CodeMirror instance
接到两条环路上：

```text
document change
  |
  +-- onSourceChange -> AppVM -> 80 ms debounce -> analyze
  |                                      |
  |                                      v
  |                         diagnostics, tokens, output type
  |                                      |
  |                                      v
  |                            FormulaPanelView.update
  |
  +-- CodeMirror update -> 120 ms debounce -> help(source, selection head)
                                             |
                                             v
                              completion rows + signature popover

selection-only change -----------------------+
```

分析环路更新 `FormulaState`。调用 WASM 前，`AppVM` 先发出 `analyzing`；成功后替换 diagnostic、
token 和 output type，再发出 `ok`。每份公式有自己的 debounce timer，所以编辑 `f1` 不会取消 `f2` 的分析。

Help 环路由各 panel 独立维护，在 document 或 selection 变化后都会运行。120 ms timer 触发时，它重新读取
当前 CodeMirror document 和 selection head。因此，没有焦点的 panel 仍会更新 completion state，只是不把
它显示出来。

每个 `EditorView` 都安装 CodeMirror 的 `history()` 与 `historyKeymap`。应用自己的 keymap 会先处理当前
completion UI 使用的 ArrowUp、ArrowDown、Escape、Tab 和 Enter，再交给 history 与 default keymap。这样既
保留普通 undo/redo，也允许可见 suggestion 使用自己的移动和应用按键。

JavaScript string 和 CodeMirror position 都使用 UTF-16 code unit，与示例所接收的 WASM DTO 一致，因此
TypeScript client 直接传递这些 position。UTF-16 到 Rust 的转换属于 `analyzer_wasm`；示例不再增加另一套
offset converter。

## Edit action 通过一次 CodeMirror transaction 回到编辑器

Completion item 中的 edit 使用原文档坐标。`applyCompletionItem` 位于
[`wasm_client.ts`](../../../../examples/vite/src/analyzer/wasm_client.ts)：它拒绝 disabled 或没有 edit 的
item，合并 primary edit 与 additional edit，再按原文档 `(from, to)` 坐标排序为 CodeMirror change。返回值
带有 cursor 时直接使用；否则，client 会把 cursor 放在 primary insertion 之后，并计入所有在 primary start
之前或正好结束于该位置的 additional edit。`FormulaPanelView` 在同一次 dispatch 中提交 change 和
selection，下一帧恢复焦点，然后重新请求 help。

Format 和 Quick Fix 走另一条路径：

- Format 把当前 source 和 selection head 交给 `format`。返回 source 有变化时，panel 替换整个 document，
  再把返回 cursor 限制到新 JavaScript string 的长度范围内。
- Quick Fix 通过 `firstDiagnosticAction` 选择第一个仍包含至少一个非负且未反转 range 的 diagnostic action，
  把 edits 交给 `apply_edits` 完成完整的边界校验，然后以同样方式替换完整 document 并限制 cursor。

只取第一个 action、过滤非法 edit、替换整份文档、按钮文案和静默失败都由 demo 决定。Edit 的校验和应用属于
Rust；UI 不复制 overlap 处理或 cursor rebasing。

## Completion row 保留结果顺序，只增加呈现标签

[`model/completions.ts`](../../../../examples/vite/src/model/completions.ts) 在不改变 completion item payload 的
前提下生成 render row：

1. 它只接受不重复、未越界的整数 `preferred_indices`，而且对应 item 必须 enabled；这些 item 按输入 index
   顺序在 `Recommended` 下各出现一次。
2. 它按原有顺序遍历剩余 enabled item。每当 item `kind` 与上一项不同，就在新的一段前插入人类可读的 group
   label。
3. Label row 不对应可选择的 item。Selection helper 会跳过 label，在首尾循环，并把无效 selection 归一化
   到 item row。

这些 group 只用于呈现，不属于 IDE service 承诺的 semantic category。Disabled item 会被省略，不显示
disabled reason。Panel 默认选择第一个可选 row，并把它滚动到可视区；方向键移动，Enter 或 Tab 应用，
Escape 清空 selection。Mouse hover 改变 selection，click 则应用对应的 completion item。

## Signature help 与 diagnostic 共用一个 popover

[`model/signature.ts`](../../../../examples/vite/src/model/signature.ts) 直接渲染 WASM 提供的 structured
signature segment。它优先使用指定的 signature，找不到时退回第一项；`Param` segment 的 `param_index`
等于 `active_parameter` 时会被标记为 active。UI 不解析 display type string，也不借此还原 parameter
structure。

Unwrapped layout 保持 segment 的原始顺序。Wrapped layout 会在 return arrow 之前寻找左右括号，再以 comma
segment 为界断开 parameter line，并使用两个空格缩进；找不到括号时仍使用 unwrapped layout。
[`signature_popover.ts`](../../../../examples/vite/src/ui/signature_popover.ts) 先绘制 unwrapped 版本，在下一帧
测量 overflow，必要时再以 wrapped mode 重绘。Viewport 宽于 760 px 时，popover 宽度取 viewport 的 28%，
限制在 240–360 px；左侧放得下时优先放左侧，否则尝试右侧，最后选择空间更大的一侧。Viewport 不超过
760 px 时，CSS 会把它静态放在 panel 内并占满宽度。

同一个 popover 还可以在 signature 下方显示 diagnostic text。
[`model/diagnostics.ts`](../../../../examples/vite/src/model/diagnostics.ts) 会把 range 限制在 CodeMirror
document 内，转换已知 severity，并在 lint message 为空时显示 `(no message)`。另一份文字列表保留 Analyzer
提供的 1-based line/column；有 chip coordinate 时还会附上对应 range。两处展示使用同一批 diagnostic，
不会另行排序或去重。

## Chip 维护第二套呈现坐标

Token decoration 和 property chip 由 CodeMirror state field 管理，分别定义在
[`editor_decorations.ts`](../../../../examples/vite/src/editor_decorations.ts) 和
[`editor/chip_decorations.ts`](../../../../examples/vite/src/editor/chip_decorations.ts)。State field 会随着
CodeMirror document transaction 映射已有 range。每当 `AppVM` 发出状态，`FormulaPanelView.update` 都会重新
构造这些 range；源码刚变化以及分析完成时都会执行这一步。

示例只把 exact token pattern `prop("Name")` 识别为 chip，而且 `Name` 必须存在于配置的 property set。完整
range 会被替换成 atomic CodeMirror widget；点击 chip 会选择它的 raw start，并把焦点交还编辑器。这些识别和
交互规则是 UI policy，不会改变 property-reference syntax。

[`chip_spans.ts`](../../../../examples/vite/src/chip_spans.ts) 校验 chip span 是否有序、互不重叠且未越界，再
构建一个把每段 raw chip range 压缩成一个 display position 的 map。Panel 会先构造并提交 decoration；该
阶段失败时会清空 chip range 与 decoration。随后单独构造 coordinate map；它的校验失败时只把 map 设为
`null`，已经提交的 decoration 仍然保留，但 raw-to-chip 坐标转换不可用。与 chip 相交的 diagnostic 会扩展到
完整 atomic range，并在 UI 中标记该 chip。因此，即使后续源码出现 syntax error，只要已有合法 chip 的
token 仍在，它们仍可呈现。

## 焦点决定 suggestion 是否可见，不决定分析状态

[`formula_panel_view.ts`](../../../../examples/vite/src/ui/formula_panel_view.ts) 在 module 作用域保存唯一的
active panel ID 和一份 panel UI handle registry。`focusin` 会激活当前 panel、隐藏上一 panel 的 completion
与 signature UI、请求当前 help 并呈现结果。`focusout` 等到下一 animation frame 才清除 active 状态，避免
焦点仍在同一个 CodeMirror 内部移动时发生闪烁。Window resize 只重新定位 active popover。

失去焦点只隐藏 suggestion，不会丢弃 source、analysis result、editor history 或 panel 最近一次 completion
state。切换到另一 panel 只会更换显示 suggestion 的面板，不会更换共享 WASM Analyzer 的配置。这项行为由
[`suggestions_panel_focus.spec.ts`](../../../../examples/vite/tests/e2e/suggestions_panel_focus.spec.ts) 做
end-to-end 覆盖。

## 失败停在最近的 demo 边界

Current 示例在不同层采用不同的降级方式：

| 失败 | Demo 响应 | 代码入口 |
| --- | --- | --- |
| WASM 初始化被拒绝 | `start()` reject，入口记录错误，panel 不会进入 ready 状态 | `main.ts`、`AppVM.start` |
| 初始化完成后 analyze 抛错 | 生成一个 zero-width `analysis failed` diagnostic，清空 token/type，并设为 `error` | `AppVM.runAnalyze` |
| Help 抛错 | 清空 completion/signature data；active panel 显示 `No suggestions` | `safeBuildCompletionState` |
| Format 或 Quick Fix 抛错 | 保持编辑器不变 | `formula_panel_view.ts` 中的 button handler |
| 单个 token span 非法 | 跳过该 span，保留其他 token decoration | `computeTokenDecorationRanges` |
| Chip decoration 构造抛错 | 清空 chip range 与 decoration | `formula_panel_view.ts` |
| Chip offset map 校验失败 | 保留已提交的 decoration，但不建立 coordinate map | `chip_spans.ts`、`formula_panel_view.ts` |

这些降级方式让 demo 能继续交互，也把失败留在方便观察的接缝。相关 message 或静默行为不属于 compatibility
promise。稳定的 editor 或 WASM 行为有误时，应修改对应的实现或 specification，不要在 client 中加入补偿
规则。

## 按改动所在接缝选择 build 与 test

示例要求 Node 20 或更新版本以及 pnpm 10。命令定义在
[`package.json`](../../../../examples/vite/package.json)：

- `wasm:build` 以 `web` target 对 `analyzer_wasm` 运行 wasm-pack，把结果写入 `src/pkg/`；
- `dev`、`build`、`preview` 负责 Vite 生命周期；
- `test` 运行 12 个 Vitest 文件，`test:e2e` 运行 Chromium Playwright suite；
- `typecheck`、`lint`、`format`、`check` 检查 TypeScript 与 frontend 源码质量。

根目录 [`justfile`](../../../../justfile) 组合这些命令。`just test-example-vite` 会安装 lockfile 固定的
frontend dependency、重新 build WASM，再依次运行 Vitest 和 Playwright。
[`tests/unit/`](../../../../examples/vite/tests/unit/) 隔离测试 row planning、edit conversion、diagnostic/chip
mapping、signature layout 和 WASM error adapter；
[`tests/e2e/`](../../../../examples/vite/tests/e2e/) 则通过浏览器覆盖真实的 focus、selection、history、
popover placement、UTF-16 cursor flow、decoration rendering 和 build 后的 WASM module。

[`playwright.config.ts`](../../../../examples/vite/playwright.config.ts) 会为测试 suite build 并启动 preview
server。`PW_HOST` 同时控制 Playwright base URL 和 preview bind address，默认值为 `127.0.0.1`。没有设置
`PW_PORT` 时，配置会根据 checkout path 计算 port，降低并行 worktree 撞端口的概率。测试用 `?debug=1`
打开应用，并等到 `window.__nf_debug` 出现后再检查 panel。

## 从呈现边界向内调试

先检查最早拥有该现象的层：

- Focus、group、popover、button 或 decoration 有误时，从 `formula_panel_view.ts` 和对应的 `src/model` 或
  editor-decoration helper 开始。能抽成纯转换的问题，先在 Vitest 复现，再进入浏览器。
- Source、status 过期或两个 panel 互相影响时，检查 `AppVM` 及其 per-formula timer。
- 初始化、DTO shape 或 host call 抛错时，在 `wasm_client.ts` 边界检查 generated DTO import 与 build
  后的 wasm-pack output。
- 结果传输正确但语义不对时，继续进入 WASM、IDE 或 Analyzer owner，不要在 demo presentation layer 改写
  结果。

[`debug/debug_bridge.ts`](../../../../examples/vite/src/debug/debug_bridge.ts) 提供浏览器层检查入口。它按
formula ID 注册 handle。`getState` 返回 source、output type、diagnostic count 和 token count；
`window.__nf_debug` 的其他方法提供 Analyzer 与 CodeMirror diagnostic、token/chip range、raw-to-chip
coordinate mapping 和 selection helper。Development、test 或显式 `?debug=1` 会启用注册；普通
production build 没有该 query 时不会暴露这套接口。

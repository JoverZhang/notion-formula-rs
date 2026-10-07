---
doc_id: experiments.appflowy-formula-benchmark
title: "AppFlowy 公式 A/B benchmark"
language: zh-CN
source_language: en
counterpart: ./README.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# AppFlowy 公式 A/B benchmark

[English](README.md)

在固定的 AppFlowy 版本上比较原 JavaScript 实现与 Rust/WASM 集成。[运行器](../../../tools/appflowy-benchmark/run.mjs)记录配对的浏览器测量结果，校验输出，并在样本缺失或工作量不同时报错。某个负载下更快，不能推出所有数据库和环境中都更快。

## 复现对比

安装 Node、固定版本 AppFlowy 指定的包管理器、带 `wasm32-unknown-unknown` target 的 Rust、`wasm-pack` 和 Chromium。准备过程拉取[不可变版本](../../../tools/appflowy-benchmark/revisions.json)，按各自的冻结 lockfile 安装依赖，并构建新版所固定的 SDK。生成的 checkout 放在运行器缓存中，不进入源码目录。

未指定 `--browser-executable` 时，运行器使用固定版本 Playwright 配套的 Chromium。先运行 `--prepare`，再通过 `pnpm --dir <cache>/native exec playwright install chromium` 安装该浏览器。

```sh
# 先用小样本验证测试工具；这种样本量不支持性能结论。
just benchmark-appflowy --smoke --output /tmp/formula-benchmark-smoke \
  --browser-executable /path/to/chromium

# 默认：四组负载 × 100/1,000/10,000 行，五个独立会话，
# 每个热运行操作在每个会话中预热五次，再测量二十次。
just benchmark-appflowy --output /tmp/formula-benchmark-chain \
  --browser-executable /path/to/chromium
```

每轮使用新的输出目录。`--cache` 指定可复用的 checkout/构建缓存；`--families`、`--rows`、`--sessions`、`--samples`、`--warmups` 可以缩小范围，实际配置会写入记录。`--help` 列出全部选项。构建完成后才开始测量，同一时间只测一个版本。

缓存锁防止多个运行器同时使用同一组生成的 checkout。进程被杀死后，应先检查 `benchmark.lock` 中的 PID，再删除遗留锁。测量期间避免在同一机器上运行其他构建或 benchmark。

完整应用层需要兼容的 AppFlowy Cloud 测试部署和普通工作区账号。通过包含 `email`、`password` 的私密 JSON 文件提供凭据，或使用私密的 Playwright storage-state 文件。运行器创建专用 benchmark 数据库，不把现有用户数据库作为测试负载。

```sh
just benchmark-appflowy --layer all --output /tmp/formula-benchmark-all \
  --backend-url http://127.0.0.1:8000 \
  --gotrue-url http://127.0.0.1:9999 \
  --credentials /private/appflowy-test-account.json \
  --full-app-rows 100 --browser-executable /path/to/chromium
```

凭据、数据库标识、登录状态和详细应用错误保存在私密文件中，与可分享的结果分开；不要公开这些私密文件。`--layer full-app` 只运行完整应用对比。该层默认使用 100 行的代表性数据库；更大的完整应用数据集通过 `--full-app-rows` 显式选择。

缓存保存专用数据库身份，后续运行复用同一批数据库，结束后保留数据供复查。`--screenshots` 在 UI 测量后仅截取虚构测试数据的 Grid 和编辑器内容，这些图片可单独分享。

## 测量内容

| 层次 | 起点 → 完成条件 | 计入的工作 |
| --- | --- | --- |
| 计算链路 | 首次请求或输入/公式修改 → 全部请求结果完成 AppFlowy 展示转换 | 生产 Yjs 解码、需要时的 schema/Engine 初始化、依赖计算、缓存行为、新版转列、Worker/WASM 往返及结果转换 |
| 完整应用 | 导航、输入提交或编辑器输入 → 当前正确结果显示在界面中 | 生产应用资源、真实 Cloud 加载、调度、渲染及 Worker/WASM；后续操作保留记录中说明的缓存状态 |

[计算负载](../../../tools/appflowy-benchmark/browser/dataset.ts)覆盖算术、字符串/列表、依赖链与菱形依赖，以及包含长公式的发票流程。两版请求相同的行和公式目标。数据构造在计时前完成，独立结果模型与校验和在计时结束后验证。这些负载不包含依赖当前时间的公式。

计算链路的每个会话启动新的浏览器进程。首次求值单独报告；热运行操作包括重复读取、修改一行、修改全部行以及修改已保存公式。双方保留生产缓存策略。原版结果缓存最多容纳 512 项 / 4 MiB，因此较大工作集的重复读取可能触发淘汰；重复读取不会被标记为必然命中缓存。

完整应用使用原 Grid 和编辑器，两版的视口和数据一致。benchmark 在双方生产构建中保留测试选择器并注入小型数据准备入口，记录插桩内容的哈希。Grid 继续使用虚拟滚动。可见行就绪与整列计算属于不同负载，分别报告。输入修改测量本地结果传播，不包含后续 Cloud 持久化确认。

完整应用的每个会话也启动新浏览器进程。首次携带登录态打开数据库时，浏览器缓存为空，Cloud 缓存仍然共享；后续导航、单元格编辑、预览更新、非法表达式诊断和属性补全复用预热后的会话。UI 就绪要求连续两个动画帧结果稳定；原始样本分别记录首次 DOM 就绪时间（`domReadyMs`）和稳定性确认时间（`stableFrameMs`）。

所有耗时取自浏览器时间戳。Playwright 只轮询获取已经结束的测量。过期结果、未完成行、错误值、意外错误或请求工作量不一致都会导致校验失败。计时过程中不会拦截并序列化每条 Worker 消息。内存及另行插桩的 CPU 剖析不属于这一版延迟对比。

## 阅读结果

`results.json` 包含应用及 benchmark 的源码版本与哈希、数据集与插桩信息、环境、限制、原始样本及对比汇总，浏览器计时代码也计入版本记录。原子写入的检查点保存当前阶段和已完成的层次；中断记录保持 `running` 或 `failed` 状态。`samples.csv` 保存原始测量行；若后续操作失败，`samples.ndjson` 保留已完成的进度。生成的 `report.md` 是 JSON 的便捷展示。

[统计模块](../../../tools/appflowy-benchmark/report.mjs)按负载、操作、行数、会话和迭代号配对样本，报告绝对 P50/P95 耗时及 **新版 / 原版** 比值，小于 1 表示新版更快。95% 区间使用固定的统计随机种子，对完整配对会话重采样 2,000 次。少于五个会话时不下方向性结论；中位数落到时钟零分辨率时不计算加速比。小样本的尾部分位数仅供探索。失败和不支持的案例不会被静默移除。

计算链路的 JSON 汇总还按中位耗时计算每秒完成展示转换的请求单元格数。该速率包含整个操作及其缓存行为，并非单独 evaluator 的吞吐量。完整应用的可见结果延迟不会换算成整库吞吐量。

`just test-benchmark-report` 检查配对、正确性约束、零分辨率处理和原始 CSV 输出；真实浏览器 smoke 验证生产接入与独立结果模型。普通 CI 不设置依赖机器性能的耗时门槛。

## 2026-10-07 观察

本次结果支持新版接入在大数据量计算中更快，但测试中的完整编辑器交互更慢。它不能单独证明 Rust evaluator 更快，因为比较还包含两版不同的缓存、批处理、Worker、调度和渲染行为。

环境为 Linux x64、AMD Ryzen 9 7950X、32 个逻辑 CPU、66.5 GB 内存。两版均使用 Chrome for Testing 145.0.7632.6、Node 25.9.0 和 pnpm 10.9.0。AppFlowy 固定版本分别为原版 `4fdb4f4`、新版 `c4485cd`，SDK 为 `43b393c`；真实后端报告 Cloud 0.19.0。完整版本号、构建哈希和[后端镜像标识](2026-10-07/backend-images.json)随证据保存。每项对比使用五个独立配对会话，每个热运行操作在每个会话中预热两次、测量五次，即每版每个案例有五个冷启动或首次样本、25 个热运行样本。

原始全层运行使用 benchmark 版本 `535a5e9`，在完成全部 2,520 条计算链路样本后，因 WebSocket 断开触发未处理的 `ECONNRESET`，于完整应用准备阶段**失败**。[原始逐条日志](2026-10-07/chain-samples.ndjson)按字节保留。[恢复上下文](2026-10-07/chain-recovery-context.json)区分原始测量与依据前一轮 smoke、匹配的缓存构建标记重建的元数据；原运行的即时负载记录已经丢失。[恢复的链路报告](2026-10-07/chain-recovered.json)核验完整样本矩阵，并保留原运行失败的事实。它不代表一次通过的全层运行。

修复连接清理并增加报告检查点后，版本 `9ee4aaf` 的独立完整应用运行**通过全部 260 条样本**，应用版本、浏览器和专用数据库保持一致。[UI 结果](2026-10-07/full-app-results.json)、[CSV](2026-10-07/full-app-samples.csv) 和[逐条样本](2026-10-07/full-app-samples.ndjson)独立保存该轮证据。两个测量层均未删除任何耗时样本。

下表为计算链路 P50，单位毫秒，顺序为 **原版 → 新版**。完整[链路 CSV](2026-10-07/chain-samples.csv) 和 JSON 包含全部数据规模、五种操作、P95 和配对会话区间。

| 负载 | 首次计算，100 行 | 首次计算，10,000 行 | 修改全部行，10,000 行 | 重复读取，10,000 行 |
| --- | ---: | ---: | ---: | ---: |
| 算术 | 8.8 → 35.3 | 386.7 → 136.4 | 409.9 → 120.8 | 367.7 → 5.9 |
| 字符串/列表 | 10.8 → 38.5 | 532.5 → 261.0 | 550.2 → 236.7 | 496.5 → 8.9 |
| 依赖计算 | 42.4 → 38.1 | 3189.7 → 178.5 | 3047.9 → 155.6 | 3035.9 → 12.7 |
| 发票长公式 | 13.5 → 38.9 | 622.8 → 204.7 | 657.9 → 168.8 | 609.5 → 6.5 |

较小负载的首次计算暴露了新版初始化成本。大数据量重复读取的优势包含原版缓存淘汰与新版快照复用的差异，不能把它报告为单纯的 Rust 与 JavaScript 算术吞吐量差异。

完整 UI 的数据库有 100 行、两个存在依赖关系的公式字段。导航就绪检查覆盖 23 个可见行、46 个公式单元格。区间针对新版与原版 P50 的比值，跨过 1 时不支持明确方向。

| UI 操作 | 原版 P50 / P95（ms） | 新版 P50 / P95（ms） | 新版 / 原版 [95% 区间] |
| --- | ---: | ---: | ---: |
| 冷启动打开数据库 | 1351.6 / 1405.6 | 1409.0 / 2267.6 | 1.04 [0.99, 1.93] |
| 热运行打开数据库 | 913.9 / 2011.0 | 958.7 / 2105.7 | 1.05 [1.02, 1.10] |
| 提交输入 → 依赖结果 | 22.3 / 39.1 | 39.1 / 70.5 | 1.75 [1.74, 1.77] |
| 修改公式 → 有效预览 | 24.0 / 25.0 | 223.6 / 256.7 | 9.32 [9.26, 9.41] |
| 修改公式 → 当前诊断 | 27.3 / 27.9 | 93.9 / 94.4 | 3.44 [3.41, 3.45] |
| 修改公式 → 属性补全 | 27.2 / 29.1 | 93.5 / 94.1 | 3.44 [3.43, 3.46] |

新版编辑器包含 [70 ms 分析防抖](https://github.com/JoverZhang/AppFlowy-Web/blob/c4485cdec14ed4e089e2472c882237d8a4ec0c3e/src/components/database/components/property/formula/FormulaEditor.tsx#L213)和 [120 ms 预览防抖](https://github.com/JoverZhang/AppFlowy-Web/blob/c4485cdec14ed4e089e2472c882237d8a4ec0c3e/src/components/database/components/property/formula/use-native-preview.ts#L287)。这些延迟会增加用户等待时间，可作为后续优化实验的具体切入点；本次延迟测量没有将它们与 Engine 和渲染耗时分别隔离。两版都保留两帧就绪确认。冷启动打开没有明确的方向性结果，五项热运行 UI 操作在本次样本中均为原版更快。

测量后截图：[原版 Grid](2026-10-07/full-app-legacy-100-grid.png)、[新版 Grid](2026-10-07/full-app-native-100-grid.png)、[原版编辑器](2026-10-07/full-app-legacy-100-editor.png)、[新版编辑器](2026-10-07/full-app-native-100-editor.png)。编辑器截图使用无敏感内容的草稿 `1 + 2`，用于展示完整界面，并非计时表达式。

复现本次样本量时，在前述命令中加入 `--sessions 5 --samples 5 --warmups 2`。不启动浏览器、只核验恢复证据时，指定新的输出目录：

```sh
node docs/experiments/appflowy-formula-benchmark/2026-10-07/recover-chain.mjs \
  /tmp/formula-benchmark-recovered-check
```

恢复脚本先验证原始 SHA-256、完整样本覆盖与配对，再重新生成统计，不会重跑测量或把失败记录改为通过。结果仅覆盖一台主机、一个 Chromium 版本和合成负载；基于五个会话的区间及小样本 P95，仍需后续测量才能支持更广泛的性能结论。

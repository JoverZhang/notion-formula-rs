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

`results.json` 包含应用及 benchmark 的源码版本与哈希、数据集与插桩信息、环境、限制、原始样本及对比汇总，浏览器计时代码也计入版本记录。`samples.csv` 保存原始测量行；若后续操作失败，`samples.ndjson` 保留已完成的进度。生成的 `report.md` 是 JSON 的便捷展示。

[统计模块](../../../tools/appflowy-benchmark/report.mjs)按负载、操作、行数、会话和迭代号配对样本，报告绝对 P50/P95 耗时及 **新版 / 原版** 比值，小于 1 表示新版更快。95% 区间使用固定的统计随机种子，对完整配对会话重采样 2,000 次。少于五个会话时不下方向性结论；中位数落到时钟零分辨率时不计算加速比。小样本的尾部分位数仅供探索。失败和不支持的案例不会被静默移除。

计算链路的 JSON 汇总还按中位耗时计算每秒完成展示转换的请求单元格数。该速率包含整个操作及其缓存行为，并非单独 evaluator 的吞吐量。完整应用的可见结果延迟不会换算成整库吞吐量。

`just test-benchmark-report` 检查配对、正确性约束、零分辨率处理和原始 CSV 输出；真实浏览器 smoke 验证生产接入与独立结果模型。普通 CI 不设置依赖机器性能的耗时门槛。

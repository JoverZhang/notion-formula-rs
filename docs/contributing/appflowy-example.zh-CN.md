---
doc_id: contributing.appflowy-example
title: "构建 AppFlowy 示例"
language: zh-CN
source_language: en
counterpart: ./appflowy-example.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# 构建 AppFlowy 示例

[English](appflowy-example.md)

[`examples/appflowy-web`](../../examples/appflowy-web) 以 submodule 固定
[AppFlowy-Web fork](https://github.com/JoverZhang/AppFlowy-Web) 的提交。
固定版本的示例使用共享 SDK 完成公式求值、编辑及数据库中各项功能的公式计算。
存储、兼容性和编辑器行为由其[接入指南](../../examples/appflowy-web/doc/NOTION_FORMULA.md)维护。
[Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67) 链接实现 PR 和验证结果。

## 准备与启动

安装仓库所需的 Rust/WASM 工具、Node.js 20 或更新版本，以及各 package manifest 声明的 pnpm 版本。
AppFlowy 保留自己的依赖锁文件。在仓库根目录运行：

```sh
just deps-appflowy
```

此命令构建 `@notion-formula/sdk`，初始化固定提交的 submodule，将 SDK 的 `package.json`
和 `dist` 复制到示例的本地 `.notion-formula-sdk` 包，然后安装 AppFlowy 的依赖。
SDK 源码和生成声明由本仓库维护。修改 Rust、Worker client 或 submodule 指向的提交后，重新运行此命令。

AppFlowy 需要 AppFlowy Cloud 后端。按照其
[开发指南](https://github.com/JoverZhang/AppFlowy-Web/blob/main/doc/DEVELOPMENT_GUIDE.md)
准备后端。首次配置本地环境时，将 `examples/appflowy-web/dev.env` 复制为
`examples/appflowy-web/.env`，调整 HTTP、认证和 WebSocket 地址。
更新 submodule 时保留已有的 `.env`。

```sh
just run-example-appflowy
just build-example-appflowy
```

第一个命令启动开发服务器，第二个命令生成生产构建。它们使用示例配置的后端，不会启动或重置后端服务。

## 同时修改两个仓库

在 AppFlowy 自己的任务 worktree 中修改代码。将该 checkout 连接到当前 SDK 构建：

```sh
just wasm
node scripts/stage-appflowy-sdk.mjs /path/to/appflowy-worktree
```

复制目录中只有可分发的包文件，使开发与生产构建使用相同的 SDK 入口。
AppFlowy 的依赖预构建需要排除 `@notion-formula/sdk`，以保留相对于模块的 Worker 和 WASM 资源地址。

先将 AppFlowy 改动提交并推送到 fork，再更新父仓库中的 submodule 指针。
Review 两个仓库的差异，将 gitlink 固定到经过验证的提交。父仓库 checkout 不会自动跟随 fork 分支的最新版本。

## 验证共享包

运行 `just test-example-vite`，执行已有的 Engine/Draft 回归测试。浏览器契约测试只安装 SDK manifest
和 `dist`，在非根路径下验证开发与生产环境的默认 Worker，并在示例的 `test-results` 中保存 JSON 结果。
执行 `just deps-appflowy` 后，可运行 AppFlowy 的真实 Worker/WASM 集成测试，无需登录 Cloud：

```sh
pnpm -C examples/appflowy-web exec playwright install chromium
pnpm -C examples/appflowy-web exec playwright test native-formula --config=playwright.integrations.config.ts --project=chromium
FORMULA_FIXTURE_PRODUCTION=1 pnpm -C examples/appflowy-web exec playwright test native-formula --config=playwright.integrations.config.ts --project=chromium
```

这些用例将 JSON 和截图保存在 submodule 的 `test-results` 目录中。
需要认证的应用测试仍须先完成上面的 Cloud 配置。

父仓库的文档检查跳过 submodule 和生成的 WASM 包。AppFlowy 的文档与检查仍由其仓库维护。

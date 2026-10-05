---
doc_id: contributing.appflowy-example
title: "Build the AppFlowy example"
language: en
source_language: en
counterpart: ./appflowy-example.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-05
---

# Build the AppFlowy example

[简体中文](appflowy-example.zh-CN.md)

[`examples/appflowy-web`](../../examples/appflowy-web) pins the
[AppFlowy-Web fork](https://github.com/JoverZhang/AppFlowy-Web) as a submodule.
The pinned example uses the shared SDK for formula evaluation, editing, and database consumers.
Its [integration guide](../../examples/appflowy-web/doc/NOTION_FORMULA.md) owns the storage,
compatibility, and editor behavior. [Issue #67](https://github.com/JoverZhang/notion-formula-rs/issues/67)
links the implementation PRs and verification.

## Prepare and run

Install the repository's Rust/WASM tools, Node.js 20 or later, and the pnpm versions declared in
the package manifests. AppFlowy keeps its own lockfile. From the repository root:

```sh
just deps-appflowy
```

This builds `@notion-formula/sdk`, initializes the pinned submodule, copies the SDK's `package.json`
and `dist` into the example's local `.notion-formula-sdk` package, and installs AppFlowy's dependencies.
The SDK source and generated declarations stay owned by this repository. Re-run the command after
changing Rust, the Worker client, or the pinned submodule commit.

AppFlowy needs an AppFlowy Cloud backend. Follow its
[development guide](https://github.com/JoverZhang/AppFlowy-Web/blob/main/doc/DEVELOPMENT_GUIDE.md)
to prepare a backend. For a new local setup, copy `examples/appflowy-web/dev.env` to
`examples/appflowy-web/.env` and adjust the HTTP, authentication, and WebSocket endpoints.
Keep an existing `.env` when updating the submodule.

```sh
just run-example-appflowy
just build-example-appflowy
```

The first command starts the development server; the second produces the production build.
These commands use the example's configured backend and do not start or reset backend services.

## Work on both repositories

Make AppFlowy changes in its own task worktree. To connect that checkout to the current SDK build:

```sh
just wasm
node scripts/stage-appflowy-sdk.mjs /path/to/appflowy-worktree
```

The staging directory contains only distributable package files, so production and development
resolve the same SDK entry points. AppFlowy's dependency optimization must exclude
`@notion-formula/sdk` to preserve its module-relative Worker and WASM URLs.

Commit and push AppFlowy changes to the fork before advancing the parent's submodule pointer.
Review both diffs and keep the gitlink pinned to the tested commit. A parent checkout does not
automatically follow the fork's latest branch.

## Verify the shared package

Run `just test-example-vite` for the existing Engine/Draft regression suite. Its browser contract
installs only the SDK manifest and `dist`, then exercises the default Worker in development and
production under a non-root URL. Playwright records JSON artifacts in the example's `test-results`.
After `just deps-appflowy`, run AppFlowy's real Worker/WASM integration fixtures without a Cloud login:

```sh
pnpm -C examples/appflowy-web exec playwright install chromium
pnpm -C examples/appflowy-web exec playwright test native-formula --config=playwright.integrations.config.ts --project=chromium
FORMULA_FIXTURE_PRODUCTION=1 pnpm -C examples/appflowy-web exec playwright test native-formula --config=playwright.integrations.config.ts --project=chromium
```

These cases retain JSON and screenshot artifacts in the submodule's `test-results` directory.
Authenticated application tests still require the Cloud setup above.

Parent documentation checks skip the submodule and generated WASM package. AppFlowy's documentation
and checks remain owned by its repository.

---
doc_id: changelog.20260213-wasm-stateful-analyzer-config-and-offset-renames
title: "把 WASM Analyzer 改为 stateful API"
language: zh-CN
source_language: en
counterpart: ./20260213-wasm-stateful-analyzer-config-and-offset-renames.md
implementation_status: historical
document_status: stable
translation_status: synced
last_verified: 2026-02-13
---

# 把 WASM Analyzer 改为 stateful API

[English](20260213-wasm-stateful-analyzer-config-and-offset-renames.md)

- Type: Changed
- Component: analyzer_wasm, examples/vite, docs

## Summary

WASM API 改为 instance-based（`new Analyzer(config)`），并删除接收 `context_json` string 的 function-style
export。Config 改用 object input（`AnalyzerConfig`），在顶层支持 `preferred_limit`，传入 `null` 时使用默认值
`5`。

UTF 转换 helper 也集中到 `analyzer_wasm/src/offsets.rs` 并改名为：

- `utf16_to_8_offset`；
- `utf8_to_16_offset`；
- `utf16_to_8_cursor`；
- `utf16_to_8_text_edits`。

## Compatibility notes

- Breaking WASM API：
  - 删除 `analyze(source, context_json)`；
  - 删除 `ide_help(source, cursor_utf16, context_json)`；
  - 改为创建一次 `Analyzer`，再调用 instance method。
- Breaking WASM config shape：
  - 删除嵌套的 `completion.preferred_limit`；
  - 改用 `AnalyzerConfig.preferred_limit`。
- Breaking demo integration：
  - `examples/vite` 改用 `initWasm(ANALYZER_CONFIG)` 初始化 wrapper，并使用 stateful `Analyzer`
    instance。

## Tests

- `cargo test -p analyzer_wasm`
- `cargo run -p analyzer_wasm --bin export_ts`
- `pnpm -C examples/vite -s run wasm:build`
- `pnpm -C examples/vite -s run test -- tests/unit/signature_help_instantiated.test.ts tests/unit/wasm_errors.test.ts`

## Links

- `analyzer_wasm/src/lib.rs`
- `analyzer_wasm/src/offsets.rs`
- `analyzer_wasm/src/dto/v1.rs`
- `analyzer_wasm/tests/analyze.rs`
- `examples/vite/src/analyzer/wasm_client.ts`

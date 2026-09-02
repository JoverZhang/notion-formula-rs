---
doc_id: specs.formula-runtime-wasm
title: "Formula Runtime 的 WASM Wrapper"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-runtime-wasm.md
implementation_status: planned
document_status: draft
translation_status: pending
last_verified: 2026-09-02
---

# Formula Runtime 的 WASM Wrapper

[English](formula-runtime-wasm.md)

> 本文描述 Planned 接口，不是当前 WASM 契约。

本文只描述 [Formula Runtime Rust 接口](formula-runtime.zh-CN.md)如何通过 WASM 在 Worker 中调用，不重复公式分析和求值语义。

```text
main thread
  → async client
    → Worker
      → WASM wrapper
        → FormulaEvaluator / FormulaAnalyzer
```

## Client

```ts
interface FormulaEvaluatorClient {
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  close(): Promise<void>;
}

interface FormulaAnalyzerClient {
  getState(): Promise<FormulaAnalyzerState>;
  queryCursorHelp(cursor: number): Promise<CursorHelp>;
  queryQuickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  queryFormatEdits(): Promise<FormulaEdit>;
  applyEdits(edit: FormulaEdit, cursor: number): Promise<ApplyEditsResult>;
  close(): Promise<void>;
}
```

Wrapper 只负责：

```text
- Worker RPC
- Rust DTO 与 JavaScript DTO 的无损转换
- UTF-8 byte offset 与 UTF-16 code-unit offset 的转换
- Rust Result 与 Promise 的转换
- 对象生命周期
```

formula diagnostics 和 row errors 属于成功结果。其他 Rust error 拒绝 Promise。

## Worker 和生命周期

```text
FormulaEvaluatorClient
  - 每个请求自包含
  - 调度方式不改变结果

FormulaAnalyzerClient
  - 一个 client 对应一个 Analyzer session
  - 同一 session 的调用按入队顺序执行
  - query 复用同一个 compiled state
```

`close()` 幂等。开始关闭后拒绝新调用，等待已入队调用完成，释放 WASM 对象，再终止 Worker。Worker 和线程池的具体数量不属于接口。

Wrapper 不实现公式解析、类型推断、依赖图、循环依赖检测、求值或文本编辑语义。

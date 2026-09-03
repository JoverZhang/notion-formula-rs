---
doc_id: specs.formula-runtime-wasm
title: "FormulaEngine 的 WASM Wrapper"
language: zh-CN
source_language: zh-CN
counterpart: ./formula-runtime-wasm.md
implementation_status: planned
document_status: draft
translation_status: pending
last_verified: 2026-09-03
---

# FormulaEngine 的 WASM Wrapper

[English](formula-runtime-wasm.md)

> 本文描述 Planned 接口，不是当前 WASM 契约。

本文只描述如何通过 WASM 在 Worker 中调用 [FormulaEngine Rust 接口](formula-runtime.zh-CN.md)，不重复 Engine 或 Draft 语义。

```text
main thread
  → FormulaEngineClient
    → Worker RPC
      → WASM wrapper
        → FormulaEngine
          → FormulaDraft
```

## Client

```ts
interface FormulaEngineClient {
  getState(): Promise<FormulaEngineState>;
  upsertProperty(property: PropertySchema): Promise<ChangeResult>;
  removeProperty(id: PropertyId): Promise<ChangeResult | null>;
  upsertFormula(formula: FormulaDefinition): Promise<ChangeResult>;
  removeFormula(id: PropertyId): Promise<ChangeResult | null>;
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  close(): Promise<void>;
}

interface FormulaDraftClient {
  getState(): Promise<FormulaDraftState>;
  help(cursor: number): Promise<CursorHelp>;
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  applyEdits(edit: FormulaEdit, cursor: number): Promise<ApplyEditsResult>;
  intoDefinition(): Promise<FormulaDefinition>;
  close(): Promise<void>;
}
```

Wrapper 只负责：

```text
- Worker RPC 与 session 路由
- Rust DTO 与 JavaScript DTO 的无损转换
- UTF-8 byte offset 与 UTF-16 code-unit offset 的转换
- Rust Result 与 Promise 的转换
- Engine 和 Draft 的生命周期
```

`FormulaEngineClient` 及其创建的所有 `FormulaDraftClient` 共享一条 FIFO 队列，调用按入队顺序串行执行。每个 `FormulaDraftClient` 对应 Worker 中的一份独立 draft；`close()` 释放它即表示 discard。`intoDefinition()` 消耗 draft，返回的 definition 仍需显式传给 `upsertFormula()` 才会修改 Engine。

Engine client 的 `close()` 幂等：开始关闭后拒绝新调用，等待已入队调用完成，释放关联对象，再终止 Worker。Worker 和线程池的具体数量不属于接口。

Wrapper 不实现依赖分析、循环检测、求值或文本编辑语义。

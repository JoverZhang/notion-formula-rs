// AUTO-GENERATED: node scripts/generate-wasm-client.mjs
// Source: docs/specs/wasm-api.md

import type {
  CompletionConfig,
  CursorHelp,
  DiagnosticId,
  EvaluateInput,
  EvaluateResult,
  ExpressionUpdate,
  FormulaDefinition,
  FormulaDraftState,
  FormulaEdit,
  FormulaEngineChangeResult,
  FormulaEngineState,
  FormulaSchema,
  PropertyDefinition,
  PropertyId,
  PropertyState,
  QuickFix,
  UpdateExpressionResult,
} from "./generated/wasm_dto.js";
import type { FormulaWorker } from "./rpc.js";

// Pure, synchronous; no Worker/WASM initialization. Quote a valid Unicode value,
// escaping backslash, double quote, newline and tab; preserve other characters.
// a"b -> "a\"b"; empty -> "".
export declare function encodeFormulaString(value: string): string;

// Pure, synchronous; input is one complete, valid String token.text from the lexer.
// Decode once: \n -> newline, \t -> tab, \" -> double quote, \\ -> backslash.
// "a\"b" -> a"b; "" -> empty. Preserve raw Unicode and control characters.
export declare function decodeFormulaString(literal: string): string;

// An Engine and all its Drafts share one FIFO queue; rejected calls do not stop it.
// Each request snapshots its arguments at enqueue; later mutations cannot change it.
// Non-cloneable arguments reject with INVALID_REQUEST at their FIFO position.
export interface FormulaEngineClient {
  getProperty(id: PropertyId): Promise<PropertyState | null>;
  getProperties(): Promise<PropertyState[]>;
  getState(): Promise<FormulaEngineState>;
  // Rejects ACTIVE_DRAFTS until every Draft has been consumed or closed.
  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult>;
  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null>;
  // EVALUATE_INPUT rejects; formula and row errors remain in EvaluateResult.
  // Every row uses the caller's RuntimeContext; the Engine reads no system clock.
  evaluate(input: EvaluateInput): Promise<EvaluateResult>;
  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient>;
  // Rejects new calls immediately, drains queued calls, releases Drafts before
  // Engine, and terminates its Worker. Repeated close returns the same Promise.
  close(): Promise<void>;
}

export interface FormulaDraftClient {
  // Diagnostic IDs are opaque, stable for this version and scoped to this client.
  getState(): Promise<FormulaDraftState>;
  // UTF-16 cursor: floor inside a surrogate pair; clamp past the document end.
  help(cursor: number, config: CompletionConfig): Promise<CursorHelp>;
  // Foreign or stale diagnostic IDs return []; all queries still enter the FIFO.
  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]>;
  formatEdits(): Promise<FormulaEdit>;
  // Edits and cursor use the original source; returned cursor uses the new source.
  // Preserve the bigint base_version; stale edits reject with UPDATE_EXPRESSION.
  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult>;
  // Consumes this Draft without saving. Persist with Engine.upsert({ Formula: ... }).
  intoDefinition(): Promise<FormulaDefinition>;
  // Discards the Draft; idempotent, including after Engine.close().
  close(): Promise<void>;
}

export interface FormulaClientOptions {
  // Optional Worker injection; the client owns and terminates the returned Worker.
  workerFactory?: () => FormulaWorker;
}

// Implemented by createFormulaEngineClient in client.ts. Resolves after WASM and
// Engine initialization; initialization failure releases the Worker.
// Controlled rejections are FormulaClientError; error.data.code discriminates
// its typed payload. A Worker failure settles every pending call.
export type CreateFormulaEngineClient = (
  schema: FormulaSchema,
  options?: FormulaClientOptions,
) => Promise<FormulaEngineClient>;

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
  PropertyDefinition,
  PropertyId,
  PropertyState,
  QuickFix,
  UpdateExpressionResult,
} from "./generated/wasm_dto.js";
import type {
  CreateFormulaEngineClient,
  FormulaDraftClient,
  FormulaEngineClient,
} from "./client.h.js";
import {
  draftClosedError,
  engineClosedError,
  FormulaClientError,
  formulaErrorData,
} from "./errors.js";
import type { FormulaMethod, FormulaOperations } from "./protocol.js";
import { FormulaRpc } from "./rpc.js";

export * from "./generated/wasm_dto.js";
export type {
  CreateFormulaEngineClient,
  FormulaClientOptions,
  FormulaDraftClient,
  FormulaEngineClient,
} from "./client.h.js";
export { FormulaClientError } from "./errors.js";
export type { FormulaWorker } from "./rpc.js";
export { decodeFormulaString, quoteFormulaString } from "./string_codec.js";

/** Each client owns one module Worker; initialization failures release that Worker. */
export const createFormulaEngineClient: CreateFormulaEngineClient = async (
  schema,
  options = {},
) => {
  let rpc: FormulaRpc;
  try {
    rpc = new FormulaRpc(
      options.workerFactory?.() ??
        new Worker(new URL("./worker.js", import.meta.url), { type: "module" }),
    );
  } catch (error) {
    throw new FormulaClientError(formulaErrorData(error, "INITIALIZATION_ERROR"));
  }
  try {
    const engine = new EngineClient(rpc);
    await engine.enqueue("initialize", [schema]);
    return engine;
  } catch (error) {
    rpc.dispose();
    throw error instanceof FormulaClientError
      ? error
      : new FormulaClientError(formulaErrorData(error, "INITIALIZATION_ERROR"));
  }
};

class EngineClient implements FormulaEngineClient {
  private readonly diagnosticScope = `formula:${Array.from(
    crypto.getRandomValues(new Uint32Array(4)),
    (value) => value.toString(16).padStart(8, "0"),
  ).join("")}:`;
  private tail: Promise<void> = Promise.resolve();
  private closed = false;
  private closePromise: Promise<void> | null = null;

  constructor(private readonly rpc: FormulaRpc) {}

  getProperty(id: PropertyId): Promise<PropertyState | null> {
    return this.call("engine.getProperty", [id]);
  }

  getProperties(): Promise<PropertyState[]> {
    return this.call("engine.getProperties", []);
  }

  getState(): Promise<FormulaEngineState> {
    return this.call("engine.getState", []);
  }

  requiredInputs(formulaIds: PropertyId[]): Promise<PropertyId[]> {
    return this.call("engine.requiredInputs", [formulaIds]);
  }

  upsert(property: PropertyDefinition): Promise<FormulaEngineChangeResult> {
    return this.call("engine.upsert", [property]);
  }

  remove(id: PropertyId): Promise<FormulaEngineChangeResult | null> {
    return this.call("engine.remove", [id]);
  }

  evaluate(input: EvaluateInput): Promise<EvaluateResult> {
    return this.call("engine.evaluate", [input]);
  }

  createDraft(formula: FormulaDefinition): Promise<FormulaDraftClient> {
    return this.call("engine.createDraft", [formula]).then(
      (handle) => new DraftClient(this, handle),
    );
  }

  close(): Promise<void> {
    if (this.closePromise) return this.closePromise;
    this.closed = true;
    this.closePromise = this.enqueue("engine.close", [])
      .then(() => undefined)
      .finally(() => this.rpc.dispose());
    return this.closePromise;
  }

  rejection(): FormulaClientError | null {
    return this.closed ? engineClosedError() : null;
  }

  closing(): Promise<void> | null {
    return this.closePromise;
  }

  scopeState(state: FormulaDraftState): FormulaDraftState {
    return {
      ...state,
      diagnostics: state.diagnostics.map((diagnostic) => ({
        ...diagnostic,
        id: this.diagnosticScope + diagnostic.id,
      })),
    };
  }

  nativeDiagnosticId(id: DiagnosticId): DiagnosticId {
    if (typeof id !== "string") return id;
    return id.startsWith(this.diagnosticScope)
      ? id.slice(this.diagnosticScope.length)
      : `foreign:${id}`;
  }

  call<Method extends FormulaMethod>(
    method: Method,
    args: FormulaOperations[Method]["args"],
  ): Promise<FormulaOperations[Method]["result"]> {
    const error = this.rejection();
    return error ? Promise.reject(error) : this.enqueue(method, args);
  }

  enqueue<Method extends FormulaMethod>(
    method: Method,
    args: FormulaOperations[Method]["args"],
  ): Promise<FormulaOperations[Method]["result"]> {
    let result: Promise<FormulaOperations[Method]["result"]>;
    try {
      // Capture caller-owned DTOs now, before any earlier queued operation finishes.
      const snapshot = structuredClone(args);
      result = this.tail.then(() => this.rpc.request(method, snapshot));
    } catch (error) {
      const rejection = new FormulaClientError(formulaErrorData(error, "INVALID_REQUEST"));
      result = this.tail.then(() => {
        throw rejection;
      });
    }
    // A rejected call settles its own Promise but never prevents the next operation.
    this.tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }
}

class DraftClient implements FormulaDraftClient {
  private closed = false;
  private closePromise: Promise<void> | null = null;
  private definitionPromise: Promise<FormulaDefinition> | null = null;

  constructor(
    private readonly engine: EngineClient,
    private readonly handle: number,
  ) {}

  getState(): Promise<FormulaDraftState> {
    return this.call("draft.getState", [this.handle]).then((state) =>
      this.engine.scopeState(state),
    );
  }

  help(cursor: number, config: CompletionConfig): Promise<CursorHelp> {
    return this.call("draft.help", [this.handle, cursor, config]);
  }

  quickFixes(diagnosticId: DiagnosticId): Promise<QuickFix[]> {
    return this.call("draft.quickFixes", [
      this.handle,
      this.engine.nativeDiagnosticId(diagnosticId),
    ]);
  }

  formatEdits(): Promise<FormulaEdit> {
    return this.call("draft.formatEdits", [this.handle]);
  }

  updateExpression(update: ExpressionUpdate): Promise<UpdateExpressionResult> {
    return this.call("draft.updateExpression", [this.handle, update]).then((result) => ({
      ...result,
      state: this.engine.scopeState(result.state),
    }));
  }

  intoDefinition(): Promise<FormulaDefinition> {
    const error = this.rejection();
    if (error) return Promise.reject(error);
    this.closed = true;
    this.definitionPromise = this.engine.enqueue("draft.intoDefinition", [this.handle]);
    return this.definitionPromise;
  }

  close(): Promise<void> {
    if (this.closePromise) return this.closePromise;
    this.closed = true;
    this.closePromise = this.definitionPromise
      ? this.definitionPromise.then(() => undefined)
      : (this.engine.closing() ??
        this.engine.enqueue("draft.close", [this.handle]).then(() => undefined));
    return this.closePromise;
  }

  private rejection(): FormulaClientError | null {
    return this.engine.rejection() ?? (this.closed ? draftClosedError(this.handle) : null);
  }

  private call<Method extends FormulaMethod>(
    method: Method,
    args: FormulaOperations[Method]["args"],
  ): Promise<FormulaOperations[Method]["result"]> {
    const error = this.rejection();
    return error ? Promise.reject(error) : this.engine.enqueue(method, args);
  }
}

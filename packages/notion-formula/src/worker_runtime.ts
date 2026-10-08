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
import { formulaErrorData } from "./errors.js";
import type { FormulaRequest, FormulaResponse } from "./protocol.js";

/** Synchronous wasm-bindgen session boundary; all business validation stays in Rust. */
export interface FormulaSession {
  get_property(id: PropertyId): PropertyState | null;
  get_properties(): PropertyState[];
  get_state(): FormulaEngineState;
  required_inputs(formulaIds: PropertyId[]): PropertyId[];
  upsert(property: PropertyDefinition): FormulaEngineChangeResult;
  remove(id: PropertyId): FormulaEngineChangeResult | null;
  evaluate(input: EvaluateInput): EvaluateResult;
  create_draft(formula: FormulaDefinition): number;
  draft_state(handle: number): FormulaDraftState;
  draft_help(handle: number, cursor: number, config: CompletionConfig): CursorHelp;
  draft_quick_fixes(handle: number, diagnosticId: DiagnosticId): QuickFix[];
  draft_format_edits(handle: number): FormulaEdit;
  draft_update_expression(handle: number, update: ExpressionUpdate): UpdateExpressionResult;
  draft_into_definition(handle: number): FormulaDefinition;
  draft_close(handle: number): void;
  close(): void;
  free(): void;
}

export class FormulaWorkerRuntime {
  private session: FormulaSession | null = null;
  private tail: Promise<void> = Promise.resolve();

  constructor(private readonly initialize: (schema: FormulaSchema) => Promise<FormulaSession>) {}

  handle(request: FormulaRequest): Promise<FormulaResponse> {
    const response = this.tail.then(async (): Promise<FormulaResponse> => {
      try {
        return { id: request.id, ok: true, value: await this.dispatch(request) };
      } catch (error) {
        return {
          id: request.id,
          ok: false,
          error: formulaErrorData(
            error,
            request.method === "initialize" ? "INITIALIZATION_ERROR" : "WORKER_FAILURE",
          ),
        };
      }
    });
    this.tail = response.then(
      () => undefined,
      () => undefined,
    );
    return response;
  }

  private async dispatch(request: FormulaRequest): Promise<unknown> {
    if (request.method === "initialize") {
      if (this.session) throw new Error("Worker session is already initialized");
      this.session = await this.initialize(request.args[0]);
      return;
    }
    const session = this.session;
    if (!session) throw new Error("Worker session is not initialized");
    switch (request.method) {
      case "engine.getProperty":
        return session.get_property(...request.args);
      case "engine.getProperties":
        return session.get_properties();
      case "engine.getState":
        return session.get_state();
      case "engine.requiredInputs":
        return session.required_inputs(...request.args);
      case "engine.upsert":
        return session.upsert(...request.args);
      case "engine.remove":
        return session.remove(...request.args);
      case "engine.evaluate":
        return session.evaluate(...request.args);
      case "engine.createDraft":
        return session.create_draft(...request.args);
      case "draft.getState":
        return session.draft_state(...request.args);
      case "draft.help":
        return session.draft_help(...request.args);
      case "draft.quickFixes":
        return session.draft_quick_fixes(...request.args);
      case "draft.formatEdits":
        return session.draft_format_edits(...request.args);
      case "draft.updateExpression":
        return session.draft_update_expression(...request.args);
      case "draft.intoDefinition":
        return session.draft_into_definition(...request.args);
      case "draft.close":
        session.draft_close(...request.args);
        return;
      case "engine.close":
        try {
          session.close();
        } finally {
          session.free();
          this.session = null;
        }
        return;
    }
  }
}

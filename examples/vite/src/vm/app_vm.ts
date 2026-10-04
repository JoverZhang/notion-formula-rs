import { DEMO_SCHEMA, FORMULA_DEMOS } from "../app/context";
import { buildEvaluateInput } from "../app/data";
import {
  FORMULA_IDS,
  type AppState,
  type FormulaEditorActions,
  type FormulaId,
  type FormulaState,
} from "../app/types";
import {
  createFormulaEngineClient,
  FormulaClientError,
  type CreateFormulaEngineClient,
  type CursorHelp,
  type EvaluateInput,
  type FormulaDraftClient,
  type FormulaDraftState,
  type FormulaEdit,
  type FormulaEngineClient,
  type FormulaSchema,
  type QuickFix,
  type RuntimeContext,
  type TextEdit,
  type UpdateExpressionResult,
} from "../formula/client";

const DEBOUNCE_MS = 80;
const COMPLETION_CONFIG = { preferred_limit: 5 };

type VMOpts = {
  onStateChange: (state: AppState) => void;
  createEngine?: CreateFormulaEngineClient;
  runtime?: RuntimeContext;
  schema?: FormulaSchema;
  evaluateInput?: (runtime: RuntimeContext) => EvaluateInput;
};

type BufferSnapshot = { source: string; revision: number };
type DraftBinding = {
  client: FormulaDraftClient;
  source: string;
  state: FormulaDraftState | null;
};
type EditOrigin = {
  id: FormulaId;
  binding: DraftBinding;
  buffer: BufferSnapshot;
  version: bigint;
};

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Owns saved Engine definitions and separate, immediately editable source buffers. */
export class AppVM implements FormulaEditorActions {
  private readonly state: AppState;
  private readonly schema: FormulaSchema;
  private readonly createEngine: CreateFormulaEngineClient;
  private readonly evaluateInput: (runtime: RuntimeContext) => EvaluateInput;
  private readonly onStateChange: (state: AppState) => void;
  private readonly revisions = new Map<FormulaId, number>();
  private readonly drafts = new Map<FormulaId, DraftBinding>();
  private readonly timers = new Map<FormulaId, ReturnType<typeof setTimeout>>();
  private readonly editOrigins = new WeakMap<FormulaEdit | TextEdit, EditOrigin>();
  private engine: FormulaEngineClient | null = null;
  private tail: Promise<void> = Promise.resolve();
  private startPromise: Promise<void> | null = null;
  private disposePromise: Promise<void> | null = null;
  private disposed = false;

  constructor(opts: VMOpts) {
    this.schema = structuredClone(opts.schema ?? DEMO_SCHEMA);
    this.createEngine = opts.createEngine ?? createFormulaEngineClient;
    this.evaluateInput = opts.evaluateInput ?? buildEvaluateInput;
    this.onStateChange = opts.onStateChange;
    this.state = {
      wasmReady: false,
      saving: false,
      error: null,
      runtime: { ...(opts.runtime ?? { now: BigInt(Date.now()), time_zone: "+00:00" }) },
      evaluation: null,
      formulas: Object.fromEntries(
        FORMULA_IDS.map((id) => {
          const property = this.schema.properties.find(
            (property) => "Formula" in property && property.Formula.id === id,
          );
          const source =
            property && "Formula" in property
              ? property.Formula.expression
              : FORMULA_DEMOS[id].sample;
          this.revisions.set(id, 0);
          const formula: FormulaState = {
            id,
            source,
            savedSource: source,
            version: null,
            diagnostics: [],
            tokens: [],
            outputType: "Unknown",
            dirty: false,
            error: null,
            cursor: null,
            status: "wasm-not-ready",
          };
          return [id, formula];
        }),
      ) as Record<FormulaId, FormulaState>,
    };
  }

  start(): Promise<void> {
    if (this.startPromise) return this.startPromise;
    if (this.disposed) return Promise.resolve();
    this.startPromise = this.enqueue(async () => {
      if (this.disposed) return;
      this.emit();
      try {
        this.engine = await this.createEngine(this.schema);
        if (this.disposed) return;
        await this.evaluateSaved(this.engine);
        for (const id of FORMULA_IDS) {
          if (this.disposed) return;
          await this.openDraft(id, this.capture(id));
        }
        if (this.disposed) return;
        this.state.wasmReady = true;
        this.scheduleUnflushedBuffers();
        this.emit();
      } catch (error) {
        const engine = this.engine;
        this.engine = null;
        this.drafts.clear();
        this.state.wasmReady = false;
        this.state.evaluation = null;
        this.state.error = `Unable to start the formula engine: ${errorMessage(error)}`;
        if (engine) {
          try {
            await engine.close();
          } catch (closeError) {
            this.state.error += `; cleanup failed: ${errorMessage(closeError)}`;
          }
        }
        this.emit();
        throw error;
      }
    });
    return this.startPromise;
  }

  dispose(): Promise<void> {
    if (this.disposePromise) return this.disposePromise;
    this.disposed = true;
    for (const id of FORMULA_IDS) this.cancelTimer(id);
    this.disposePromise = this.enqueue(async () => {
      const engine = this.engine;
      this.engine = null;
      this.drafts.clear();
      await engine?.close();
    });
    return this.disposePromise;
  }

  setSource(id: FormulaId, source: string): void {
    if (this.disposed || this.state.formulas[id].source === source) return;
    this.resetBuffer(id, source);
    if (this.state.wasmReady) this.scheduleAnalyze(id);
    this.emit();
  }

  help(id: FormulaId, cursor: number): Promise<CursorHelp | null> {
    const buffer = this.capture(id);
    return this.command(id, buffer, "get cursor help", null, async () => {
      const binding = await this.currentDraft(id, buffer);
      if (!binding) return null;
      const help = await binding.client.help(cursor, COMPLETION_CONFIG);
      if (!this.isCurrent(id, binding, buffer)) return null;
      const origin: EditOrigin = { id, binding, buffer, version: help.base_version };
      for (const item of help.completion.items) {
        if (item.primary_edit) this.editOrigins.set(item.primary_edit, origin);
        for (const edit of item.additional_edits) this.editOrigins.set(edit, origin);
      }
      return help;
    });
  }

  format(id: FormulaId, cursor: number): Promise<UpdateExpressionResult | null> {
    const buffer = this.capture(id);
    return this.command(id, buffer, "format the formula", null, async () => {
      const binding = await this.currentDraft(id, buffer);
      if (!binding) return null;
      const edit = await binding.client.formatEdits();
      if (!this.isCurrent(id, binding, buffer)) return null;
      return this.performEdit(id, binding, buffer, edit, cursor);
    });
  }

  quickFixes(id: FormulaId, diagnosticId: string): Promise<QuickFix[]> {
    const buffer = this.capture(id);
    return this.command(id, buffer, "get quick fixes", [], async () => {
      const binding = await this.currentDraft(id, buffer);
      if (!binding) return [];
      const fixes = await binding.client.quickFixes(diagnosticId);
      if (!this.isCurrent(id, binding, buffer)) return [];
      for (const fix of fixes) {
        const origin: EditOrigin = { id, binding, buffer, version: fix.edit.base_version };
        this.editOrigins.set(fix.edit, origin);
        for (const edit of fix.edit.edits) this.editOrigins.set(edit, origin);
      }
      return fixes;
    });
  }

  /** Accepts native quick-fix edits or original TextEdit references returned by help. */
  applyEdit(
    id: FormulaId,
    edit: FormulaEdit,
    cursor: number,
  ): Promise<UpdateExpressionResult | null> {
    const buffer = this.capture(id);
    const origin = this.editOrigin(edit);
    // Versions restart at zero after reopening; the Draft identity must also match.
    if (!origin || origin.id !== id || origin.version !== edit.base_version) {
      return Promise.resolve(null);
    }
    const snapshot = structuredClone(edit);
    return this.command(id, buffer, "apply the edit", null, async () => {
      if (
        origin.buffer.revision !== buffer.revision ||
        !this.isCurrent(id, origin.binding, buffer)
      ) {
        return null;
      }
      await this.flush(id, origin.binding, buffer);
      if (!this.isCurrent(id, origin.binding, buffer)) return null;
      return this.performEdit(id, origin.binding, buffer, snapshot, cursor);
    });
  }

  save(id: FormulaId): Promise<void> {
    // Capture the click's source, even while earlier commands or later typing continue.
    const target = this.capture(id);
    return this.enqueue(async () => {
      if (this.disposed) return;
      const engine = this.engine;
      if (!engine || !this.state.wasmReady) {
        this.reportSaveError(id, "save the formula", new Error("The formula engine is not ready"));
        return;
      }
      this.state.saving = true;
      this.state.error = null;
      this.state.formulas[id].error = null;
      this.emit();
      let rebuild = false;
      let stage = "save the formula";
      try {
        const binding = await this.ensureDraft(id);
        await this.flush(id, binding, target);
        if (this.disposed) return;
        rebuild = true;
        this.drafts.delete(id);
        const definition = await binding.client.intoDefinition();
        if (this.disposed) return;
        await this.closeDrafts();
        if (this.disposed) return;
        await engine.upsert({ Formula: definition });
        if (this.disposed) return;
        const formula = this.state.formulas[id];
        formula.savedSource = definition.expression;
        formula.dirty = formula.source !== formula.savedSource;
        this.state.evaluation = null;
        this.emit();
        stage = "evaluate the saved formulas";
        await this.evaluateSaved(engine);
      } catch (error) {
        this.reportSaveError(id, stage, error);
      } finally {
        if (rebuild && !this.disposed) {
          try {
            await this.restoreDrafts();
          } catch (error) {
            this.reportSaveError(id, "restore the formula editors", error);
          }
        }
        if (!this.disposed) {
          this.state.saving = false;
          this.emit();
        }
      }
    });
  }

  discard(id: FormulaId): Promise<void> {
    const clicked = this.capture(id);
    return this.enqueue(async () => {
      if (this.disposed || !this.bufferMatches(id, clicked)) return;
      this.cancelTimer(id);
      this.resetBuffer(id, this.state.formulas[id].savedSource);
      const buffer = this.capture(id);
      this.emit();
      if (!this.engine || !this.state.wasmReady) return;
      const binding = this.drafts.get(id);
      this.drafts.delete(id);
      try {
        await binding?.client.close();
        if (this.disposed) return;
        await this.openDraft(id, buffer);
        this.scheduleUnflushedBuffers();
        this.emit();
      } catch (error) {
        this.reportFormulaError(id, buffer, "discard the draft", error);
      }
    });
  }

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    const result = this.tail.then(operation);
    this.tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }

  private command<T>(
    id: FormulaId,
    buffer: BufferSnapshot,
    action: string,
    fallback: T,
    operation: () => Promise<T>,
  ): Promise<T> {
    return this.enqueue(async () => {
      if (this.disposed || !this.state.wasmReady || !this.bufferMatches(id, buffer)) {
        return fallback;
      }
      try {
        return await operation();
      } catch (error) {
        this.reportFormulaError(id, buffer, action, error);
        return fallback;
      }
    });
  }

  private capture(id: FormulaId): BufferSnapshot {
    return { source: this.state.formulas[id].source, revision: this.revisions.get(id)! };
  }

  private bufferMatches(id: FormulaId, buffer: BufferSnapshot): boolean {
    return (
      this.revisions.get(id) === buffer.revision && this.state.formulas[id].source === buffer.source
    );
  }

  private isCurrent(id: FormulaId, binding: DraftBinding, buffer: BufferSnapshot): boolean {
    return !this.disposed && this.drafts.get(id) === binding && this.bufferMatches(id, buffer);
  }

  private resetBuffer(id: FormulaId, source: string): void {
    const formula = this.state.formulas[id];
    this.revisions.set(id, this.revisions.get(id)! + 1);
    formula.source = source;
    formula.dirty = source !== formula.savedSource;
    formula.version = null;
    formula.diagnostics = [];
    formula.tokens = [];
    formula.outputType = "Unknown";
    formula.cursor = null;
    formula.error = null;
    formula.status = this.state.wasmReady ? "analyzing" : "wasm-not-ready";
  }

  private acceptState(id: FormulaId, binding: DraftBinding, buffer: BufferSnapshot): void {
    const native = binding.state;
    if (!native || !this.isCurrent(id, binding, buffer) || binding.source !== buffer.source) return;
    const formula = this.state.formulas[id];
    formula.version = native.version;
    formula.diagnostics = native.diagnostics;
    formula.tokens = native.tokens;
    formula.outputType = native.output_type;
    formula.status = native.diagnostics.length ? "error" : "ok";
  }

  private async openDraft(id: FormulaId, buffer: BufferSnapshot): Promise<DraftBinding> {
    const client = await this.engine!.createDraft({ id, expression: buffer.source });
    const binding: DraftBinding = { client, source: buffer.source, state: null };
    this.drafts.set(id, binding);
    if (this.disposed) return binding;
    binding.state = await client.getState();
    binding.source = binding.state.definition.expression;
    this.acceptState(id, binding, buffer);
    return binding;
  }

  private ensureDraft(id: FormulaId): Promise<DraftBinding> {
    const binding = this.drafts.get(id);
    return binding ? Promise.resolve(binding) : this.openDraft(id, this.capture(id));
  }

  private async currentDraft(id: FormulaId, buffer: BufferSnapshot): Promise<DraftBinding | null> {
    const binding = await this.ensureDraft(id);
    if (!this.isCurrent(id, binding, buffer)) return null;
    await this.flush(id, binding, buffer);
    return this.isCurrent(id, binding, buffer) ? binding : null;
  }

  private async flush(id: FormulaId, binding: DraftBinding, buffer: BufferSnapshot): Promise<void> {
    this.cancelTimer(id);
    if (binding.source !== buffer.source) {
      const result = await binding.client.updateExpression({ Replace: buffer.source });
      binding.state = result.state;
    } else if (!binding.state) {
      binding.state = await binding.client.getState();
    }
    binding.source = binding.state.definition.expression;
    if (this.isCurrent(id, binding, buffer)) {
      this.state.formulas[id].error = null;
      this.acceptState(id, binding, buffer);
      this.emit();
    }
  }

  private async performEdit(
    id: FormulaId,
    binding: DraftBinding,
    buffer: BufferSnapshot,
    edit: FormulaEdit,
    cursor: number,
  ): Promise<UpdateExpressionResult | null> {
    let result: UpdateExpressionResult;
    try {
      result = await binding.client.updateExpression({ Edits: { edit, cursor } });
    } catch (error) {
      if (
        error instanceof FormulaClientError &&
        error.data.code === "UPDATE_EXPRESSION" &&
        error.data.payload.error === "VersionMismatch"
      ) {
        return null;
      }
      throw error;
    }
    binding.state = result.state;
    binding.source = result.state.definition.expression;
    if (!this.isCurrent(id, binding, buffer)) return null;
    this.cancelTimer(id);
    this.resetBuffer(id, binding.source);
    this.state.formulas[id].cursor = result.cursor;
    this.acceptState(id, binding, this.capture(id));
    this.emit();
    return result;
  }

  private editOrigin(edit: FormulaEdit): EditOrigin | null {
    const direct = this.editOrigins.get(edit);
    if (direct) return direct;
    const origins = edit.edits.map((textEdit) => this.editOrigins.get(textEdit));
    const first = origins[0];
    return first && origins.every((origin) => origin === first) ? first : null;
  }

  private async evaluateSaved(engine: FormulaEngineClient): Promise<void> {
    try {
      const result = await engine.evaluate(this.evaluateInput({ ...this.state.runtime }));
      if (!this.disposed) this.state.evaluation = result;
    } catch (error) {
      this.state.evaluation = null;
      throw error;
    }
  }

  private async closeDrafts(): Promise<void> {
    const failures: string[] = [];
    for (const [id, binding] of this.drafts) {
      this.drafts.delete(id);
      try {
        await binding.client.close();
      } catch (error) {
        failures.push(`${id}: ${errorMessage(error)}`);
      }
    }
    if (failures.length) throw new Error(failures.join("; "));
  }

  private async restoreDrafts(): Promise<void> {
    await this.closeDrafts();
    const failures: string[] = [];
    for (const id of FORMULA_IDS) {
      if (this.disposed) return;
      this.cancelTimer(id);
      const buffer = this.capture(id);
      try {
        await this.openDraft(id, buffer);
      } catch (error) {
        this.reportFormulaError(id, buffer, "restore the draft", error);
        failures.push(`${id}: ${errorMessage(error)}`);
      }
    }
    this.scheduleUnflushedBuffers();
    if (failures.length) throw new Error(failures.join("; "));
  }

  private cancelTimer(id: FormulaId): void {
    const timer = this.timers.get(id);
    if (timer !== undefined) clearTimeout(timer);
    this.timers.delete(id);
  }

  private scheduleUnflushedBuffers(): void {
    for (const id of FORMULA_IDS) {
      if (this.drafts.get(id)?.source !== this.state.formulas[id].source) {
        this.scheduleAnalyze(id);
      }
    }
  }

  private scheduleAnalyze(id: FormulaId): void {
    this.cancelTimer(id);
    this.timers.set(
      id,
      setTimeout(() => {
        this.timers.delete(id);
        const buffer = this.capture(id);
        void this.command(id, buffer, "analyze the formula", undefined, async () => {
          await this.currentDraft(id, buffer);
        });
      }, DEBOUNCE_MS),
    );
  }

  private reportFormulaError(
    id: FormulaId,
    buffer: BufferSnapshot,
    action: string,
    error: unknown,
  ): void {
    if (this.disposed || !this.bufferMatches(id, buffer)) return;
    const formula = this.state.formulas[id];
    formula.error = `Unable to ${action}: ${errorMessage(error)}`;
    formula.status = "error";
    this.emit();
  }

  private reportSaveError(id: FormulaId, action: string, error: unknown): void {
    if (this.disposed) return;
    const message = `Unable to ${action}: ${errorMessage(error)}`;
    this.state.error = this.state.error ? `${this.state.error}; ${message}` : message;
    this.state.formulas[id].error = message;
    this.emit();
  }

  private emit(): void {
    if (!this.disposed) this.onStateChange(structuredClone(this.state));
  }
}

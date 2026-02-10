import { analyzeSource, initWasm } from "../analyzer/wasm_client";
import {
  FORMULA_IDS,
  type AppState,
  type AnalyzerDiagnostic,
  type FormulaId,
  type FormulaState,
} from "../app/types";

const DEBOUNCE_MS = 80;

type VMOpts = {
  contextJson: string;
  onStateChange: (state: AppState) => void;
};

export class AppVM {
  private state: AppState;
  private onStateChange: (state: AppState) => void;
  private timers = new Map<FormulaId, ReturnType<typeof setTimeout>>();

  constructor(opts: VMOpts) {
    this.state = {
      wasmReady: false,
      contextJson: opts.contextJson,
      formulas: this.createFormulas(),
    };
    this.onStateChange = opts.onStateChange;
  }

  async start(): Promise<void> {
    await initWasm();
    this.state.wasmReady = true;
    for (const formula of Object.values(this.state.formulas)) {
      if (formula.source) {
        this.scheduleAnalyze(formula.id);
      }
    }
    this.onStateChange(this.state);
  }

  setSource(id: FormulaId, source: string): void {
    const formula = this.state.formulas[id];
    if (!formula) return;

    formula.source = source;
    if (!this.state.wasmReady) {
      formula.status = "wasm-not-ready";
      this.onStateChange(this.state);
      return;
    }

    this.scheduleAnalyze(id);
    this.onStateChange(this.state);
  }

  private createFormula(id: FormulaId): FormulaState {
    return {
      id,
      source: "",
      diagnostics: [],
      tokens: [],
      formatted: "",
      outputType: "unknown",
      status: "idle",
    };
  }

  private createFormulas(): Record<FormulaId, FormulaState> {
    return Object.fromEntries(FORMULA_IDS.map((id) => [id, this.createFormula(id)])) as Record<
      FormulaId,
      FormulaState
    >;
  }

  private scheduleAnalyze(id: FormulaId): void {
    const existing = this.timers.get(id);
    if (existing) {
      clearTimeout(existing);
    }

    const timer = setTimeout(() => {
      this.timers.delete(id);
      void this.runAnalyze(id);
    }, DEBOUNCE_MS);

    this.timers.set(id, timer);
  }

  private runAnalyze(id: FormulaId): void {
    const formula = this.state.formulas[id];
    if (!formula || !this.state.wasmReady) {
      return;
    }

    formula.status = "analyzing";
    this.onStateChange(this.state);

    try {
      const result = analyzeSource(formula.source, this.state.contextJson);
      formula.diagnostics = result.diagnostics || [];
      formula.tokens = result.tokens || [];
      formula.formatted = result.formatted || "";
      formula.outputType = result.output_type || "unknown";
      formula.status = "ok";
    } catch {
      const diag: AnalyzerDiagnostic = {
        kind: "error",
        message: "analysis failed",
        span: { range: { start: 0, end: 0 } },
      };
      formula.diagnostics = [diag];
      formula.tokens = [];
      formula.formatted = "";
      formula.outputType = "unknown";
      formula.status = "error";
    }

    this.onStateChange(this.state);
  }
}

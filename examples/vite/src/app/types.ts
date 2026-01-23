import type { Token } from "../editor_decorations";
import type { DiagnosticView } from "../analyzer/generated/wasm_dto";

export type FormulaId = "f1" | "f2" | "f3";

export type AnalyzerDiagnostic = DiagnosticView;

export type FormulaState = {
  id: FormulaId;
  source: string;
  diagnostics: AnalyzerDiagnostic[];
  tokens: Token[];
  formatted: string;
  status: "idle" | "wasm-not-ready" | "analyzing" | "ok" | "error";
};

export type AppState = {
  wasmReady: boolean;
  contextJson: string;
  formulas: Record<FormulaId, FormulaState>;
};

import type { DiagnosticView } from "../analyzer/generated/wasm_dto";
import type { Token } from "../editor_decorations";

export const FORMULA_IDS = ["f1", "f2"] as const;
export type FormulaId = (typeof FORMULA_IDS)[number];

export type AnalyzerDiagnostic = DiagnosticView;

export type FormulaState = {
  id: FormulaId;
  source: string;
  diagnostics: AnalyzerDiagnostic[];
  tokens: Token[];
  formatted: string;
  outputType: string;
  status: "idle" | "wasm-not-ready" | "analyzing" | "ok" | "error";
};

export type AppState = {
  wasmReady: boolean;
  contextJson: string;
  formulas: Record<FormulaId, FormulaState>;
};

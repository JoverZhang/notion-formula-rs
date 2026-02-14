import type { AnalyzerConfig, Diagnostic } from "../analyzer/generated/wasm_dto";
import type { Token } from "../editor_decorations";

export const FORMULA_IDS = ["f1", "f2"] as const;
export type FormulaId = (typeof FORMULA_IDS)[number];

export type AnalyzerDiagnostic = Diagnostic;

export type FormulaState = {
  id: FormulaId;
  source: string;
  diagnostics: AnalyzerDiagnostic[];
  tokens: Token[];
  outputType: string;
  status: "idle" | "wasm-not-ready" | "analyzing" | "ok" | "error";
};

export type AppState = {
  wasmReady: boolean;
  analyzerConfig: AnalyzerConfig;
  formulas: Record<FormulaId, FormulaState>;
};

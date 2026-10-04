import type {
  CursorHelp,
  DraftVersion,
  EvaluateResult,
  ExpressionDiagnostic,
  FormulaEdit,
  QuickFix,
  RuntimeContext,
  Token,
  UpdateExpressionResult,
  ValueType,
} from "@notion-formula/sdk";

export const FORMULA_IDS = ["Formula 1", "Formula 2"] as const;
export type FormulaId = (typeof FORMULA_IDS)[number];
export type FormulaDiagnostic = ExpressionDiagnostic;

export type FormulaState = {
  id: FormulaId;
  source: string;
  savedSource: string;
  version: DraftVersion | null;
  diagnostics: FormulaDiagnostic[];
  tokens: Token[];
  outputType: ValueType;
  dirty: boolean;
  error: string | null;
  cursor: number | null;
  status: "idle" | "wasm-not-ready" | "analyzing" | "ok" | "error";
};

export type AppState = {
  wasmReady: boolean;
  saving: boolean;
  error: string | null;
  runtime: RuntimeContext;
  evaluation: EvaluateResult | null;
  formulas: Record<FormulaId, FormulaState>;
};

// Editor commands use raw UTF-16 offsets; Rust validates and applies versioned edits.
export interface FormulaEditorActions {
  setSource(id: FormulaId, source: string): void;
  help(id: FormulaId, cursor: number): Promise<CursorHelp | null>;
  format(id: FormulaId, cursor: number): Promise<UpdateExpressionResult | null>;
  quickFixes(id: FormulaId, diagnosticId: string): Promise<QuickFix[]>;
  // Use edits returned by this editor's current help/quick-fix result, retaining
  // their TextEdit objects. Draft recreation can restart versions at zero.
  applyEdit(
    id: FormulaId,
    edit: FormulaEdit,
    cursor: number,
  ): Promise<UpdateExpressionResult | null>;
  save(id: FormulaId): Promise<void>;
  discard(id: FormulaId): Promise<void>;
}

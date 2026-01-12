import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { AnalyzerDiagnostic, FormulaId } from "../app/types";
import type { ChipSpan } from "../chip_spans";
import type { TokenDecorationRange } from "../editor_decorations";

export type DebugTokenDeco = { from: number; to: number; className: string };
type CmSeverity = NonNullable<CmDiagnostic["severity"]>;
export type DebugCmDiag = {
  from: number;
  to: number;
  severity: CmSeverity;
  message: string;
};

export interface NfDebug {
  listPanels(): FormulaId[];
  getState(id: FormulaId): {
    source: string;
    formatted: string;
    diagnosticsCount: number;
    tokenCount: number;
    status: string;
  };
  getSelectionHead(id: FormulaId): number;
  getAnalyzerDiagnostics(id: FormulaId): AnalyzerDiagnostic[];
  getCmDiagnostics(id: FormulaId): DebugCmDiag[];
  getTokenDecorations(id: FormulaId): DebugTokenDeco[];
  getChipSpans(id: FormulaId): { start: number; end: number }[];
  toChipPos(id: FormulaId, rawUtf16Pos: number): number;
  toRawPos(id: FormulaId, chipPos: number): number;
}

declare global {
  interface Window {
    __nf_debug?: NfDebug;
  }
}

export type PanelDebugHandle = {
  getState(): {
    source: string;
    formatted: string;
    diagnosticsCount: number;
    tokenCount: number;
    status: string;
  };
  getSelectionHead(): number;
  getAnalyzerDiagnostics(): AnalyzerDiagnostic[];
  getCmDiagnostics(): CmDiagnostic[];
  getTokenDecorations(): TokenDecorationRange[];
  getChipSpans(): ChipSpan[];
  toChipPos(rawUtf16Pos: number): number;
  toRawPos(chipPos: number): number;
};

export function getWindow(): Window | undefined {
  const w = (globalThis as unknown as { window?: unknown }).window;
  return typeof w === "object" && w !== null ? (w as Window) : undefined;
}

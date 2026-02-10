import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { AnalyzerDiagnostic, FormulaId } from "../app/types";
import type { ChipSpan } from "../chip_spans";
import type { ChipDecorationRange } from "../editor/chip_decorations";
import type { TokenDecorationRange } from "../editor_decorations";

type DebugState = {
  source: string;
  formatted: string;
  outputType: string;
  diagnosticsCount: number;
  tokenCount: number;
};

export interface NfDebug {
  listPanels(): FormulaId[];
  getState(id: FormulaId): DebugState;
  getSelectionHead(id: FormulaId): number;
  getAnalyzerDiagnostics(id: FormulaId): AnalyzerDiagnostic[];
  getCmDiagnostics(id: FormulaId): CmDiagnostic[];
  getTokenDecorations(id: FormulaId): TokenDecorationRange[];
  getChipSpans(id: FormulaId): { start: number; end: number }[];
  getChipUiRanges(id: FormulaId): ChipDecorationRange[];
  toChipPos(id: FormulaId, rawUtf16Pos: number): number;
  toRawPos(id: FormulaId, chipPos: number): number;
  setSelectionHead(id: FormulaId, pos: number): void;
  isChipUiEnabled(): boolean;
  getChipUiCount(id: FormulaId): number;
}

declare global {
  interface Window {
    __nf_debug?: NfDebug;
  }
}

export type PanelDebugHandle = {
  getState(): DebugState;
  getSelectionHead(): number;
  getAnalyzerDiagnostics(): AnalyzerDiagnostic[];
  getCmDiagnostics(): CmDiagnostic[];
  getTokenDecorations(): TokenDecorationRange[];
  getChipSpans(): ChipSpan[];
  getChipUiRanges(): ChipDecorationRange[];
  toChipPos(rawUtf16Pos: number): number;
  toRawPos(chipPos: number): number;
  setSelectionHead(pos: number): void;
  isChipUiEnabled(): boolean;
  getChipUiCount(): number;
};

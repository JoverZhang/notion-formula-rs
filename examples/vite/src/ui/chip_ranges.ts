import type { AnalyzerDiagnostic } from "../app/types";
import { normalizeDiagRange } from "./diag_range";

export type ChipUiRange = {
  from: number;
  to: number;
  propName: string;
  hasError: boolean;
  hasWarning: boolean;
  message?: string;
};

function chipIntersects(
  range: { from: number; to: number },
  diagRange: { from: number; to: number },
): boolean {
  return diagRange.from < range.to && diagRange.to > range.from;
}

export function mergeChipRangesWithDiagnostics(
  chipRanges: Array<{ from: number; to: number; propName: string }>,
  diagnostics: AnalyzerDiagnostic[],
  docLen: number,
): ChipUiRange[] {
  if (chipRanges.length === 0) return [];
  if (diagnostics.length === 0) {
    return chipRanges.map((range) => ({
      ...range,
      hasError: false,
      hasWarning: false,
      message: undefined,
    }));
  }

  return chipRanges.map((range) => {
    let hasError = false;
    let message: string | undefined;
    for (const diag of diagnostics) {
      const diagRange = normalizeDiagRange(diag, docLen);
      if (!diagRange || !chipIntersects(range, diagRange)) continue;
      if (!hasError) message = diag.message;
      hasError = true;
    }
    return {
      ...range,
      hasError,
      hasWarning: false,
      message,
    };
  });
}

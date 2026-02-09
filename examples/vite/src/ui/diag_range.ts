import type { AnalyzerDiagnostic } from "../app/types";
import type { ChipSpan } from "../chip_spans";

export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function normalizeDiagRange(
  diag: AnalyzerDiagnostic,
  docLen: number,
): { from: number; to: number } | null {
  const start = diag.span?.range?.start;
  if (typeof start !== "number") return null;
  const end = diag.span?.range?.end;
  const from = clamp(start, 0, docLen);
  const toRaw = typeof end === "number" ? end : start + 1;
  const to = clamp(Math.max(toRaw, from + 1), 0, docLen);
  return { from, to };
}

export function remapRangeToChip(
  range: { from: number; to: number },
  chipSpans: ChipSpan[],
): { from: number; to: number } {
  for (const span of chipSpans) {
    if (range.from < span.end && range.to > span.start) {
      return { from: span.start, to: span.end };
    }
  }
  return range;
}

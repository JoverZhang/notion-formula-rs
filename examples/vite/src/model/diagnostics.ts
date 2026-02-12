import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { AnalyzerDiagnostic } from "../app/types";
import type { ChipOffsetMap, ChipSpan } from "../chip_spans";

type Range = { from: number; to: number };

export type ChipUiRange = {
  from: number;
  to: number;
  propName: string;
  hasError: boolean;
  hasWarning: boolean;
  message?: string;
};

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function intersects(a: Range, b: Range): boolean {
  return a.from < b.to && a.to > b.from;
}

function remapRangeToChip(range: Range, chipSpans: ChipSpan[]): Range {
  for (const span of chipSpans) {
    if (range.from < span.end && range.to > span.start) {
      return { from: span.start, to: span.end };
    }
  }
  return range;
}

function toCmSeverity(kind?: string): "error" | "warning" | "info" {
  if (kind === "warning") return "warning";
  if (kind === "info") return "info";
  return "error";
}

export function normalizeDiagRange(diag: AnalyzerDiagnostic, docLen: number): Range | null {
  const start = diag.span?.start;
  if (typeof start !== "number") return null;
  const end = diag.span?.end;
  const from = clamp(start, 0, docLen);
  const rawTo = typeof end === "number" ? end : start + 1;
  return { from, to: clamp(Math.max(rawTo, from + 1), 0, docLen) };
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
      if (!diagRange || !intersects(range, diagRange)) continue;
      if (!message) message = diag.message;
      hasError = true;
    }
    return { ...range, hasError, hasWarning: false, message };
  });
}

export function toCmDiagnostics(
  diagnostics: AnalyzerDiagnostic[],
  docLen: number,
  chipSpans: ChipSpan[] = [],
): CmDiagnostic[] {
  const out: CmDiagnostic[] = [];
  for (const diag of diagnostics) {
    const range = normalizeDiagRange(diag, docLen);
    if (!range) continue;
    const remapped = remapRangeToChip(range, chipSpans);
    const from = clamp(remapped.from, 0, docLen);
    const to = clamp(Math.max(remapped.to, from + 1), 0, docLen);
    out.push({
      from,
      to,
      severity: toCmSeverity(diag.kind),
      message: diag.message || "(no message)",
    });
  }
  return out;
}

function chipPosLabel(
  diag: AnalyzerDiagnostic,
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
): string | null {
  if (!chipMap) return null;
  const range = normalizeDiagRange(diag, Number.MAX_SAFE_INTEGER);
  if (!range) return null;
  for (const span of chipSpans) {
    if (!intersects(range, { from: span.start, to: span.end })) continue;
    const chipStart = chipMap.toChipPos(span.start);
    return `chipPos=[${chipStart},${chipStart + 1})`;
  }
  const from = chipMap.toChipPos(range.from);
  const to = chipMap.toChipPos(range.to);
  return `chipPos=[${from},${Math.max(to, from + 1)})`;
}

export function buildDiagnosticTextRows(
  _source: string,
  diagnostics: AnalyzerDiagnostic[],
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
): string[] {
  if (!diagnostics.length) return ["No diagnostics"];
  return diagnostics.map((diag) => {
    const kind = diag.kind || "error";
    const lineColLabel = ` ${diag.line}:${diag.col}`;
    const chipLabel = chipPosLabel(diag, chipMap, chipSpans);
    return chipLabel
      ? `${kind}${lineColLabel}: ${diag.message} ${chipLabel}`
      : `${kind}${lineColLabel}: ${diag.message}`;
  });
}

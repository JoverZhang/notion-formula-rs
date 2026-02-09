import type { Diagnostic as CmDiagnostic } from "@codemirror/lint";
import type { AnalyzerDiagnostic } from "../app/types";
import type { ChipSpan } from "../chip_spans";
import { clamp, normalizeDiagRange, remapRangeToChip } from "./diag_range";

function toCmSeverity(kind?: string): "error" | "warning" | "info" {
  if (kind === "warning") return "warning";
  if (kind === "info") return "info";
  return "error";
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

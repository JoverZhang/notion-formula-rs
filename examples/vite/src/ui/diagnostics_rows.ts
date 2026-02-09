import { posToLineCol } from "../analyzer/wasm_client";
import type { AnalyzerDiagnostic } from "../app/types";
import type { ChipOffsetMap, ChipSpan } from "../chip_spans";
import { normalizeDiagRange } from "./diag_range";

function buildChipPosLabel(
  diag: AnalyzerDiagnostic,
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
): string | null {
  if (!chipMap) return null;
  const range = normalizeDiagRange(diag, Number.MAX_SAFE_INTEGER);
  if (!range) return null;

  for (const span of chipSpans) {
    if (range.from < span.end && range.to > span.start) {
      const chipStart = chipMap.toChipPos(span.start);
      return `chipPos=[${chipStart},${chipStart + 1})`;
    }
  }

  const chipStart = chipMap.toChipPos(range.from);
  const chipEnd = chipMap.toChipPos(range.to);
  return `chipPos=[${chipStart},${Math.max(chipEnd, chipStart + 1)})`;
}

export function buildDiagnosticTextRows(
  source: string,
  diagnostics: AnalyzerDiagnostic[],
  chipMap: ChipOffsetMap | null,
  chipSpans: ChipSpan[],
): string[] {
  if (!diagnostics.length) {
    return ["No diagnostics"];
  }

  return diagnostics.map((diag) => {
    const kind = diag.kind || "error";
    const start = diag.span?.range?.start;
    let line = 0;
    let col = 0;
    if (typeof start === "number") {
      try {
        const lc = posToLineCol(source, start);
        line = lc.line;
        col = lc.col;
      } catch {
        line = 0;
        col = 0;
      }
    }

    const chipLabel = buildChipPosLabel(diag, chipMap, chipSpans);
    const posLabel = chipLabel ? `${chipLabel} line=${line} col=${col}` : `line=${line} col=${col}`;
    return `${kind}: ${diag.message} ${posLabel}`;
  });
}

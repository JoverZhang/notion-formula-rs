import { computePropChips, type Token } from "./editor_decorations";

export type ChipSpan = {
  start: number;
  end: number;
};

export type ChipOffsetMap = {
  toChipPos: (rawUtf16Pos: number) => number;
  toRawPos: (chipPos: number) => number;
};

export function computeChipSpans(source: string, tokens: Token[]): ChipSpan[] {
  const chips = computePropChips(source, tokens);
  const spans = chips.map((chip) => ({
    start: chip.spanStart,
    end: chip.spanEnd,
  }));
  return spans.sort((a, b) => a.start - b.start || a.end - b.end);
}

function formatSpan(span: ChipSpan): string {
  return `{start:${span.start},end:${span.end}}`;
}

export function buildChipOffsetMap(docLen: number, chipSpans: ChipSpan[]): ChipOffsetMap {
  let prevSpan: ChipSpan | null = null;
  for (const span of chipSpans) {
    if (span.start < 0 || span.end < 0 || span.end > docLen || span.start >= span.end) {
      throw new Error(`Invalid chip span range ${formatSpan(span)} for docLen=${docLen}`);
    }
    if (prevSpan) {
      if (span.start < prevSpan.start || (span.start === prevSpan.start && span.end < prevSpan.end)) {
        throw new Error(
          `Chip spans must be sorted by start/end: prev=${formatSpan(prevSpan)} next=${formatSpan(span)}`,
        );
      }
      if (span.start < prevSpan.end) {
        throw new Error(`Chip spans overlap: prev=${formatSpan(prevSpan)} next=${formatSpan(span)}`);
      }
    }
    prevSpan = span;
  }

  let totalCompression = 0;
  for (const span of chipSpans) {
    totalCompression += span.end - span.start - 1;
  }
  const chipDocLen = docLen - totalCompression;

  const toChipPos = (rawUtf16Pos: number): number => {
    let pos = rawUtf16Pos;
    if (pos < 0) pos = 0;
    if (pos > docLen) pos = docLen;

    let compression = 0;
    for (const span of chipSpans) {
      if (pos < span.start) {
        break;
      }
      if (pos > span.start && pos < span.end) {
        return span.start - compression;
      }
      if (pos >= span.end) {
        compression += span.end - span.start - 1;
        continue;
      }
      return span.start - compression;
    }
    return pos - compression;
  };

  const toRawPos = (chipPos: number): number => {
    let pos = chipPos;
    if (pos < 0) pos = 0;
    if (pos > chipDocLen) pos = chipDocLen;

    let compression = 0;
    for (const span of chipSpans) {
      const chipStart = span.start - compression;
      const chipEnd = chipStart + 1;
      if (pos < chipStart) {
        break;
      }
      if (pos === chipStart) {
        return span.start;
      }
      if (pos === chipEnd) {
        return span.end;
      }
      compression += span.end - span.start - 1;
    }
    return pos + compression;
  };

  return { toChipPos, toRawPos };
}

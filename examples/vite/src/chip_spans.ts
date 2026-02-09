export type ChipSpan = {
  start: number;
  end: number;
};

export type ChipOffsetMap = {
  toChipPos: (rawUtf16Pos: number) => number;
  toRawPos: (chipPos: number) => number;
};

type SpanEntry = {
  start: number;
  end: number;
  compressionBefore: number;
  chipStart: number;
};

export function buildChipOffsetMap(docLen: number, chipSpans: ChipSpan[]): ChipOffsetMap {
  const spans: SpanEntry[] = [];
  let prev: ChipSpan | null = null;
  let compression = 0;
  for (const span of chipSpans) {
    if (span.start < 0 || span.end < 0 || span.end > docLen || span.start >= span.end) {
      throw new Error(
        `Invalid chip span range {start:${span.start},end:${span.end}} for docLen=${docLen}`,
      );
    }
    if (prev) {
      if (span.start < prev.start || (span.start === prev.start && span.end < prev.end)) {
        throw new Error("Chip spans must be sorted by start/end");
      }
      if (span.start < prev.end) {
        throw new Error("Chip spans overlap");
      }
    }
    spans.push({
      start: span.start,
      end: span.end,
      compressionBefore: compression,
      chipStart: span.start - compression,
    });
    compression += span.end - span.start - 1;
    prev = span;
  }
  const chipDocLen = docLen - compression;

  const toChipPos = (rawUtf16Pos: number): number => {
    const pos = Math.max(0, Math.min(docLen, rawUtf16Pos));
    let currentCompression = 0;
    for (const span of spans) {
      if (pos < span.start) break;
      if (pos < span.end) return span.chipStart;
      currentCompression = span.compressionBefore + (span.end - span.start - 1);
    }
    return pos - currentCompression;
  };

  const toRawPos = (chipPos: number): number => {
    const pos = Math.max(0, Math.min(chipDocLen, chipPos));
    let currentCompression = 0;
    for (const span of spans) {
      const chipStart = span.chipStart;
      if (pos < chipStart) break;
      if (pos === chipStart) return span.start;
      if (pos === chipStart + 1) return span.end;
      currentCompression = span.compressionBefore + (span.end - span.start - 1);
    }
    return pos + currentCompression;
  };

  return { toChipPos, toRawPos };
}

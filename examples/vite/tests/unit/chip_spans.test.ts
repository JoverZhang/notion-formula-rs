import { describe, expect, it } from "vitest";
import { buildChipOffsetMap, type ChipSpan } from "../../src/chip_spans";

describe("buildChipOffsetMap", () => {
  it("maps identity when there are no chips", () => {
    const source = "a + b";
    const map = buildChipOffsetMap(source.length, []);
    expect(map.toChipPos(0)).toBe(0);
    expect(map.toChipPos(2)).toBe(2);
    expect(map.toChipPos(source.length)).toBe(source.length);
    expect(map.toRawPos(0)).toBe(0);
    expect(map.toRawPos(2)).toBe(2);
    expect(map.toRawPos(source.length)).toBe(source.length);
  });

  it("compresses a single chip span", () => {
    const source = 'prop("Title") + 1';
    const chipSpans: ChipSpan[] = [{ start: 0, end: 13 }];
    const map = buildChipOffsetMap(source.length, chipSpans);

    for (let i = 0; i < 13; i += 1) {
      expect(map.toChipPos(i)).toBe(0);
    }
    expect(map.toChipPos(13)).toBe(1);
    expect(map.toChipPos(14)).toBe(2);
    expect(map.toChipPos(16)).toBe(4);
    expect(map.toChipPos(17)).toBe(5);

    const chipDocLen = source.length - (13 - 1);
    let lastRaw = -1;
    for (let pos = 0; pos <= chipDocLen; pos += 1) {
      const raw = map.toRawPos(pos);
      expect(raw).toBeGreaterThanOrEqual(lastRaw);
      lastRaw = raw;
    }
  });

  it("maps single chip boundaries exactly", () => {
    const source = 'prop("Title")';
    const chipSpans: ChipSpan[] = [{ start: 0, end: 13 }];
    const map = buildChipOffsetMap(source.length, chipSpans);

    for (let i = 0; i < 13; i += 1) {
      expect(map.toChipPos(i)).toBe(0);
    }
    expect(map.toChipPos(13)).toBe(1);

    expect(map.toRawPos(0)).toBe(0);
    expect(map.toRawPos(1)).toBe(13);
  });

  it("single chip roundtrip boundaries", () => {
    const source = 'prop("Title")';
    const chipSpans: ChipSpan[] = [{ start: 0, end: 13 }];
    const map = buildChipOffsetMap(source.length, chipSpans);

    expect(map.toRawPos(map.toChipPos(0))).toBe(0);
    expect(map.toRawPos(map.toChipPos(12))).toBe(0);
    expect(map.toRawPos(map.toChipPos(13))).toBe(13);

    expect(map.toChipPos(map.toRawPos(0))).toBe(0);
    expect(map.toChipPos(map.toRawPos(1))).toBe(1);
  });

  it("maps positions around multiple chips", () => {
    const source = 'a + prop("X") + prop("Y")';
    const chipSpans: ChipSpan[] = [
      { start: 4, end: 13 },
      { start: 16, end: 25 },
    ];
    const map = buildChipOffsetMap(source.length, chipSpans);

    expect(map.toChipPos(0)).toBe(0);
    expect(map.toChipPos(4)).toBe(4);
    expect(map.toChipPos(5)).toBe(4);
    expect(map.toChipPos(13)).toBe(5);
    expect(map.toChipPos(16)).toBe(8);
    expect(map.toChipPos(17)).toBe(8);
    expect(map.toChipPos(25)).toBe(9);
  });

  it("multi chip roundtrip boundaries", () => {
    const source = 'a + prop("X") + prop("Y")';
    const chipSpans: ChipSpan[] = [
      { start: 4, end: 13 },
      { start: 16, end: 25 },
    ];
    const map = buildChipOffsetMap(source.length, chipSpans);

    expect(map.toRawPos(map.toChipPos(4))).toBe(4);
    expect(map.toRawPos(map.toChipPos(12))).toBe(4);
    expect(map.toRawPos(map.toChipPos(13))).toBe(13);
    expect(map.toRawPos(map.toChipPos(16))).toBe(16);
    expect(map.toRawPos(map.toChipPos(24))).toBe(16);
    expect(map.toRawPos(map.toChipPos(25))).toBe(25);
  });

  it("maps multi-chip boundaries exactly", () => {
    const source = 'a + prop("X") + prop("Y")';
    const chipSpans: ChipSpan[] = [
      { start: 4, end: 13 },
      { start: 16, end: 25 },
    ];
    const map = buildChipOffsetMap(source.length, chipSpans);

    expect(map.toRawPos(4)).toBe(4);
    expect(map.toRawPos(5)).toBe(13);
    expect(map.toRawPos(8)).toBe(16);
    expect(map.toRawPos(9)).toBe(25);
  });
});

describe("buildChipOffsetMap validation", () => {
  it("throws on overlapping spans", () => {
    const chipSpans: ChipSpan[] = [
      { start: 0, end: 5 },
      { start: 4, end: 6 },
    ];
    expect(() => buildChipOffsetMap(10, chipSpans)).toThrow();
  });

  it("throws on out of bounds spans", () => {
    const chipSpans: ChipSpan[] = [{ start: 0, end: 100 }];
    expect(() => buildChipOffsetMap(10, chipSpans)).toThrow();
  });

  it("throws on unsorted spans", () => {
    const chipSpans: ChipSpan[] = [
      { start: 10, end: 12 },
      { start: 0, end: 2 },
    ];
    expect(() => buildChipOffsetMap(20, chipSpans)).toThrow();
  });
});

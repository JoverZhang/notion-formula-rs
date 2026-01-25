import { describe, expect, it } from "vitest";
import {
  computePropChips,
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  type Token,
} from "../../src/editor_decorations";

describe("computePropChips", () => {
  it('detects prop("Title")', () => {
    const source = 'prop("Title")';
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { range: { start: 0, end: 4 } } },
      { kind: "OpenParen", text: "(", span: { range: { start: 4, end: 5 } } },
      { kind: "String", text: '"Title"', span: { range: { start: 5, end: 12 } } },
      { kind: "CloseParen", text: ")", span: { range: { start: 12, end: 13 } } },
      { kind: "Eof", text: "", span: { range: { start: 13, end: 13 } } },
    ];

    const chips = computePropChips(source, tokens);
    expect(chips).toHaveLength(1);
    expect(chips[0]).toMatchObject({
      spanStart: 0,
      spanEnd: 13,
      argValue: "Title",
    });
  });
});

describe("computeTokenDecorationRanges", () => {
  it("covers all non-trivia tokens and skips Eof", () => {
    const source = 'prop("Title") + 1 +';
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { range: { start: 0, end: 4 } } },
      { kind: "OpenParen", text: "(", span: { range: { start: 4, end: 5 } } },
      { kind: "String", text: '"Title"', span: { range: { start: 5, end: 12 } } },
      { kind: "CloseParen", text: ")", span: { range: { start: 12, end: 13 } } },
      { kind: "Plus", text: "+", span: { range: { start: 14, end: 15 } } },
      { kind: "Number", text: "1", span: { range: { start: 16, end: 17 } } },
      { kind: "Plus", text: "+", span: { range: { start: 18, end: 19 } } },
      { kind: "Eof", text: "", span: { range: { start: 19, end: 19 } } },
    ];

    const ranges = computeTokenDecorationRanges(source.length, tokens);
    const classNames = ranges.map((range) => range.className);

    expect(classNames).toEqual([
      "tok tok-Ident",
      "tok tok-OpenParen",
      "tok tok-String",
      "tok tok-CloseParen",
      "tok tok-Plus",
      "tok tok-Number",
      "tok tok-Plus",
    ]);
    expect(ranges.every((range) => range.to > range.from)).toBe(true);
  });
});

describe("getTokenSpanIssues", () => {
  it("flags out-of-bounds spans without overlap", () => {
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { range: { start: 0, end: 4 } } },
    ];

    expect(getTokenSpanIssues(3, tokens)).toEqual({ outOfBounds: true, overlap: false });
  });

  it("flags overlapping spans", () => {
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { range: { start: 0, end: 4 } } },
      { kind: "Plus", text: "+", span: { range: { start: 3, end: 5 } } },
    ];

    expect(getTokenSpanIssues(10, tokens)).toEqual({ outOfBounds: false, overlap: true });
  });

  it("reports clean spans", () => {
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { range: { start: 0, end: 4 } } },
      { kind: "Plus", text: "+", span: { range: { start: 4, end: 5 } } },
    ];

    expect(getTokenSpanIssues(10, tokens)).toEqual({ outOfBounds: false, overlap: false });
  });
});

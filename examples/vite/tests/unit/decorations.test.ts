import { describe, expect, it } from "vitest";
import {
  computePropChips,
  computeTokenDecorationRanges,
  getTokenSpanIssues,
  type Token,
} from "../../src/editor_decorations";

describe("computePropChips", () => {
  function propertyTokens(source: string, raw: string): Token[] {
    return [
      { kind: "Ident", text: "prop", span: { start: 0, end: 4 } },
      { kind: "OpenParen", text: "(", span: { start: 4, end: 5 } },
      { kind: "String", text: raw, span: { start: 5, end: 5 + raw.length } },
      {
        kind: "CloseParen",
        text: ")",
        span: { start: source.length - 1, end: source.length },
      },
    ];
  }

  it('detects prop("Title")', () => {
    const source = 'prop("Title")';
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { start: 0, end: 4 } },
      { kind: "OpenParen", text: "(", span: { start: 4, end: 5 } },
      { kind: "String", text: '"Title"', span: { start: 5, end: 12 } },
      { kind: "CloseParen", text: ")", span: { start: 12, end: 13 } },
      { kind: "Eof", text: "", span: { start: 13, end: 13 } },
    ];

    const chips = computePropChips(source, tokens);
    expect(chips).toHaveLength(1);
    expect(chips[0]).toMatchObject({
      spanStart: 0,
      spanEnd: 13,
      argValue: "Title",
    });
  });

  it("looks up quote and backslash field names by decoding raw token text", () => {
    const source = String.raw`prop("quote\"slash\\")`;
    const raw = String.raw`"quote\"slash\\"`;
    const fieldName = 'quote"slash\\';
    const fields = new Map([[fieldName, "field-id"]]);
    const chips = computePropChips(source, propertyTokens(source, raw));
    expect(chips).toHaveLength(1);
    expect(fields.get(chips[0].argValue)).toBe("field-id");
    expect(chips[0]).toMatchObject({
      spanStart: 0,
      spanEnd: source.length,
      argContentStart: 6,
      argContentEnd: source.length - 2,
    });
  });

  it("keeps valid empty names and rejects invalid escapes and incomplete calls", () => {
    const emptySource = 'prop("")';
    expect(computePropChips(emptySource, propertyTokens(emptySource, '""'))[0].argValue).toBe("");
    const invalidSource = String.raw`prop("bad\q")`;
    expect(
      computePropChips(invalidSource, propertyTokens(invalidSource, String.raw`"bad\q"`)),
    ).toEqual([]);
    const tokens = propertyTokens('prop("Title")', '"Title"');
    expect(computePropChips('prop("Title"', tokens.slice(0, 3))).toEqual([]);
  });

  it("excludes member prop calls while retaining complete calls in incomplete formulas", () => {
    const directSource = 'prop("Title")';
    const tokens = propertyTokens(directSource, '"Title"');
    expect(computePropChips(`${directSource} +`, tokens)).toHaveLength(1);
    const memberTokens: Token[] = [
      { kind: "Ident", text: "value", span: { start: 0, end: 5 } },
      { kind: "Dot", text: ".", span: { start: 5, end: 6 } },
      ...tokens.map((token) => ({
        ...token,
        span: { start: token.span.start + 6, end: token.span.end + 6 },
      })),
    ];
    expect(computePropChips(`value.${directSource}`, memberTokens)).toEqual([]);
  });

  it("requires a String token before decoding the argument text", () => {
    const source = "prop(1)";
    const tokens = propertyTokens(source, "1");
    tokens[2].kind = "Number";
    expect(computePropChips(source, tokens)).toEqual([]);
  });
});

describe("computeTokenDecorationRanges", () => {
  it("covers all non-trivia tokens and skips Eof", () => {
    const source = 'prop("Title") + 1 +';
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { start: 0, end: 4 } },
      { kind: "OpenParen", text: "(", span: { start: 4, end: 5 } },
      { kind: "String", text: '"Title"', span: { start: 5, end: 12 } },
      { kind: "CloseParen", text: ")", span: { start: 12, end: 13 } },
      { kind: "Plus", text: "+", span: { start: 14, end: 15 } },
      { kind: "Number", text: "1", span: { start: 16, end: 17 } },
      { kind: "Plus", text: "+", span: { start: 18, end: 19 } },
      { kind: "Eof", text: "", span: { start: 19, end: 19 } },
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
    const tokens: Token[] = [{ kind: "Ident", text: "prop", span: { start: 0, end: 4 } }];

    expect(getTokenSpanIssues(3, tokens)).toEqual({ outOfBounds: true, overlap: false });
  });

  it("flags overlapping spans", () => {
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { start: 0, end: 4 } },
      { kind: "Plus", text: "+", span: { start: 3, end: 5 } },
    ];

    expect(getTokenSpanIssues(10, tokens)).toEqual({ outOfBounds: false, overlap: true });
  });

  it("reports clean spans", () => {
    const tokens: Token[] = [
      { kind: "Ident", text: "prop", span: { start: 0, end: 4 } },
      { kind: "Plus", text: "+", span: { start: 4, end: 5 } },
    ];

    expect(getTokenSpanIssues(10, tokens)).toEqual({ outOfBounds: false, overlap: false });
  });
});

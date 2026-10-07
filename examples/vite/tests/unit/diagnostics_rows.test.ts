import type { ExpressionDiagnostic } from "@notion-formula/sdk";
import { describe, expect, it } from "vitest";
import { buildDiagnosticTextRows } from "../../src/model/diagnostics";

function diag(overrides: Partial<ExpressionDiagnostic>): ExpressionDiagnostic {
  return {
    id: "opaque-diagnostic-id",
    message: "msg",
    span: { start: 0, end: 1 },
    ...overrides,
  };
}

describe("buildDiagnosticTextRows", () => {
  it("includes 1-based line/col", () => {
    const rows = buildDiagnosticTextRows(
      "1 +\n2 *",
      [diag({ message: "expected expression", span: { start: 6, end: 7 } })],
      null,
      [],
    );

    expect(rows).toEqual(["error 2:3: expected expression"]);
  });

  it("keeps chip position suffix with line/col prefix", () => {
    const rows = buildDiagnosticTextRows(
      "abx",
      [diag({ message: "expected expression", span: { start: 2, end: 3 } })],
      {
        toChipPos: (rawPos: number) => rawPos,
        toRawPos: (chipPos: number) => chipPos,
      },
      [],
    );

    expect(rows).toEqual(["error 1:3: expected expression chipPos=[2,3)"]);
  });

  it("computes columns from UTF-16 source offsets", () => {
    expect(
      buildDiagnosticTextRows("😀\nx", [diag({ span: { start: 3, end: 4 } })], null, []),
    ).toEqual(["error 2:1: msg"]);
  });
});

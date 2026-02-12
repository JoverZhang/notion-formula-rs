import { describe, expect, it } from "vitest";
import type { Diagnostic } from "../../src/analyzer/generated/wasm_dto";
import { buildDiagnosticTextRows } from "../../src/model/diagnostics";

function diag(overrides: Partial<Diagnostic>): Diagnostic {
  return {
    kind: "error",
    message: "msg",
    span: { start: 0, end: 1 },
    line: 1,
    col: 1,
    actions: [],
    ...overrides,
  };
}

describe("buildDiagnosticTextRows", () => {
  it("includes 1-based line/col", () => {
    const rows = buildDiagnosticTextRows(
      "1 +\n2 *",
      [diag({ message: "expected expression", line: 2, col: 3 })],
      null,
      [],
    );

    expect(rows).toEqual(["error 2:3: expected expression"]);
  });

  it("keeps chip position suffix with line/col prefix", () => {
    const rows = buildDiagnosticTextRows(
      "x",
      [diag({ message: "expected expression", line: 4, col: 9, span: { start: 2, end: 3 } })],
      {
        toChipPos: (rawPos: number) => rawPos,
        toRawPos: (chipPos: number) => chipPos,
      },
      [],
    );

    expect(rows).toEqual(["error 4:9: expected expression chipPos=[2,3)"]);
  });
});

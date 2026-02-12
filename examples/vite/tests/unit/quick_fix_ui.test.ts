import { describe, expect, it } from "vitest";
import type { Diagnostic } from "../../src/analyzer/generated/wasm_dto";
import { firstDiagnosticAction } from "../../src/ui/formula_panel_view";

function diag(actions: Diagnostic["actions"]): Diagnostic {
  return {
    kind: "error",
    message: "msg",
    span: { start: 0, end: 1 },
    line: 1,
    col: 1,
    actions,
  };
}

describe("firstDiagnosticAction", () => {
  it("selects only the first available action", () => {
    const diagnostics: Diagnostic[] = [
      diag([
        { title: "Insert `)`", edits: [{ range: { start: 4, end: 4 }, new_text: ")" }] },
        { title: "Insert `,`", edits: [{ range: { start: 2, end: 2 }, new_text: "," }] },
      ]),
    ];

    const action = firstDiagnosticAction(diagnostics);
    expect(action).toEqual({
      title: "Insert `)`",
      edits: [{ range: { start: 4, end: 4 }, new_text: ")" }],
    });
  });

  it("filters invalid edits in first action", () => {
    const diagnostics: Diagnostic[] = [
      diag([
        {
          title: "bad",
          edits: [
            { range: { start: -1, end: 1 }, new_text: "x" },
            { range: { start: 5, end: 4 }, new_text: "x" },
          ],
        },
      ]),
    ];

    const action = firstDiagnosticAction(diagnostics);
    expect(action).toBeNull();
  });
});

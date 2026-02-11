import { describe, expect, it } from "vitest";
import type { QuickFixView } from "../../src/analyzer/generated/wasm_dto";
import { firstQuickFixChanges } from "../../src/ui/formula_panel_view";

function fix(title: string, edits: QuickFixView["edits"]): QuickFixView {
  return { title, edits };
}

describe("firstQuickFixChanges", () => {
  it("applies only the first quick fix per click", () => {
    const fixes: QuickFixView[] = [
      fix("Insert `)`", [{ range: { start: 4, end: 4 }, new_text: ")" }]),
      fix("Insert `,`", [{ range: { start: 2, end: 2 }, new_text: "," }]),
    ];

    const changes = firstQuickFixChanges(fixes, 10);
    expect(changes).toEqual([{ from: 4, to: 4, insert: ")" }]);
  });

  it("drops invalid edits in the first quick fix", () => {
    const fixes: QuickFixView[] = [
      fix("bad", [
        { range: { start: -1, end: 1 }, new_text: "x" },
        { range: { start: 5, end: 4 }, new_text: "x" },
      ]),
      fix("Insert `)`", [{ range: { start: 3, end: 3 }, new_text: ")" }]),
    ];

    const changes = firstQuickFixChanges(fixes, 4);
    expect(changes).toEqual([]);
  });
});

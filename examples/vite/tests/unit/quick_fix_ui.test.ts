// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { QuickFix } from "../../src/formula/client";
import { mountFormulaPanel } from "../helpers/formula_panel";

const mounted: ReturnType<typeof mountFormulaPanel>[] = [];
function mount() {
  const view = mountFormulaPanel("if(");
  mounted.push(view);
  return view;
}
beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  for (const view of mounted.splice(0)) view.panel.dispose();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const fix: QuickFix = {
  title: "Insert `)`",
  edit: { base_version: 0n, edits: [{ range: { start: 3, end: 3 }, new_text: ")" }] },
};
const diagnostic = (id: string) => ({ id, span: { start: 3, end: 3 }, message: "Expected `)`" });

describe("native quick fixes", () => {
  it("queries opaque diagnostic IDs and uses the first available native action", async () => {
    const view = mount();
    view.actions.quickFixes.mockResolvedValueOnce([]).mockResolvedValueOnce([fix]);
    view.actions.applyEdit.mockImplementation(() => Promise.resolve(view.publishNative("if()", 3)));
    view.update({ diagnostics: [diagnostic("opaque:first"), diagnostic("opaque:second")] });
    await vi.advanceTimersByTimeAsync(1);
    expect(view.actions.quickFixes.mock.calls).toEqual([
      ["f1", "opaque:first"],
      ["f1", "opaque:second"],
    ]);
    const button = view.panel.root.querySelector<HTMLButtonElement>(
      '[data-testid="quick-fix-button"]',
    )!;
    expect(button.disabled).toBe(false);
    expect(button.title).toBe(fix.title);
    button.click();
    await Promise.resolve();
    expect(view.actions.applyEdit).toHaveBeenCalledWith("f1", fix.edit, 0);
    expect(view.actions.applyEdit.mock.calls[0][1]).toBe(fix.edit);
    expect(view.editor.state.doc.toString()).toBe("if()");
    expect(view.actions.setSource).not.toHaveBeenCalled();
  });

  it("ignores an old diagnostic's delayed fixes", async () => {
    const view = mount();
    let finish!: (fixes: QuickFix[]) => void;
    view.actions.quickFixes.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.update({ diagnostics: [diagnostic("opaque:old")] });
    view.update({ diagnostics: [diagnostic("opaque:new")], version: 1n });
    await vi.advanceTimersByTimeAsync(1);
    finish([fix]);
    await Promise.resolve();
    const button = view.panel.root.querySelector<HTMLButtonElement>(
      '[data-testid="quick-fix-button"]',
    )!;
    expect(button.disabled).toBe(true);
  });
});

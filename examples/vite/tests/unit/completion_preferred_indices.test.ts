// @vitest-environment jsdom
import type { CursorHelp, UpdateExpressionResult } from "@notion-formula/sdk";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { completionItem, cursorHelp, mountFormulaPanel } from "../helpers/formula_panel";

type MountedPanel = ReturnType<typeof mountFormulaPanel>;
const mounted: MountedPanel[] = [];
function mount(source = "i") {
  const view = mountFormulaPanel(source);
  mounted.push(view);
  return view;
}

beforeEach(() => {
  vi.useFakeTimers();
  Object.defineProperty(Range.prototype, "getClientRects", { configurable: true, value: () => [] });
  Object.defineProperty(Range.prototype, "getBoundingClientRect", {
    configurable: true,
    value: () => new DOMRect(),
  });
});
afterEach(() => {
  for (const view of mounted.splice(0)) view.panel.dispose();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

const items = [
  completionItem({ label: "textFn", kind: "FunctionText" }),
  completionItem({ label: "generalFn", kind: "FunctionGeneral" }),
];
async function showCompletions(view: MountedPanel, preferred: number[] = []) {
  view.actions.help.mockResolvedValue(
    cursorHelp({
      completion: { items, replace: { start: 0, end: 1 }, preferred_indices: preferred },
    }),
  );
  view.editor.focus();
  view.editor.dispatch({ selection: { anchor: view.editor.state.doc.length } });
  await vi.advanceTimersByTimeAsync(200);
}

const itemLabels = (view: MountedPanel) =>
  Array.from(
    view.panel.root.querySelectorAll(".completion-item-label"),
    (node) => node.textContent,
  );

describe("recommended completions", () => {
  it("does not show Recommended when preferred_indices is empty", async () => {
    const view = mount();
    await showCompletions(view);
    expect(itemLabels(view)).toEqual(["textFn", "generalFn"]);
    expect(view.panel.root.querySelector(".completion-recommended-header")).toBeNull();
  });

  it("shows Recommended when preferred_indices is non-empty and marks items", async () => {
    const view = mount();
    await showCompletions(view, [0]);
    expect(view.panel.root.querySelector(".completion-recommended-header")).toBeTruthy();
    const first = view.panel.root.querySelector(".completion-item");
    expect(first?.getAttribute("data-completion-index")).toBe("0");
    expect(first?.getAttribute("data-completion-recommended")).toBe("true");
    expect(view.panel.root.querySelectorAll('[data-completion-index="0"]')).toHaveLength(1);
  });

  it("renders preferred_indices once each in their requested order", async () => {
    const view = mount();
    await showCompletions(view, [1, 0, 1]);
    expect(itemLabels(view)).toEqual(["generalFn", "textFn"]);
    expect(view.panel.root.querySelectorAll('[data-completion-recommended="true"]')).toHaveLength(
      2,
    );
  });
});

describe("asynchronous editor actions", () => {
  it.each(["Tab", "Enter"])(
    "applies %s completions through versioned native edits",
    async (key) => {
      const view = mount();
      const item = completionItem();
      view.actions.help.mockResolvedValue(
        cursorHelp({
          base_version: 7n,
          completion: { items: [item], replace: { start: 0, end: 1 }, preferred_indices: [] },
        }),
      );
      view.actions.applyEdit.mockImplementation(() =>
        Promise.resolve(view.publishNative("if()", 4)),
      );
      view.editor.focus();
      view.editor.dispatch({ selection: { anchor: 1 } });
      await vi.advanceTimersByTimeAsync(200);
      view.editor.contentDOM.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
      await Promise.resolve();
      expect(view.actions.applyEdit).toHaveBeenCalledWith(
        "Formula 1",
        {
          base_version: 7n,
          edits: [item.primary_edit],
        },
        1,
      );
      expect(view.actions.applyEdit.mock.calls[0][1].edits[0]).toBe(item.primary_edit);
      expect(view.editor.state.doc.toString()).toBe("if()");
      expect(view.editor.state.selection.main.head).toBe(3);
      expect(view.actions.setSource).not.toHaveBeenCalled();
    },
  );

  it("uses the native cursor when completion has no cursor hint", async () => {
    const view = mount();
    view.actions.help.mockResolvedValue(
      cursorHelp({
        completion: {
          items: [completionItem({ cursor: null })],
          replace: { start: 0, end: 1 },
          preferred_indices: [],
        },
      }),
    );
    view.actions.applyEdit.mockImplementation(() => Promise.resolve(view.publishNative("if()", 2)));
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    view.panel.root.querySelector<HTMLElement>(".completion-item")!.click();
    await Promise.resolve();
    expect(view.editor.state.selection.main.head).toBe(2);
  });

  it("ignores help that resolves after text changed", async () => {
    const view = mount();
    let finish!: (help: CursorHelp) => void;
    view.actions.help.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    view.editor.dispatch({ changes: { from: 0, to: 1, insert: "new" }, selection: { anchor: 3 } });
    view.actions.help.mockResolvedValue(
      cursorHelp({
        completion: {
          items: [completionItem({ label: "fresh" })],
          replace: { start: 0, end: 3 },
          preferred_indices: [],
        },
      }),
    );
    await vi.advanceTimersByTimeAsync(200);
    finish(
      cursorHelp({
        completion: {
          items: [completionItem({ label: "stale" })],
          replace: { start: 0, end: 1 },
          preferred_indices: [],
        },
      }),
    );
    await Promise.resolve();
    expect(itemLabels(view)).toEqual(["fresh"]);
  });

  it("ignores help that resolves after a cursor move", async () => {
    const view = mount("ab");
    let finish!: (help: CursorHelp) => void;
    view.actions.help.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    view.editor.dispatch({ selection: { anchor: 1 } });
    finish(cursorHelp());
    await Promise.resolve();
    expect(itemLabels(view)).toEqual([]);
  });

  it("does not reopen a blurred panel when help resolves", async () => {
    const view = mount();
    let finish!: (help: CursorHelp) => void;
    view.actions.help.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    view.editor.contentDOM.blur();
    await vi.advanceTimersByTimeAsync(30);
    finish(cursorHelp());
    await Promise.resolve();
    expect(view.panel.root.querySelector(".completion-panel")?.classList.contains("hidden")).toBe(
      true,
    );
    expect(itemLabels(view)).toEqual([]);
  });

  it("does not move the cursor for a delayed edit after further typing", async () => {
    const view = mount();
    let finish!: (result: UpdateExpressionResult) => void;
    view.actions.applyEdit.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    view.panel.root.querySelector<HTMLElement>(".completion-item")!.click();
    view.editor.dispatch({ changes: { from: 1, insert: "x" }, selection: { anchor: 2 } });
    finish({
      state: {
        version: 1n,
        definition: { id: "Formula 1", expression: "if()" },
        output_type: "Unknown",
        diagnostics: [],
        tokens: [],
        property_references: [],
      },
      cursor: 3,
    });
    await Promise.resolve();
    expect(view.editor.state.doc.toString()).toBe("ix");
    expect(view.editor.state.selection.main.head).toBe(2);
  });

  it("syncs native/discard text without creating another Replace operation", () => {
    const view = mount();
    view.update({ source: "saved", cursor: 4, dirty: false });
    expect(view.editor.state.doc.toString()).toBe("saved");
    expect(view.editor.state.selection.main.head).toBe(4);
    expect(view.actions.setSource).not.toHaveBeenCalled();
  });

  it("formats through actions and uses the native source and cursor", async () => {
    const view = mount("1+2");
    view.editor.dispatch({ selection: { anchor: 2 } });
    view.actions.format.mockImplementation(() => Promise.resolve(view.publishNative("1 + 2", 3)));
    view.panel.root.querySelector<HTMLButtonElement>('[data-testid="format-button"]')!.click();
    await Promise.resolve();
    expect(view.actions.format).toHaveBeenCalledWith("Formula 1", 2);
    expect(view.editor.state.doc.toString()).toBe("1 + 2");
    expect(view.editor.state.selection.main.head).toBe(3);
    expect(view.actions.setSource).not.toHaveBeenCalled();
  });

  it.each(["format", "completion"])(
    "keeps a newer user cursor when delayed %s publishes native state before resolving",
    async (command) => {
      const view = mount(command === "format" ? "1+2" : "i");
      let finish!: (result: UpdateExpressionResult) => void;
      const pending = new Promise<UpdateExpressionResult>((resolve) => {
        finish = resolve;
      });
      if (command === "format") {
        view.editor.dispatch({ selection: { anchor: 2 } });
        view.actions.format.mockReturnValue(pending);
        view.panel.root.querySelector<HTMLButtonElement>('[data-testid="format-button"]')!.click();
      } else {
        view.actions.applyEdit.mockReturnValue(pending);
        view.editor.focus();
        view.editor.dispatch({ selection: { anchor: 1 } });
        await vi.advanceTimersByTimeAsync(200);
        view.panel.root.querySelector<HTMLElement>(".completion-item")!.click();
      }
      view.editor.dispatch({ selection: { anchor: 0 } });
      const source = command === "format" ? "1 + 2" : "if()";
      // AppVM emits state.cursor synchronously before its actions Promise settles.
      const native = view.publishNative(source, 3);
      expect(view.editor.state.doc.toString()).toBe(source);
      expect(view.editor.state.selection.main.head).toBe(0);
      finish(native);
      await Promise.resolve();
      expect(view.editor.state.selection.main.head).toBe(0);
      expect(view.actions.setSource).not.toHaveBeenCalled();
    },
  );

  it("refreshes help when saving recreates a Draft at the same version", async () => {
    const view = mount();
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    expect(view.actions.help).toHaveBeenCalledTimes(1);
    view.update({}, true);
    expect(itemLabels(view)).toEqual([]);
    view.update({}, false);
    await vi.advanceTimersByTimeAsync(200);
    expect(view.actions.help).toHaveBeenCalledTimes(2);
    expect(itemLabels(view)).toEqual(["if()"]);
  });

  it("defers state rendering if setSource publishes synchronously", async () => {
    const view = mount();
    view.actions.setSource.mockImplementation((_id, source) =>
      view.update({ source, version: 1n }),
    );
    view.editor.dispatch({ changes: { from: 1, insert: "x" }, selection: { anchor: 2 } });
    await Promise.resolve();
    expect(view.editor.state.doc.toString()).toBe("ix");
    expect(view.actions.setSource).toHaveBeenCalledExactlyOnceWith("Formula 1", "ix");
  });

  it("disables Save and Discard during saving while typing still works", () => {
    const view = mount();
    const save = view.panel.root.querySelector<HTMLButtonElement>('[data-testid="save-button"]')!;
    const discard = view.panel.root.querySelector<HTMLButtonElement>(
      '[data-testid="discard-button"]',
    )!;
    expect(save.disabled).toBe(false);
    expect(discard.disabled).toBe(false);
    save.click();
    discard.click();
    expect(view.actions.save).toHaveBeenCalledWith("Formula 1");
    expect(view.actions.discard).toHaveBeenCalledWith("Formula 1");
    view.update({}, true);
    expect(save.disabled).toBe(true);
    expect(discard.disabled).toBe(true);
    view.editor.dispatch({ changes: { from: 1, insert: "x" } });
    expect(view.actions.setSource).toHaveBeenCalledWith("Formula 1", "ix");
  });

  it("shows draft status and action errors", () => {
    const view = mount();
    expect(view.panel.root.querySelector('[data-testid="formula-dirty"]')?.textContent).toBe(
      "Unsaved changes",
    );
    view.update({ error: "Save failed. Try again." });
    const error = view.panel.root.querySelector('[data-testid="formula-error"]');
    expect(error?.textContent).toBe("Save failed. Try again.");
    expect(error?.classList.contains("hidden")).toBe(false);
  });

  it("releases debug handles and ignores pending help after disposal", async () => {
    const view = mount();
    let finish!: (help: CursorHelp) => void;
    view.actions.help.mockReturnValueOnce(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    view.editor.focus();
    await vi.advanceTimersByTimeAsync(200);
    expect(window.__nf_debug?.listPanels()).toContain("Formula 1");
    view.panel.dispose();
    finish(cursorHelp());
    await Promise.resolve();
    expect(window.__nf_debug?.listPanels() ?? []).not.toContain("Formula 1");
    expect(itemLabels(view)).toEqual([]);
  });
});
